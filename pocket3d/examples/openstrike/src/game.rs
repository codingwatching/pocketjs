//! The windowed OpenStrike app (DESIGN.md §12–§21).
//!
//! This wires the authoritative [`crate::sim::Sim`] to the wgpu backend: it
//! uploads the BSP world + bot/weapon models through the [`RenderDevice`]
//! contract, translates window input into [`Control`], advances the sim on the
//! fixed tick, drives per-bot animation state machines, and builds a
//! [`SceneView`] each frame. It is display-gated (needs a GPU + window), so it
//! is compiled behind the `window` feature and cannot run in headless CI.

use anyhow::{Context, Result};
use std::path::Path;

use pocket3d_anim::{compute_joint_matrices, AnimStateMachine, AnimationClip};
use pocket3d_app::{
    AppInitContext, FixedUpdateContext, FrameUpdateContext, Pocket3dApp, RenderContext,
};
use pocket3d_assets::{import_glb, import_static_glb, procedural_bot, world_upload, ImportedModel};
use pocket3d_audio::{AudioBackend, NullAudio, SoundBank};
use pocket3d_bsp::{load_bsp, BspWorldAsset};
use pocket3d_core::glam::{Mat4, Quat, Vec2, Vec3};
use pocket3d_core::mesh::{MeshData, StaticVertex};
use pocket3d_core::{
    Button, Capsule, Camera, Key, MaterialHandle, MeshHandle, SkinnedMeshHandle, SoundHandle,
    TextureData, WorldHandle,
};
use pocket3d_render::{
    debug, Hud, MaterialDesc, MaterialKind, SceneView, SkinnedInstance, ViewmodelInstance,
};
use pocket3d_render_wgpu::{run_app, RunConfig};
use pocket3d_script::{BotConfig, RoundConfig, WeaponConfig};

use crate::sim::{Control, RoundState, Sim};

const MOUSE_SENS: f32 = 0.0022;

/// Load the map + models and open the window.
pub fn run(
    map: &Path,
    wads: &Path,
    weapon: WeaponConfig,
    round: RoundConfig,
    bot: BotConfig,
    bots: usize,
) -> Result<()> {
    let bsp = load_bsp(map, &[wads.to_path_buf()]).context("loading map")?;
    let sim = Sim::new(&bsp, weapon, round, bot, bots.max(1));

    // Load the committed project-owned models; fall back to the in-memory
    // procedural bot if the .glb is missing (DESIGN.md §11).
    let models = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
    let bot_model = import_glb(models.join("bot.glb")).unwrap_or_else(|_| procedural_bot());
    let weapon_cpu = import_static_glb(models.join("weapon.glb")).ok();

    let game = OpenStrikeGame::new(bsp, sim, bot_model, weapon_cpu);
    run_app(game, RunConfig { title: "OpenStrike".into(), width: 1280, height: 720 })
}

struct OpenStrikeGame {
    bsp: BspWorldAsset,
    sim: Sim,
    bot_model: ImportedModel,
    weapon_cpu: Option<MeshData<StaticVertex>>,
    /// One animation state machine per bot (independent playback).
    bot_anims: Vec<AnimStateMachine>,

    // GPU resource handles, filled in `init`.
    world: WorldHandle,
    bot_mesh: SkinnedMeshHandle,
    bot_material: MaterialHandle,
    weapon_mesh: Option<MeshHandle>,
    weapon_material: MaterialHandle,

    // Camera look accumulated from the mouse.
    yaw: f32,
    pitch: f32,
    show_debug: bool,

    // Minimal audio (fire-and-forget), driven from sim events.
    audio: NullAudio,
    snd_shot: SoundHandle,
    snd_death: SoundHandle,
    event_cursor: usize,
}

impl OpenStrikeGame {
    fn new(
        bsp: BspWorldAsset,
        sim: Sim,
        bot_model: ImportedModel,
        weapon_cpu: Option<MeshData<StaticVertex>>,
    ) -> Self {
        let yaw = sim.player.yaw;
        let pitch = sim.player.pitch;
        let bot_anims = (0..sim.bots.len())
            .map(|_| make_state_machine(&bot_model.clips))
            .collect();
        // Register logical sounds; the bank is only needed to mint the stable
        // handles (playback is a no-op under NullAudio).
        let mut sounds = SoundBank::new();
        let snd_shot = sounds.register("gunshot");
        let snd_death = sounds.register("bot_death");
        Self {
            bsp,
            sim,
            bot_model,
            weapon_cpu,
            bot_anims,
            world: WorldHandle::INVALID,
            bot_mesh: SkinnedMeshHandle::INVALID,
            bot_material: MaterialHandle::INVALID,
            weapon_mesh: None,
            weapon_material: MaterialHandle::INVALID,
            yaw,
            pitch,
            show_debug: false,
            audio: NullAudio,
            snd_shot,
            snd_death,
            event_cursor: 0,
        }
    }

    fn player_capsule(&self) -> Capsule {
        let b = &self.sim.player.body;
        Capsule::from_base_height(b.position, b.height, b.radius)
    }
}

/// Build a bot state machine from its imported clips (found by name).
fn make_state_machine(clips: &[AnimationClip]) -> AnimStateMachine {
    let find = |name: &str| -> AnimationClip {
        clips
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
            .or_else(|| clips.first())
            .cloned()
            .unwrap_or_else(|| AnimationClip {
                name: name.to_string(),
                duration: 1.0,
                channels: Vec::new(),
            })
    };
    AnimStateMachine::new(find("Idle"), find("Walk"), find("Death"))
}

