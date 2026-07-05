//! OpenStrike authoritative simulation (DESIGN.md §13–§19).
//!
//! This module is renderer- and asset-agnostic: it owns the player, bots,
//! weapon, and round state machine, and advances them on a fixed tick. Both the
//! windowed runtime and the headless `openstrike sim` runner drive it, which is
//! what makes the whole gameplay loop testable without a GPU (DESIGN.md §6:
//! "Headless-testable gameplay logic").

use glam::{Vec2, Vec3};
use pocket3d_anim::AnimState;
use pocket3d_bsp::{BspWorldAsset, Team};
use pocket3d_core::{Camera, Event, Ray};
use pocket3d_kcc::{CharacterBody, CharacterController, CharacterMoveResult, SlideController};
use pocket3d_physics::PhysicsWorld;
use pocket3d_script::{BotConfig, RoundConfig, WeaponConfig};

// --- Movement constants (DESIGN.md §15). Starting points, not CS-accurate. ---
const WALK_SPEED: f32 = 240.0;
const GRAVITY: f32 = 800.0;
const JUMP_SPEED: f32 = 270.0;
const GROUND_ACCEL: f32 = 12.0;
const AIR_ACCEL: f32 = 2.0;
const FRICTION: f32 = 8.0;
const STEP_HEIGHT: f32 = 18.0;
const CAPSULE_RADIUS: f32 = 16.0;
const CAPSULE_HEIGHT: f32 = 72.0;
const EYE_HEIGHT: f32 = 64.0;
const HEAD_HEIGHT: f32 = 60.0;
const HEAD_RADIUS: f32 = 10.0;
const SLOPE_LIMIT: f32 = 45.0;

/// Per-tick control input, produced by winit (windowed) or the autopilot
/// (headless). Look angles are absolute so the autopilot can aim precisely.
#[derive(Clone, Copy, Debug, Default)]
pub struct Control {
    /// Local move axes: x = right/left, y = forward/back (from WASD).
    pub move_axes: Vec2,
    pub jump: bool,
    pub fire: bool,
    pub yaw: f32,
    pub pitch: f32,
}

/// The player character.
pub struct Player {
    pub body: CharacterBody,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: i32,
    pub grounded: bool,
}

impl Player {
    pub fn eye(&self) -> Vec3 {
        self.body.position + Vec3::Z * EYE_HEIGHT
    }

    pub fn camera(&self) -> Camera {
        Camera {
            eye: self.eye(),
            yaw: self.yaw,
            pitch: self.pitch,
            fov_y_deg: 80.0,
            near: 0.03,
            far: 8192.0,
        }
    }
}

/// A simple waypoint-following bot (DESIGN.md §16).
pub struct Bot {
    pub body: CharacterBody,
    pub velocity: Vec3,
    pub yaw: f32,
    pub health: i32,
    pub alive: bool,
    pub anim: AnimState,
    /// Index into the shared waypoint loop.
    pub waypoint: usize,
    pub grounded: bool,
    /// Ticks since death (for the death animation / cleanup).
    pub death_timer: f32,
}

impl Bot {
    pub fn center(&self) -> Vec3 {
        self.body.position + Vec3::Z * (CAPSULE_HEIGHT * 0.5)
    }
    pub fn head(&self) -> Vec3 {
        self.body.position + Vec3::Z * HEAD_HEIGHT
    }
}

/// Weapon runtime state (DESIGN.md §18).
pub struct Weapon {
    pub config: WeaponConfig,
    pub cooldown: f32,
    pub ammo: i32,
    pub reloading: f32,
    pub muzzle_flash: f32,
}

impl Weapon {
    fn new(config: WeaponConfig) -> Self {
        let ammo = config.magazine_size as i32;
        Self {
            config,
            cooldown: 0.0,
            ammo,
            reloading: 0.0,
            muzzle_flash: 0.0,
        }
    }
}

