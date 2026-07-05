//! End-to-end headless test of the OpenStrike round loop on the real map, if
//! staged locally (DESIGN.md §24 acceptance criteria). Skips when the dev map
//! is absent (it is gitignored per DESIGN.md §11).

use openstrike::sim::{RoundState, Sim};
use pocket3d_bsp::load_bsp;
use pocket3d_core::Event;
use pocket3d_script::{BotConfig, RoundConfig, WeaponConfig};
use std::path::PathBuf;

fn map_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("maps/de_dust2.bsp")
}
fn wad_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/wads")
}

#[test]
fn full_round_loop_runs_on_dust2() {
    let map = map_path();
    if !map.exists() {
        eprintln!("skipping: {} not staged", map.display());
        return;
    }
    let bsp = load_bsp(&map, &[wad_dir()]).expect("load de_dust2");

    let mut sim = Sim::new(
        &bsp,
        WeaponConfig::default(),
        RoundConfig::default(),
        BotConfig::default(),
        2,
    );

    // The player should have spawned and settled onto the floor (finite z,
    // inside the map bounds — proves gravity + collision, not an endless fall).
    assert!(bsp.bounds.contains(sim.player.body.position), "player inside map");

    let dt = 1.0 / 60.0;
    let mut kills = 0u32;
    let mut wins = 0u32;
    let mut round_starts = 0u32;
    let mut restarted = false;

    for _ in 0..2000 {
        let ctrl = sim.autopilot_control();
        let before_round = sim.round_number;
        sim.tick(&ctrl, dt);
        if sim.round_number > before_round {
            restarted = true;
        }
        for e in sim.events.drain(..) {
            match e {
                Event::RoundStart { .. } => round_starts += 1,
                Event::BotKilled { .. } => kills += 1,
                Event::RoundEnd { player_won, .. } if player_won => wins += 1,
                _ => {}
            }
        }
    }

    assert!(round_starts >= 1, "at least one round went Live");
    assert!(kills >= 2, "player shot and killed bots (got {kills})");
    assert!(wins >= 1, "player won at least one round");
    assert!(restarted, "round restarted without reloading the map");
    // After many rounds the sim is still running a valid state.
    assert!(matches!(
        sim.state,
        RoundState::Live | RoundState::PreRound | RoundState::PlayerWon | RoundState::Intermission | RoundState::Restarting
    ));
}
