//! Simulation event types and a simple double-buffered queue.

use crate::world::EntityId;
use glam::Vec3;

/// What a bullet ray struck.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitKind {
    World,
    Bot,
    Nothing,
}

/// A resolved hitscan result for one shot.
#[derive(Clone, Copy, Debug)]
pub struct HitEvent {
    pub kind: HitKind,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    /// The entity hit, if `kind == Bot`.
    pub entity: Option<EntityId>,
    /// Whether the hit landed on a head hitbox.
    pub headshot: bool,
    pub damage: f32,
}

/// High-level simulation events surfaced to scripts / HUD / audio.
#[derive(Clone, Debug)]
pub enum Event {
    RoundStart { round: u32 },
    RoundEnd { round: u32, player_won: bool },
    ShotFired { origin: Vec3, dir: Vec3 },
    Hit(HitEvent),
    BotKilled { entity: EntityId },
    PlayerKilled,
    TriggerEnter { entity: EntityId, trigger: String },
}

/// A drain-per-tick event queue.
#[derive(Default)]
pub struct EventQueue {
    events: Vec<Event>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, e: Event) {
        self.events.push(e);
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Take all pending events, leaving the queue empty.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }
}