/// The round state machine (DESIGN.md §19).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoundState {
    Loading,
    PreRound,
    Live,
    PlayerWon,
    PlayerLost,
    Intermission,
    Restarting,
}

/// The full simulation.
pub struct Sim {
    pub physics: PhysicsWorld,
    pub player: Player,
    pub bots: Vec<Bot>,
    pub weapon: Weapon,
    pub round: RoundConfig,
    pub bot_cfg: BotConfig,
    pub state: RoundState,
    pub state_timer: f32,
    pub round_number: u32,
    /// Shared bot patrol loop (from map spawns, DESIGN.md §16).
    pub waypoints: Vec<Vec3>,
    /// Spawn anchors captured at load for round resets.
    ct_spawn: (Vec3, f32),
    t_spawns: Vec<Vec3>,
    kcc: SlideController,
    /// Events emitted this session (drained by the host for audio/HUD/scripts).
    pub events: Vec<Event>,
    /// Last bullet ray, for the debug overlay.
    pub last_shot: Option<(Vec3, Vec3)>,
}

impl Sim {
    /// Build a simulation from a compiled BSP world + script-provided configs.
    pub fn new(
        bsp: &BspWorldAsset,
        weapon: WeaponConfig,
        round: RoundConfig,
        bot_cfg: BotConfig,
        bot_count: usize,
    ) -> Sim {
        let physics = PhysicsWorld::from_collision_mesh(&bsp.collision);

        let ct = bsp.pick_spawn(Team::Ct);

        let player = Player {
            body: CharacterBody {
                position: ct.pos,
                radius: CAPSULE_RADIUS,
                height: CAPSULE_HEIGHT,
                step_height: STEP_HEIGHT,
                slope_limit_deg: SLOPE_LIMIT,
            },
            velocity: Vec3::ZERO,
            yaw: ct.yaw_deg.to_radians(),
            pitch: 0.0,
            health: round.player_health,
            grounded: false,
        };

        let mut sim = Sim {
            physics,
            player,
            bots: Vec::new(),
            weapon: Weapon::new(weapon),
            round,
            bot_cfg,
            state: RoundState::Loading,
            state_timer: 0.0,
            round_number: 1,
            waypoints: Vec::new(),
            ct_spawn: (ct.pos, ct.yaw_deg.to_radians()),
            t_spawns: bsp.spawns_for(Team::T).map(|s| s.pos).collect(),
            kcc: SlideController::new(),
            events: Vec::new(),
            last_shot: None,
        };

        // Settle the player onto the floor so eye/LOS are consistent with the
        // real geometry, then anchor the reset spawn to the settled position.
        for _ in 0..90 {
            sim.settle_player(1.0 / 60.0);
        }
        sim.ct_spawn = (sim.player.body.position, sim.player.yaw);

        // Choose bot patrol spots that are on the ground AND in the player's
        // line of sight (verified by raycast), so the vertical slice reliably
        // demonstrates the full shoot->kill->win loop. Falls back to the map's
        // T spawns, then the map center.
        let spots = sim.find_open_spots(4);
        sim.waypoints = if !spots.is_empty() {
            spots
        } else if !sim.t_spawns.is_empty() {
            sim.t_spawns.clone()
        } else {
            vec![bsp.bounds.center()]
        };
        sim.t_spawns = sim.waypoints.clone();

        sim.spawn_bots(bot_count);
        sim.enter(RoundState::PreRound);
        sim
    }