impl Pocket3dApp for OpenStrikeGame {
    fn init(&mut self, ctx: &mut AppInitContext<'_>) -> Result<()> {
        // Upload the compiled BSP world (geometry + lightmap + textures).
        self.world = ctx.device.upload_world(&world_upload(&self.bsp));

        // Bot skinned mesh + material.
        self.bot_mesh = ctx.device.upload_skinned_mesh(&self.bot_model.skinned_mesh);
        let bot_tex = self
            .bot_model
            .base_texture
            .clone()
            .unwrap_or_else(|| TextureData::solid(190, 170, 150, 255));
        let bot_tex = ctx.device.upload_texture(&bot_tex);
        self.bot_material =
            ctx.device
                .create_material(&MaterialDesc::new("bot", MaterialKind::SkinnedLit), bot_tex);

        // Weapon viewmodel.
        if let Some(mesh) = &self.weapon_cpu {
            self.weapon_mesh = Some(ctx.device.upload_static_mesh(mesh));
            let wtex = ctx.device.upload_texture(&TextureData::solid(70, 70, 80, 255));
            self.weapon_material = ctx
                .device
                .create_material(&MaterialDesc::new("weapon", MaterialKind::Viewmodel), wtex);
        }
        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut FixedUpdateContext<'_>) {
        let input = ctx.input;
        let (mx, my) = input.move_axes();
        let control = Control {
            move_axes: Vec2::new(mx, my),
            jump: input.key_down(Key::Space),
            fire: input.button_down(Button::Left),
            yaw: self.yaw,
            pitch: self.pitch,
        };
        self.sim.tick(&control, ctx.tick.dt);

        // Drive bot animation state machines from the sim's per-bot states.
        for (i, bot) in self.sim.bots.iter().enumerate() {
            if let Some(sm) = self.bot_anims.get_mut(i) {
                sm.request(bot.anim);
                sm.update(ctx.tick.dt);
            }
        }

        // Fire-and-forget audio from new sim events (DESIGN.md §22).
        while self.event_cursor < self.sim.events.len() {
            match &self.sim.events[self.event_cursor] {
                pocket3d_core::Event::ShotFired { .. } => self.audio.play_2d(self.snd_shot, 1.0),
                pocket3d_core::Event::BotKilled { .. } => self.audio.play_2d(self.snd_death, 1.0),
                _ => {}
            }
            self.event_cursor += 1;
        }
    }

    fn update(&mut self, ctx: &mut FrameUpdateContext<'_>) {
        // Accumulate mouse look once per frame.
        let (dx, dy) = ctx.input.mouse_delta;
        self.yaw -= dx * MOUSE_SENS;
        self.pitch = (self.pitch - dy * MOUSE_SENS).clamp(-Camera::PITCH_LIMIT, Camera::PITCH_LIMIT);
        if ctx.input.key_pressed(Key::F1) {
            self.show_debug = !self.show_debug;
        }
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) {
        let scene: &mut SceneView = ctx.scene;
        scene.begin_frame();
        scene.camera = self.sim.player.camera();
        scene.world = Some(self.world);

        // Bots (skinned actors).
        for (i, bot) in self.sim.bots.iter().enumerate() {
            let Some(sm) = self.bot_anims.get(i) else { continue };
            let pose = sm.sample(&self.bot_model.skeleton);
            let joint_matrices = compute_joint_matrices(&self.bot_model.skeleton, &pose);
            let transform =
                Mat4::from_rotation_translation(Quat::from_rotation_z(bot.yaw), bot.body.position);
            scene.skinned.push(SkinnedInstance {
                mesh: self.bot_mesh,
                material: self.bot_material,
                transform,
                joint_matrices,
            });
        }

        // First-person weapon.
        if let Some(mesh) = self.weapon_mesh {
            let cam = scene.camera;
            let fwd = cam.forward();
            let right = cam.right();
            let pos = cam.eye + fwd * 18.0 + right * 7.0 - Vec3::Z * 7.0;
            let rot = Quat::from_rotation_z(cam.yaw) * Quat::from_rotation_y(-cam.pitch);
            scene.viewmodel = Some(ViewmodelInstance {
                mesh,
                material: self.weapon_material,
                transform: Mat4::from_rotation_translation(rot, pos),
                fov_y_deg: 60.0,
            });
        }

        // Debug overlays (DESIGN.md §23).
        if self.show_debug {
            scene.debug.capsule(&self.player_capsule(), debug::GREEN);
            for bot in &self.sim.bots {
                let cap =
                    Capsule::from_base_height(bot.body.position, bot.body.height, bot.body.radius);
                let color = if bot.alive { debug::RED } else { debug::YELLOW };
                scene.debug.capsule(&cap, color);
            }
            for wp in &self.sim.waypoints {
                scene.debug.cross(*wp, 20.0, debug::CYAN);
            }
            if let Some((a, b)) = self.sim.last_shot {
                scene.debug.line(a, b, debug::WHITE);
            }
        }

        // HUD.
        scene.hud = Hud {
            show_crosshair: true,
            health: self.sim.player.health,
            ammo: Some(self.sim.weapon.ammo),
            round_text: round_text(self.sim.state, self.sim.round_number),
            show_debug: self.show_debug,
            debug_lines: vec![
                format!("round {} ({:?})", self.sim.round_number, self.sim.state),
                format!("bots alive: {}", self.sim.alive_bots()),
            ],
        };
    }
}

fn round_text(state: RoundState, round: u32) -> Option<String> {
    match state {
        RoundState::PreRound => Some(format!("Round {round}")),
        RoundState::PlayerWon => Some("You win!".to_string()),
        RoundState::PlayerLost => Some("You died".to_string()),
        _ => None,
    }
}
