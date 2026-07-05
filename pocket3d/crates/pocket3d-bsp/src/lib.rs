//! `pocket3d-bsp` — GoldSrc BSP v30 ingestion (DESIGN.md §10).
//!
//! Loads a `.bsp` map, resolves textures from `.wad` archives, generates a
//! renderable world mesh with a packed lightmap atlas, extracts static
//! collision geometry, and parses the entity lump into raw records plus
//! recognized spawn points and trigger volumes. BSP is a **first-class** format
//! here: the importer keeps independent layers rather than collapsing to a
//! generic mesh.

pub mod build;
pub mod entity;
pub mod format;
pub mod reader;
pub mod wad;

pub use build::{classify, BuiltGeometry, ResolvedTexture};
pub use entity::{Entity, SpawnPoint, Team, TriggerVolume};
pub use format::{Header, LumpId, RawBsp, LUMP_NAMES};
pub use wad::Wad;

use anyhow::{Context, Result};
use glam::Vec3;
use pocket3d_core::{
    material::MaterialDesc,
    mesh::{CollisionMesh, MeshData, WorldVertex},
    texture::TextureData,
    Aabb,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The world index in the BSP models lump (worldspawn geometry).
pub const WORLD_MODEL: usize = 0;

/// A compiled BSP world with independent layers (DESIGN.md §10).
pub struct BspWorldAsset {
    pub info: BspInfo,
    /// World render geometry (batched by material).
    pub mesh: MeshData<WorldVertex>,
    /// Material table; `Submesh::material` indexes into this.
    pub materials: Vec<MaterialDesc>,
    /// Base textures; `MaterialDesc::base_texture` indexes into this.
    pub textures: Vec<TextureData>,
    /// One combined lightmap atlas.
    pub lightmap_atlas: TextureData,
    /// Static collision triangle soup (world model).
    pub collision: CollisionMesh,
    /// Every raw entity record, verbatim.
    pub entities: Vec<Entity>,
    /// Recognized player spawn points.
    pub spawns: Vec<SpawnPoint>,
    /// Recognized trigger volumes.
    pub triggers: Vec<TriggerVolume>,
    /// World bounds (model 0).
    pub bounds: Aabb,
}

impl BspWorldAsset {
    pub fn spawns_for(&self, team: Team) -> impl Iterator<Item = &SpawnPoint> {
        self.spawns.iter().filter(move |s| s.team == team)
    }

    /// The first spawn for a team, or any spawn, or map center.
    pub fn pick_spawn(&self, team: Team) -> SpawnPoint {
        self.spawns_for(team)
            .next()
            .or_else(|| self.spawns.first())
            .copied()
            .unwrap_or(SpawnPoint {
                pos: self.bounds.center(),
                yaw_deg: 0.0,
                team,
            })
    }
}

/// Inspector metadata (DESIGN.md §23 "Required inspector output").
#[derive(Clone, Debug)]
pub struct BspInfo {
    pub version: i32,
    /// `(name, offset, length, element_count)` per lump.
    pub lumps: Vec<LumpSummary>,
    pub texture_count: usize,
    pub missing_textures: Vec<String>,
    pub world_vertices: usize,
    pub world_triangles: usize,
    pub lightmap_atlas_size: (u32, u32),
    pub entity_count_by_classname: BTreeMap<String, usize>,
    pub collision_triangles: usize,
    pub spawn_count: usize,
    pub bounds: Aabb,
}

#[derive(Clone, Debug)]
pub struct LumpSummary {
    pub name: String,
    pub offset: u32,
    pub length: u32,
    pub count: u32,
}

/// Extract the WAD file basenames referenced by worldspawn's `wad` key.
pub fn referenced_wads(raw: &RawBsp) -> Vec<String> {
    let ents = entity::parse_entities(&raw.entities_text);
    let Some(world) = ents.iter().find(|e| e.class_name == "worldspawn") else {
        return Vec::new();
    };
    let Some(list) = world.get("wad") else {
        return Vec::new();
    };
    list.split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|p| {
            // Paths use backslashes; take the final component.
            let p = p.replace('\\', "/");
            p.rsplit('/').next().unwrap_or(&p).to_string()
        })
        .collect()
}

/// Locate and load the WADs a map references, searching `wad_dirs`.
pub fn load_referenced_wads(raw: &RawBsp, wad_dirs: &[PathBuf]) -> Wad {
    let mut combined = Wad::default();
    for name in referenced_wads(raw) {
        for dir in wad_dirs {
            let candidate = dir.join(&name);
            if candidate.exists() {
                if let Ok(w) = Wad::load_file(&candidate) {
                    combined.merge(w);
                }
                break;
            }
        }
    }
    // Also load any loose .wad files present in the search dirs, as a fallback.
    for dir in wad_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("wad") {
                    if let Ok(w) = Wad::load_file(&p) {
                        combined.merge(w);
                    }
                }
            }
        }
    }
    combined
}