    /// Find ground spots in front of the player with a clear line of sight,
    /// used as bot spawns/waypoints for the vertical slice.
    fn find_open_spots(&self, want: usize) -> Vec<Vec3> {
        let eye = self.player.eye();
        let feet = self.player.body.position;
        let yaw = self.player.yaw;
        let mut spots: Vec<Vec3> = Vec::new();
        for &dist in &[220.0f32, 300.0, 380.0, 460.0] {
            for k in -6..=6 {
                if spots.len() >= want {
                    return spots;
                }
                let a = yaw + k as f32 * 0.26;
                let (s, c) = a.sin_cos();
                let probe_top = feet + Vec3::new(c, s, 0.0) * dist + Vec3::Z * 96.0;
                // Drop to the floor.
                let Some(ground) = self.physics.raycast(&Ray::new(probe_top, -Vec3::Z), 500.0) else {
                    continue;
                };
                let spot = ground.point + Vec3::Z * 1.0;
                if ground.normal.z < 0.7 {
                    continue; // too steep to stand
                }
                // Require a clear line of sight from the eye to the bot center.
                let center = spot + Vec3::Z * (CAPSULE_HEIGHT * 0.5);
                let d = center - eye;
                let dlen = d.length();
                if dlen < 64.0 {
                    continue;
                }
                if self
                    .physics
                    .raycast(&Ray::new(eye, d / dlen), dlen - 40.0)
                    .is_some()
                {
                    continue; // blocked
                }
                if spots.iter().all(|p| (*p - spot).length() > 120.0) {
                    spots.push(spot);
                }
            }
        }
        spots
    }

    fn spawn_bots(&mut self, count: usize) {
        self.bots.clear();
        for i in 0..count {
            let spawn = self.t_spawns[i % self.t_spawns.len()];
            self.bots.push(Bot {
                body: CharacterBody {
                    position: spawn,
                    radius: CAPSULE_RADIUS,
                    height: CAPSULE_HEIGHT,
                    step_height: STEP_HEIGHT,
                    slope_limit_deg: SLOPE_LIMIT,
                },
                velocity: Vec3::ZERO,
                yaw: 0.0,
                health: self.round.bot_health,
                alive: true,
                anim: AnimState::Idle,
                waypoint: (i + 1) % self.waypoints.len().max(1),
                grounded: false,
                death_timer: 0.0,
            });
        }
    }

    fn enter(&mut self, state: RoundState) {
        self.state = state;
        self.state_timer = match state {
            RoundState::PreRound => self.round.pre_round_ms as f32 / 1000.0,
            RoundState::PlayerWon | RoundState::PlayerLost => self.round.intermission_ms as f32 / 1000.0,
            _ => 0.0,
        };
        match state {
            RoundState::Live => self.events.push(Event::RoundStart {
                round: self.round_number,
            }),
            RoundState::PlayerWon => self.events.push(Event::RoundEnd {
                round: self.round_number,
                player_won: true,
            }),
            RoundState::PlayerLost => self.events.push(Event::RoundEnd {
                round: self.round_number,
                player_won: false,
            }),
            _ => {}
        }
    }

    pub fn alive_bots(&self) -> usize {
        self.bots.iter().filter(|b| b.alive).count()
    }

    /// Advance the simulation one fixed tick.
    pub fn tick(&mut self, control: &Control, dt: f32) {
        // Look angles always track input so the view is responsive.
        self.player.yaw = control.yaw;
        self.player.pitch = control.pitch.clamp(-Camera::PITCH_LIMIT, Camera::PITCH_LIMIT);
        self.weapon.cooldown = (self.weapon.cooldown - dt).max(0.0);
        self.weapon.muzzle_flash = (self.weapon.muzzle_flash - dt).max(0.0);

        match self.state {
            RoundState::Loading => self.enter(RoundState::PreRound),
            RoundState::PreRound => {
                // Player can look/settle; movement disabled until Live.
                self.settle_player(dt);
                self.state_timer -= dt;
                if self.state_timer <= 0.0 {
                    self.enter(RoundState::Live);
                }
            }
            RoundState::Live => {
                self.update_player(control, dt);
                self.update_weapon(control, dt);
                self.update_bots(dt);
                if self.alive_bots() == 0 {
                    self.enter(RoundState::PlayerWon);
                } else if self.player.health <= 0 {
                    self.events.push(Event::PlayerKilled);
                    self.enter(RoundState::PlayerLost);
                }
            }
            RoundState::PlayerWon | RoundState::PlayerLost => {
                // Bots keep their death pose; player settles.
                self.settle_player(dt);
                for b in &mut self.bots {
                    b.death_timer += dt;
                }
                self.state_timer -= dt;
                if self.state_timer <= 0.0 {
                    self.enter(RoundState::Intermission);
                }
            }
            RoundState::Intermission => self.enter(RoundState::Restarting),
            RoundState::Restarting => {
                self.reset_round();
                self.round_number += 1;
                self.enter(RoundState::PreRound);
            }
        }
    }

