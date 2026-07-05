//! OpenStrike library surface: the authoritative simulation is exposed so both
//! the `openstrike` binary and integration tests can drive it (DESIGN.md §6).

pub mod sim;

#[cfg(feature = "window")]
pub mod game;
