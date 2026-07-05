//! OpenStrike — the first Pocket3D application (DESIGN.md §1, §24).
//!
//! Two entry paths share one authoritative [`sim::Sim`]:
//! - `openstrike sim`  : headless, deterministic runner that drives the full
//!   round loop with an autopilot and prints events — verifiable without a GPU.
//! - `openstrike run`  : windowed play via the wgpu backend (feature `window`).

use openstrike::sim;

#[cfg(feature = "window")]
use openstrike::game;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pocket3d_bsp::load_bsp;
use pocket3d_script::{BotConfig, RoundConfig, ScriptEngine, WeaponConfig};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "openstrike", about = "A BSP-based FPS example on Pocket3D")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the simulation headlessly and print the round loop (no window).
    Sim(SimArgs),
    /// Run windowed (requires building with `--features window`).
    Run(RunArgs),
    /// Load a map and report its assets / spawns.
    CheckAssets(CheckArgs),
}

#[derive(Parser)]
struct SimArgs {
    #[arg(long)]
    map: Option<PathBuf>,
    #[arg(long)]
    wads: Option<PathBuf>,
    #[arg(long)]
    script: Option<PathBuf>,
    /// Number of fixed ticks to simulate.
    #[arg(long, default_value_t = 1200)]
    ticks: u32,
    #[arg(long, default_value_t = 2)]
    bots: usize,
}

#[derive(Parser)]
struct RunArgs {
    #[arg(long)]
    map: Option<PathBuf>,
    #[arg(long)]
    wads: Option<PathBuf>,
    #[arg(long)]
    script: Option<PathBuf>,
    #[arg(long, default_value_t = 2)]
    bots: usize,
}

#[derive(Parser)]
struct CheckArgs {
    #[arg(long)]
    map: Option<PathBuf>,
    #[arg(long)]
    wads: Option<PathBuf>,
}

/// Default dev map/wad locations (gitignored, staged locally).
fn default_map() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("maps/de_dust2.bsp")
}
fn default_wads() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/wads")
}

/// Load weapon/round/bot config from a script, or fall back to defaults.
fn load_config(script: &Option<PathBuf>) -> (WeaponConfig, RoundConfig, BotConfig) {
    if let Some(path) = script {
        match ScriptEngine::new().and_then(|e| {
            e.load_file(path)?;
            Ok(e)
        }) {
            Ok(engine) => {
                println!("[script] loaded {}", path.display());
                return (engine.weapon(), engine.round(), engine.bot());
            }
            Err(e) => eprintln!("[script] failed ({e}); using defaults"),
        }
    }
    (
        WeaponConfig::default(),
        RoundConfig::default(),
        BotConfig::default(),
    )
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Sim(a) => run_sim(a),
        Cmd::CheckAssets(a) => run_check(a),
        Cmd::Run(a) => run_windowed(a),
    }
}

fn run_sim(args: SimArgs) -> Result<()> {
    let map = args.map.unwrap_or_else(default_map);
    let wads = args.wads.unwrap_or_else(default_wads);
    let (weapon, round, bot) = load_config(&args.script);

    println!("Loading map {} ...", map.display());
    let bsp = load_bsp(&map, &[wads]).context("loading BSP")?;
    println!(
        "  v{}  {} world tris, {} collision tris, {} spawns, atlas {}x{}",
        bsp.info.version,
        bsp.info.world_triangles,
        bsp.info.collision_triangles,
        bsp.info.spawn_count,
        bsp.lightmap_atlas.width,
        bsp.lightmap_atlas.height
    );

    let mut sim = sim::Sim::new(&bsp, weapon, round, bot, args.bots.max(1));
    println!(
        "Spawned player + {} bots. Weapon dmg {}, {} waypoints.",
        sim.bots.len(),
        sim.weapon.config.damage,
        sim.waypoints.len(),
    );

    let dt = 1.0 / 60.0;
    let mut rounds_won = 0u32;
    let mut bots_killed = 0u32;
    let mut shots = 0u32;
    let mut event_cursor = 0usize;

    for tick in 0..args.ticks {
        let control = sim.autopilot_control();
        sim.tick(&control, dt);

        // Report new events.
        while event_cursor < sim.events.len() {
            match &sim.events[event_cursor] {
                pocket3d_core::Event::RoundStart { round } => {
                    println!("[t{tick:>4}] ROUND {round} START — {} bots", sim.alive_bots());
                }
                pocket3d_core::Event::ShotFired { .. } => shots += 1,
                pocket3d_core::Event::BotKilled { .. } => {
                    bots_killed += 1;
                    println!("[t{tick:>4}] bot killed ({} left)", sim.alive_bots());
                }
                pocket3d_core::Event::RoundEnd { round, player_won } => {
                    if *player_won {
                        rounds_won += 1;
                    }
                    println!(
                        "[t{tick:>4}] ROUND {round} END — player {}",
                        if *player_won { "WON" } else { "LOST" }
                    );
                }
                _ => {}
            }
            event_cursor += 1;
        }
    }

    println!("\n=== Summary after {} ticks ({:.1}s sim time) ===", args.ticks, args.ticks as f32 * dt);
    println!("  rounds won   : {rounds_won}");
    println!("  bots killed  : {bots_killed}");
    println!("  shots fired  : {shots}");
    println!("  round state  : {:?} (round #{})", sim.state, sim.round_number);
    println!("  player pos   : {:?}", sim.player.body.position);
    println!("  player health: {}", sim.player.health);

    if rounds_won == 0 {
        anyhow::bail!("no round was won in {} ticks — loop did not complete", args.ticks);
    }
    println!("\nOK: the spawn -> aim -> shoot -> kill -> win -> restart loop ran.");
    Ok(())
}

fn run_check(args: CheckArgs) -> Result<()> {
    let map = args.map.unwrap_or_else(default_map);
    let wads = args.wads.unwrap_or_else(default_wads);
    if !map.exists() {
        anyhow::bail!("map not found: {} (stage a dev map there)", map.display());
    }
    let bsp = load_bsp(&map, &[wads])?;
    println!("Map: {}", map.display());
    println!("  BSP version      : {}", bsp.info.version);
    println!("  world triangles  : {}", bsp.info.world_triangles);
    println!("  collision tris   : {}", bsp.info.collision_triangles);
    println!("  textures         : {}", bsp.info.texture_count);
    println!("  missing textures : {}", bsp.info.missing_textures.len());
    println!("  spawn points     : {}", bsp.info.spawn_count);
    println!("  lightmap atlas   : {}x{}", bsp.lightmap_atlas.width, bsp.lightmap_atlas.height);
    Ok(())
}

#[cfg(feature = "window")]
fn run_windowed(args: RunArgs) -> Result<()> {
    let map = args.map.unwrap_or_else(default_map);
    let wads = args.wads.unwrap_or_else(default_wads);
    let (weapon, round, bot) = load_config(&args.script);
    game::run(&map, &wads, weapon, round, bot, args.bots.max(1))
}

#[cfg(not(feature = "window"))]
fn run_windowed(_args: RunArgs) -> Result<()> {
    anyhow::bail!(
        "windowed play requires building with `--features window` (wgpu backend). \
         Use `openstrike sim` for the headless loop."
    );
}

// Silence unused-import warnings when the `window` feature is off.
#[allow(dead_code)]
fn _touch(_p: &Path) {}