    /// Reset transient round state without reloading the map (DESIGN.md §19).
    fn reset_round(&mut self) {
        self.player.body.position = self.ct_spawn.0;
        self.player.yaw = self.ct_spawn.1;
        self.player.pitch = 0.0;
        self.player.velocity = Vec3::ZERO;
        self.player.health = self.round.player_health;
        self.weapon.ammo = self.weapon.config.magazine_size as i32;
        self.weapon.cooldown = 0.0;
        self.weapon.reloading = 0.0;
        let count = self.bots.len().max(1);
        self.spawn_bots(count);
        self.last_shot = None;
    }

    /// Apply gravity/ground settling only (used outside Live).
    fn settle_player(&mut self, dt: f32) {
        self.player.velocity.z -= GRAVITY * dt;
        let delta = Vec3::new(0.0, 0.0, self.player.velocity.z * dt);
        let r = self.kcc.move_character(&self.physics, self.player.body, delta, dt);
        self.apply_player_result(r);
    }

    fn update_player(&mut self, control: &Control, dt: f32) {
        let (fwd, right) = yaw_basis(self.player.yaw);
        let wish = right * control.move_axes.x + fwd * control.move_axes.y;
        let wishdir = if wish.length_squared() > 1e-6 {
            wish.normalize()
        } else {
            Vec3::ZERO
        };

        let mut horiz = Vec3::new(self.player.velocity.x, self.player.velocity.y, 0.0);
        if self.player.grounded {
            horiz = apply_friction(horiz, FRICTION, dt);
            horiz = accelerate(horiz, wishdir, WALK_SPEED, GROUND_ACCEL, dt);
            if control.jump {
                self.player.velocity.z = JUMP_SPEED;
                self.player.grounded = false;
            }
        } else {
            horiz = accelerate(horiz, wishdir, WALK_SPEED, AIR_ACCEL, dt);
        }
        self.player.velocity.x = horiz.x;
        self.player.velocity.y = horiz.y;
        self.player.velocity.z -= GRAVITY * dt;

        let delta = self.player.velocity * dt;
        let r = self.kcc.move_character(&self.physics, self.player.body, delta, dt);
        self.apply_player_result(r);
    }

    fn apply_player_result(&mut self, r: CharacterMoveResult) {
        self.player.body.position = r.position;
        self.player.grounded = r.grounded;
        if r.grounded && self.player.velocity.z < 0.0 {
            self.player.velocity.z = 0.0;
        }
    }

    fn update_weapon(&mut self, control: &Control, dt: f32) {
        if self.weapon.reloading > 0.0 {
            self.weapon.reloading -= dt;
            if self.weapon.reloading <= 0.0 {
                self.weapon.ammo = self.weapon.config.magazine_size as i32;
            }
            return;
        }
        if control.fire && self.weapon.cooldown <= 0.0 && self.weapon.ammo != 0 {
            self.fire();
            self.weapon.cooldown = self.weapon.config.fire_interval_ms as f32 / 1000.0;
            self.weapon.muzzle_flash = 0.04;
            if self.weapon.ammo > 0 {
                self.weapon.ammo -= 1;
                if self.weapon.ammo == 0 {
                    self.weapon.reloading = self.weapon.config.reload_ms as f32 / 1000.0;
                }
            }
        }
    }

