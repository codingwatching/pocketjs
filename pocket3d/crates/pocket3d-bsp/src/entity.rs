//! Entity lump parsing (DESIGN.md §10).
//!
//! The entity lump is plain text: a sequence of `{ "key" "value" ... }` blocks.
//! We preserve every key/value pair verbatim (even for entities OpenStrike does
//! not understand) so inspection tools can show them, then extract the subset
//! the game needs: spawn points and trigger volumes.

use glam::Vec3;
use std::collections::BTreeMap;

/// A raw entity: its classname plus all key/value pairs.
#[derive(Clone, Debug, Default)]
pub struct Entity {
    pub class_name: String,
    pub props: BTreeMap<String, String>,
}

impl Entity {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.props.get(key).map(|s| s.as_str())
    }

    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|v| v.trim().parse().ok())
    }

    /// Parse a `"x y z"` triple.
    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        let v = self.get(key)?;
        let mut it = v.split_whitespace().filter_map(|p| p.parse::<f32>().ok());
        Some(Vec3::new(it.next()?, it.next()?, it.next()?))
    }

    pub fn origin(&self) -> Option<Vec3> {
        self.get_vec3("origin")
    }

    /// A brush-entity model reference like `"*3"` → model index 3.
    pub fn brush_model_index(&self) -> Option<u32> {
        self.get("model")
            .and_then(|m| m.strip_prefix('*'))
            .and_then(|n| n.parse().ok())
    }

    /// Yaw in degrees from `angle` (GoldSrc single yaw) or `angles`.
    pub fn yaw_deg(&self) -> f32 {
        if let Some(a) = self.get_f32("angle") {
            return a;
        }
        if let Some(angles) = self.get_vec3("angles") {
            return angles.y; // angles = pitch yaw roll
        }
        0.0
    }
}

/// Which side a spawn point belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    /// Counter-Terrorist (`info_player_start`).
    Ct,
    /// Terrorist (`info_player_deathmatch`).
    T,
    Other,
}

/// A player spawn point.
#[derive(Clone, Copy, Debug)]
pub struct SpawnPoint {
    pub pos: Vec3,
    pub yaw_deg: f32,
    pub team: Team,
}

/// A brush-based trigger volume (buy zone, bomb target, trigger_*).
#[derive(Clone, Debug)]
pub struct TriggerVolume {
    pub class_name: String,
    pub target_name: Option<String>,
    /// Model index referenced by the entity (`"*N"`), if any.
    pub model_index: Option<u32>,
}

/// Parse the entity lump text into a list of entities.
pub fn parse_entities(text: &str) -> Vec<Entity> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find the next '{'.
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        i += 1; // past '{'
        let mut ent = Entity::default();
        loop {
            // Skip whitespace.
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] == b'}' {
                i += 1; // past '}'
                break;
            }
            // Expect a quoted key.
            let Some((key, ni)) = read_quoted(bytes, i) else {
                break;
            };
            i = ni;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let Some((val, ni)) = read_quoted(bytes, i) else {
                break;
            };
            i = ni;
            if key == "classname" {
                ent.class_name = val;
            } else {
                ent.props.insert(key, val);
            }
        }
        if !ent.class_name.is_empty() || !ent.props.is_empty() {
            out.push(ent);
        }
    }
    out
}

/// Read a `"..."` token starting at index `i` (which must point at the opening
/// quote). Returns the unquoted content and the index just past the close quote.
fn read_quoted(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    if i >= bytes.len() || bytes[i] != b'"' {
        return None;
    }
    let mut j = i + 1;
    let start = j;
    while j < bytes.len() && bytes[j] != b'"' {
        j += 1;
    }
    if j >= bytes.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&bytes[start..j]).into_owned();
    Some((s, j + 1))
}

/// Extract player spawn points from parsed entities.
pub fn extract_spawns(entities: &[Entity]) -> Vec<SpawnPoint> {
    entities
        .iter()
        .filter_map(|e| {
            let team = match e.class_name.as_str() {
                "info_player_start" => Team::Ct,
                "info_player_deathmatch" => Team::T,
                _ => return None,
            };
            let pos = e.origin()?;
            Some(SpawnPoint {
                pos,
                yaw_deg: e.yaw_deg(),
                team,
            })
        })
        .collect()
}

/// Extract trigger-like brush volumes (buy zones, bomb targets, triggers).
pub fn extract_triggers(entities: &[Entity]) -> Vec<TriggerVolume> {
    entities
        .iter()
        .filter(|e| {
            let c = e.class_name.as_str();
            c.starts_with("trigger_")
                || c == "func_buyzone"
                || c == "func_bomb_target"
                || c == "func_hostage_rescue"
        })
        .map(|e| TriggerVolume {
            class_name: e.class_name.clone(),
            target_name: e.get("targetname").map(|s| s.to_string()),
            model_index: e.brush_model_index(),
        })
        .collect()
}