#[allow(clippy::too_many_arguments)] // each layer is a distinct compiled output
fn build_info(
    raw: &RawBsp,
    textures: &[ResolvedTexture],
    mesh: &MeshData<WorldVertex>,
    atlas: &TextureData,
    collision: &CollisionMesh,
    entities: &[Entity],
    spawns: &[SpawnPoint],
    bounds: Aabb,
) -> BspInfo {
    use format::LumpId::*;
    let strides = [
        (Entities, 1u32),
        (Planes, 20),
        (Textures, 1),
        (Vertices, 12),
        (Visibility, 1),
        (Nodes, 24),
        (TexInfo, 40),
        (Faces, 20),
        (Lighting, 1),
        (ClipNodes, 8),
        (Leaves, 28),
        (MarkSurfaces, 2),
        (Edges, 4),
        (SurfEdges, 4),
        (Models, 64),
    ];
    let lumps = strides
        .iter()
        .map(|&(id, stride)| {
            let l = raw.header.lump(id);
            LumpSummary {
                name: LUMP_NAMES[id as usize].to_string(),
                offset: l.offset,
                length: l.length,
                count: l.length.checked_div(stride).unwrap_or(0),
            }
        })
        .collect();

    let missing_textures = textures
        .iter()
        .filter(|t| !t.found)
        .map(|t| t.name.clone())
        .collect();

    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    for e in entities {
        let key = if e.class_name.is_empty() {
            "<none>".to_string()
        } else {
            e.class_name.clone()
        };
        *by_class.entry(key).or_default() += 1;
    }

    BspInfo {
        version: raw.header.version,
        lumps,
        texture_count: textures.len(),
        missing_textures,
        world_vertices: mesh.vertices.len(),
        world_triangles: mesh.triangle_count(),
        lightmap_atlas_size: (atlas.width, atlas.height),
        entity_count_by_classname: by_class,
        collision_triangles: collision.triangle_count(),
        spawn_count: spawns.len(),
        bounds,
    }
}

/// Load and fully compile a BSP world from raw bytes, using already-loaded WADs.
pub fn compile_world(raw: &RawBsp, wads: &Wad) -> BspWorldAsset {
    let textures_resolved = build::resolve_textures(raw, wads);
    let geom = build::build_world_mesh(raw, &textures_resolved, WORLD_MODEL);
    let collision = build::build_collision(raw, WORLD_MODEL);
    let entities = entity::parse_entities(&raw.entities_text);
    let spawns = entity::extract_spawns(&entities);
    let triggers = entity::extract_triggers(&entities);
    let bounds = raw
        .models
        .get(WORLD_MODEL)
        .map(|m| Aabb::from_min_max(m.mins, m.maxs))
        .unwrap_or(geom.mesh.bounds);

    let info = build_info(
        raw,
        &textures_resolved,
        &geom.mesh,
        &geom.lightmap_atlas,
        &collision,
        &entities,
        &spawns,
        bounds,
    );

    let textures = textures_resolved.into_iter().map(|t| t.texture).collect();

    BspWorldAsset {
        info,
        mesh: geom.mesh,
        materials: geom.materials,
        textures,
        lightmap_atlas: geom.lightmap_atlas,
        collision,
        entities,
        spawns,
        triggers,
        bounds,
    }
}

/// Load a BSP map from a file path, resolving WADs from `wad_dirs`.
pub fn load_bsp(path: impl AsRef<Path>, wad_dirs: &[PathBuf]) -> Result<BspWorldAsset> {
    let path = path.as_ref();
    let data = std::fs::read(path).with_context(|| format!("reading BSP {}", path.display()))?;
    let raw = RawBsp::parse(&data).with_context(|| format!("parsing BSP {}", path.display()))?;
    let wads = load_referenced_wads(&raw, wad_dirs);
    Ok(compile_world(&raw, &wads))
}

/// Inspect a BSP without needing WADs present (missing textures are reported).
pub fn inspect(path: impl AsRef<Path>, wad_dirs: &[PathBuf]) -> Result<BspInfo> {
    let path = path.as_ref();
    let data = std::fs::read(path).with_context(|| format!("reading BSP {}", path.display()))?;
    let raw = RawBsp::parse(&data)?;
    let wads = load_referenced_wads(&raw, wad_dirs);
    Ok(compile_world(&raw, &wads).info)
}

/// Convenience: bounds center of a compiled world.
pub fn world_center(asset: &BspWorldAsset) -> Vec3 {
    asset.bounds.center()
}