    /// Hitscan shot from the camera center (DESIGN.md §18).
    fn fire(&mut self) {
        let cam = self.player.camera();
        let origin = cam.eye;
        let dir = cam.forward();
        let range = self.weapon.config.range;
        self.last_shot = Some((origin, origin + dir * range));
        self.events.push(Event::ShotFired { origin, dir });

        let ray = Ray::new(origin, dir);
        // Nearest world hit bounds the shot.
        let world_t = self
            .physics
            .raycast(&ray, range)
            .map(|h| h.t)
            .unwrap_or(range);

        // Nearest bot hit within the world distance.
        let mut best: Option<(usize, f32, bool)> = None;
        for (i, bot) in self.bots.iter().enumerate() {
            if !bot.alive {
                continue;
            }
            let cap = bot.body;
            let a = bot.body.position + Vec3::Z * cap.radius;
            let b = bot.body.position + Vec3::Z * (cap.height - cap.radius);
            let head_hit = ray_sphere(&ray, bot.head(), HEAD_RADIUS);
            let body_hit = ray_capsule(&ray, a, b, cap.radius);
            let (t, headshot) = match (head_hit, body_hit) {
                (Some(ht), Some(bt)) => {
                    if ht <= bt {
                        (ht, true)
                    } else {
                        (bt, false)
                    }
                }
                (Some(ht), None) => (ht, true),
                (None, Some(bt)) => (bt, false),
                (None, None) => continue,
            };
            if t < world_t && best.map_or(true, |(_, bt, _)| t < bt) {
                best = Some((i, t, headshot));
            }
        }

        if let Some((i, t, headshot)) = best {
            let mut dmg = self.weapon.config.damage;
            if headshot {
                dmg *= self.weapon.config.headshot_multiplier;
            }
            let point = ray.at(t);
            let bot = &mut self.bots[i];
            bot.health -= dmg as i32;
            let entity = None; // sim bots aren't in the ECS World; None is fine.
            self.events.push(Event::Hit(pocket3d_core::HitEvent {
                kind: pocket3d_core::HitKind::Bot,
                point,
                normal: (origin - point).normalize_or_zero(),
                distance: t,
                entity,
                headshot,
                damage: dmg,
            }));
            if bot.health <= 0 && bot.alive {
                bot.alive = false;
                bot.anim = AnimState::Death;
                bot.velocity = Vec3::ZERO;
                self.events.push(Event::BotKilled { entity: default_entity() });
            }
        } else if world_t < range {
            let point = ray.at(world_t);
            self.events.push(Event::Hit(pocket3d_core::HitEvent {
                kind: pocket3d_core::HitKind::World,
                point,
                normal: Vec3::Z,
                distance: world_t,
                entity: None,
                headshot: false,
                damage: 0.0,
            }));
        }
    }

    fn update_bots(&mut self, dt: f32) {
        let player_pos = self.player.body.position;
        let speed = self.bot_cfg.move_speed;
        let wp_count = self.waypoints.len().max(1);
        for bot in &mut self.bots {
            if !bot.alive {
                bot.anim = AnimState::Death;
                bot.death_timer += dt;
                continue;
            }
            let target = self.waypoints[bot.waypoint % wp_count];
            let to = Vec2::new(target.x - bot.body.position.x, target.y - bot.body.position.y);
            if to.length() < 48.0 {
                bot.waypoint = (bot.waypoint + 1) % wp_count;
            }
            let wishdir = if to.length_squared() > 1e-4 {
                let n = to.normalize();
                Vec3::new(n.x, n.y, 0.0)
            } else {
                Vec3::ZERO
            };

            // Face the player if reasonably close (DESIGN.md §16 optional).
            let to_player = player_pos - bot.body.position;
            if to_player.length() < self.bot_cfg.aggro_range {
                bot.yaw = to_player.y.atan2(to_player.x);
            } else if wishdir.length_squared() > 1e-4 {
                bot.yaw = wishdir.y.atan2(wishdir.x);
            }

            let mut vel = wishdir * speed;
            bot.velocity.z -= GRAVITY * dt;
            vel.z = bot.velocity.z;
            let delta = vel * dt;
            let r = self
                .kcc
                .move_character(&self.physics, bot.body, delta, dt);
            let moved_h = Vec2::new(r.moved.x, r.moved.y).length();
            bot.body.position = r.position;
            bot.grounded = r.grounded;
            if r.grounded && bot.velocity.z < 0.0 {
                bot.velocity.z = 0.0;
            }
            bot.anim = if moved_h / dt > 20.0 {
                AnimState::Walk
            } else {
                AnimState::Idle
            };
        }
    }

    /// Aim at and fire on the nearest alive bot — used by the headless runner to
    /// exercise the whole shoot→kill→win loop without a human (DESIGN.md §6).
    pub fn autopilot_control(&self) -> Control {
        let eye = self.player.eye();
        let mut ctrl = Control {
            yaw: self.player.yaw,
            pitch: self.player.pitch,
            ..Default::default()
        };
        if let Some(bot) = self
            .bots
            .iter()
            .filter(|b| b.alive)
            .min_by(|a, b| {
                let da = (a.center() - eye).length();
                let db = (b.center() - eye).length();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let to = bot.center() - eye;
            ctrl.yaw = to.y.atan2(to.x);
            let horiz = (to.x * to.x + to.y * to.y).sqrt();
            ctrl.pitch = to.z.atan2(horiz);
            ctrl.fire = true;
        }
        ctrl
    }
}

fn default_entity() -> pocket3d_core::world::EntityId {
    // A placeholder key; sim bots are indexed directly, not stored in `World`.
    pocket3d_core::world::EntityId::default()
}

/// Horizontal forward/right basis from a yaw angle (Z-up).
fn yaw_basis(yaw: f32) -> (Vec3, Vec3) {
    let (s, c) = yaw.sin_cos();
    let fwd = Vec3::new(c, s, 0.0);
    let right = Vec3::new(s, -c, 0.0);
    (fwd, right)
}

/// Quake-style ground/air acceleration on the horizontal plane.
fn accelerate(vel: Vec3, wishdir: Vec3, wishspeed: f32, accel: f32, dt: f32) -> Vec3 {
    if wishdir.length_squared() < 1e-6 {
        return vel;
    }
    let current = vel.dot(wishdir);
    let addspeed = wishspeed - current;
    if addspeed <= 0.0 {
        return vel;
    }
    let accelspeed = (accel * wishspeed * dt).min(addspeed);
    vel + wishdir * accelspeed
}

fn apply_friction(vel: Vec3, friction: f32, dt: f32) -> Vec3 {
    let speed = vel.length();
    if speed < 1.0 {
        return Vec3::ZERO;
    }
    let drop = speed * friction * dt;
    let newspeed = (speed - drop).max(0.0) / speed;
    vel * newspeed
}

/// Nearest positive-t ray/sphere intersection.
fn ray_sphere(ray: &Ray, center: Vec3, radius: f32) -> Option<f32> {
    let oc = ray.origin - center;
    let b = oc.dot(ray.dir);
    let c = oc.dot(oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 {
        Some(t)
    } else {
        let t2 = -b + disc.sqrt();
        (t2 >= 0.0).then_some(t2)
    }
}

/// Ray vs capsule (segment `a`–`b`, `radius`). Returns the true **surface
/// entry** distance along the ray — the nearest of the infinite-cylinder near
/// root (clipped to the segment) and the two hemispherical end caps. Returning
/// a real surface `t` (not a closest-approach parameter) is what makes the
/// nearest-hit ranking against the world raycast, the head/body headshot
/// classification, and the reported impact point all correct.
fn ray_capsule(ray: &Ray, a: Vec3, b: Vec3, radius: f32) -> Option<f32> {
    let axis = b - a;
    let len2 = axis.dot(axis);
    let mut best = f32::INFINITY;

    // Infinite cylinder around the axis, near root, clipped to the segment.
    if len2 > 1e-8 {
        let w = ray.origin - a;
        let m = ray.dir - axis * (ray.dir.dot(axis) / len2); // perp. ray dir
        let n = w - axis * (w.dot(axis) / len2); // perp. offset
        let aa = m.dot(m);
        let bb = 2.0 * m.dot(n);
        let cc = n.dot(n) - radius * radius;
        if aa > 1e-8 {
            let disc = bb * bb - 4.0 * aa * cc;
            if disc >= 0.0 {
                let t = (-bb - disc.sqrt()) / (2.0 * aa);
                if t >= 0.0 {
                    let s = (ray.at(t) - a).dot(axis) / len2;
                    if (0.0..=1.0).contains(&s) {
                        best = best.min(t);
                    }
                }
            }
        }
    }

    // Hemispherical end caps: accept a sphere hit only when it lands on the
    // cap (beyond the corresponding segment end), so the capsule is exact.
    let denom = len2.max(1e-8);
    if let Some(t) = ray_sphere(ray, a, radius) {
        if len2 <= 1e-8 || (ray.at(t) - a).dot(axis) / denom <= 0.0 {
            best = best.min(t);
        }
    }
    if let Some(t) = ray_sphere(ray, b, radius) {
        if len2 <= 1e-8 || (ray.at(t) - a).dot(axis) / denom >= 1.0 {
            best = best.min(t);
        }
    }

    best.is_finite().then_some(best)
}

#[cfg(test)]
mod tests {
    use super::{ray_capsule, ray_sphere};
    use glam::Vec3;
    use pocket3d_core::Ray;

    /// Regression (review finding): ray_capsule must return the SURFACE entry
    /// distance, not the closest-approach parameter — otherwise the hit point
    /// lands inside the bot and distance is overstated by up to the radius.
    #[test]
    fn ray_capsule_returns_surface_entry() {
        // Vertical capsule at x=100, radius 16; ray along +X at the mid-height.
        let a = Vec3::new(100.0, 0.0, 0.0);
        let b = Vec3::new(100.0, 0.0, 72.0);
        let ray = Ray::new(Vec3::new(0.0, 0.0, 36.0), Vec3::new(1.0, 0.0, 0.0));
        let t = ray_capsule(&ray, a, b, 16.0).expect("ray should hit capsule");
        // Enters the near surface at x = 100 - 16 = 84, NOT the axis at 100.
        assert!((t - 84.0).abs() < 1.0, "surface entry t≈84, got {t}");
    }

    /// A shot aimed at the torso must miss the (higher) head sphere, so it is
    /// classified as a body hit — not a 2x headshot.
    #[test]
    fn torso_shot_misses_head_sphere() {
        let a = Vec3::new(100.0, 0.0, 0.0);
        let b = Vec3::new(100.0, 0.0, 72.0);
        let head = Vec3::new(100.0, 0.0, 60.0);
        // Aim at chest height (z=36).
        let ray = Ray::new(Vec3::new(0.0, 0.0, 36.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(ray_capsule(&ray, a, b, 16.0).is_some(), "body is hit");
        assert!(
            ray_sphere(&ray, head, 10.0).is_none(),
            "chest shot must not register on the head sphere"
        );
    }

    /// A ray pointing away from the capsule returns no hit (t stays positive).
    #[test]
    fn ray_capsule_behind_shooter_is_none() {
        let a = Vec3::new(100.0, 0.0, 0.0);
        let b = Vec3::new(100.0, 0.0, 72.0);
        let ray = Ray::new(Vec3::new(0.0, 0.0, 36.0), Vec3::new(-1.0, 0.0, 0.0));
        assert!(ray_capsule(&ray, a, b, 16.0).is_none());
    }
}
