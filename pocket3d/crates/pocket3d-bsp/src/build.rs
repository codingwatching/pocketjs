//! Turn a [`RawBsp`] + resolved textures into renderable world geometry, a
//! packed lightmap atlas, and static collision (DESIGN.md §10, §12).

use crate::format::{RawBsp, TEX_SPECIAL};
use crate::wad::{decode_miptex, MipTexHeader, Wad};
use crate::reader::Reader;
use glam::Vec3;
use pocket3d_core::{
    material::{MaterialDesc, MaterialKind},
    mesh::{CollisionMesh, MeshData, Submesh, WorldVertex},
    texture::TextureData,
    Aabb,
};

/// GoldSrc lightmaps sample one luxel per 16 world units.
const LUXEL_SIZE: f32 = 16.0;
/// Fixed atlas width; height grows via shelf packing.
const ATLAS_W: u32 = 1024;
/// Clamp pathological lightmap blocks.
const MAX_LM_EXTENT: i32 = 256;

/// A texture resolved (from embedded data or a WAD) and classified.
pub struct ResolvedTexture {
    pub name: String,
    pub texture: TextureData,
    pub kind: MaterialKind,
    /// True if the texture existed (embedded or found in a WAD).
    pub found: bool,
}

/// Classify a GoldSrc texture name into a material kind.
pub fn classify(name: &str) -> MaterialKind {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("sky") {
        MaterialKind::BspSky
    } else if name.starts_with('{') {
        MaterialKind::BspAlphaTest
    } else if name.starts_with('!') || lower.contains("water") || lower.contains("liquid") {
        MaterialKind::BspWater
    } else {
        MaterialKind::BspWorldLit
    }
}

/// Non-rendered tool textures (kept out of the visible mesh).
fn is_tool_texture(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "aaatrigger" | "null" | "nodraw" | "skip" | "hint" | "origin" | "clip"
    )
}

/// Resolve every miptex in the BSP textures lump, pulling external textures from
/// the provided WADs and falling back to a magenta checker when missing.
pub fn resolve_textures(raw: &RawBsp, wads: &Wad) -> Vec<ResolvedTexture> {
    let lump = &raw.textures_lump;
    let mut out = Vec::new();
    if lump.len() < 4 {
        return out;
    }
    let mut r = Reader::new(lump);
    let num = match r.i32() {
        Ok(n) if n >= 0 => n as usize,
        _ => return out,
    };
    // Bound the reservation by what the lump can physically hold (one i32 offset
    // per miptex) so a hostile count can't request a multi-GB allocation.
    let num = num.min(lump.len().saturating_sub(4) / 4);
    let mut offsets = Vec::with_capacity(num);
    for _ in 0..num {
        offsets.push(r.i32().unwrap_or(-1));
    }

    for ofs in offsets {
        if ofs < 0 || (ofs as usize) >= lump.len() {
            out.push(missing_texture("<invalid>"));
            continue;
        }
        let bytes = &lump[ofs as usize..];
        let hdr = match MipTexHeader::parse(bytes) {
            Ok(h) => h,
            Err(_) => {
                out.push(missing_texture("<bad>"));
                continue;
            }
        };
        let kind = classify(&hdr.name);
        let (texture, found) = if hdr.has_embedded_pixels() {
            match decode_miptex(bytes, &hdr.name) {
                Ok(t) => (t, true),
                Err(_) => (TextureData::missing(), false),
            }
        } else {
            match wads.decode(&hdr.name) {
                Some(t) => (t, true),
                None => (TextureData::missing(), false),
            }
        };
        out.push(ResolvedTexture {
            name: hdr.name,
            texture,
            kind,
            found,
        });
    }
    out
}

fn missing_texture(name: &str) -> ResolvedTexture {
    ResolvedTexture {
        name: name.to_string(),
        texture: TextureData::missing(),
        kind: MaterialKind::BspWorldLit,
        found: false,
    }
}

/// A single face resolved to world-space polygon vertices + texture math.
struct FaceGeom {
    positions: Vec<Vec3>,
    /// Raw (undivided) texture-space S/T per vertex.
    st: Vec<(f32, f32)>,
    normal: Vec3,
    miptex: usize,
    light_ofs: i32,
    special: bool,
}

/// Reconstruct one face's polygon and per-vertex texture coordinates.
fn face_geom(raw: &RawBsp, face_idx: usize) -> Option<FaceGeom> {
    let face = &raw.faces[face_idx];
    let ti = raw.texinfos.get(face.texinfo as usize)?;
    let n = face.num_edges as usize;
    let mut positions = Vec::with_capacity(n);
    let mut st = Vec::with_capacity(n);
    for i in 0..n {
        let se_idx = face.first_edge as usize + i;
        let se = *raw.surf_edges.get(se_idx)?;
        let vidx = if se >= 0 {
            raw.edges.get(se as usize)?.v[0]
        } else {
            raw.edges.get((-se) as usize)?.v[1]
        };
        let pos = *raw.vertices.get(vidx as usize)?;
        let s = pos.dot(ti.s_axis) + ti.s_offset;
        let t = pos.dot(ti.t_axis) + ti.t_offset;
        positions.push(pos);
        st.push((s, t));
    }
    let mut normal = raw
        .planes
        .get(face.plane as usize)
        .map(|p| p.normal)
        .unwrap_or(Vec3::Z);
    if face.side != 0 {
        normal = -normal;
    }
    Some(FaceGeom {
        positions,
        st,
        normal,
        miptex: ti.miptex as usize,
        light_ofs: face.light_ofs,
        special: (ti.flags & TEX_SPECIAL) != 0,
    })
}

/// A packed lightmap block for one face.
#[derive(Clone, Copy)]
struct LmBlock {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    bmin_s: i32,
    bmin_t: i32,
    lit: bool,
}

/// Simple shelf packer over a fixed-width atlas.
struct ShelfPacker {
    width: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_h: u32,
}

impl ShelfPacker {
    fn new(width: u32) -> Self {
        Self {
            width,
            cursor_x: 0,
            cursor_y: 0,
            shelf_h: 0,
        }
    }

    /// Place a `w×h` block (with 1px padding) and return its top-left corner.
    fn place(&mut self, w: u32, h: u32) -> (u32, u32) {
        let pw = w + 1;
        if self.cursor_x + pw > self.width {
            self.cursor_y += self.shelf_h;
            self.cursor_x = 0;
            self.shelf_h = 0;
        }
        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += pw;
        self.shelf_h = self.shelf_h.max(h + 1);
        pos
    }

    fn height(&self) -> u32 {
        (self.cursor_y + self.shelf_h).max(1)
    }
}

/// Compute the luxel extents of a face from its S/T ranges.
fn lm_extents(g: &FaceGeom) -> (i32, i32, i32, i32) {
    let mut min_s = f32::INFINITY;
    let mut max_s = f32::NEG_INFINITY;
    let mut min_t = f32::INFINITY;
    let mut max_t = f32::NEG_INFINITY;
    for &(s, t) in &g.st {
        min_s = min_s.min(s);
        max_s = max_s.max(s);
        min_t = min_t.min(t);
        max_t = max_t.max(t);
    }
    let bmin_s = (min_s / LUXEL_SIZE).floor() as i32;
    let bmax_s = (max_s / LUXEL_SIZE).ceil() as i32;
    let bmin_t = (min_t / LUXEL_SIZE).floor() as i32;
    let bmax_t = (max_t / LUXEL_SIZE).ceil() as i32;
    let w = (bmax_s - bmin_s + 1).clamp(1, MAX_LM_EXTENT);
    let h = (bmax_t - bmin_t + 1).clamp(1, MAX_LM_EXTENT);
    (bmin_s, bmin_t, w, h)
}

/// The compiled render geometry + lightmap atlas.
pub struct BuiltGeometry {
    pub mesh: MeshData<WorldVertex>,
    pub materials: Vec<MaterialDesc>,
    pub lightmap_atlas: TextureData,
}

/// Build the world render mesh for `model_index`, packing lightmaps into one
/// atlas and batching draws by material (miptex).
pub fn build_world_mesh(
    raw: &RawBsp,
    textures: &[ResolvedTexture],
    model_index: usize,
) -> BuiltGeometry {
    let model = match raw.models.get(model_index) {
        Some(m) => m,
        None => {
            return BuiltGeometry {
                mesh: MeshData::new(),
                materials: Vec::new(),
                lightmap_atlas: white_atlas(),
            }
        }
    };
    let first = model.first_face as usize;
    let count = model.num_faces as usize;

    // Pass 1: resolve geometry + compute lightmap blocks + pack.
    let mut packer = ShelfPacker::new(ATLAS_W);
    // Reserve a 2×2 white block so unlit surfaces sample pure white.
    let white_pos = packer.place(2, 2);
    let mut faces: Vec<(usize, FaceGeom, LmBlock)> = Vec::new();
    for fi in first..first + count {
        let Some(g) = face_geom(raw, fi) else { continue };
        // Skip tool textures and degenerate polys from the visible mesh.
        let name = textures.get(g.miptex).map(|t| t.name.as_str()).unwrap_or("");
        if g.positions.len() < 3 || is_tool_texture(name) {
            continue;
        }
        let (bmin_s, bmin_t, w, h) = lm_extents(&g);
        let has_light = g.light_ofs >= 0
            && !g.special
            && (g.light_ofs as usize + (w * h * 3) as usize) <= raw.lighting.len();
        let (x, y) = if has_light {
            packer.place(w as u32, h as u32)
        } else {
            white_pos
        };
        let block = LmBlock {
            x,
            y,
            w: w as u32,
            h: h as u32,
            bmin_s,
            bmin_t,
            lit: has_light,
        };
        faces.push((fi, g, block));
    }

    let atlas_h = packer.height().min(8192);
    let mut atlas = TextureData::new(ATLAS_W, atlas_h);
    // White reserve block.
    blit_white(&mut atlas, white_pos.0, white_pos.1, 2, 2);
    for (_, g, block) in &faces {
        if block.lit {
            blit_lightmap(&mut atlas, &raw.lighting, g.light_ofs as usize, block);
        }
    }
    let (aw, ah) = (atlas.width as f32, atlas.height as f32);

    // Pass 2: build the mesh, batched by miptex material.
    let mut mesh = MeshData::new();
    let num_tex = textures.len();
    // Group faces by material index.
    let mut by_mat: Vec<Vec<usize>> = vec![Vec::new(); num_tex.max(1)];
    for (idx, (_, g, _)) in faces.iter().enumerate() {
        let m = g.miptex.min(by_mat.len() - 1);
        by_mat[m].push(idx);
    }

    let mut materials = Vec::new();
    for (mat_idx, face_list) in by_mat.iter().enumerate() {
        if face_list.is_empty() {
            continue;
        }
        let index_start = mesh.indices.len() as u32;
        for &fli in face_list {
            let (_, g, block) = &faces[fli];
            let base = mesh.vertices.len() as u32;
            let tex = textures.get(g.miptex);
            let (tw, th) = tex
                .map(|t| (t.texture.width.max(1) as f32, t.texture.height.max(1) as f32))
                .unwrap_or((64.0, 64.0));
            for (vi, &pos) in g.positions.iter().enumerate() {
                let (s, t) = g.st[vi];
                let uv = [s / tw, t / th];
                let uv_lm = if block.lit {
                    let ls = s / LUXEL_SIZE - block.bmin_s as f32;
                    let lt = t / LUXEL_SIZE - block.bmin_t as f32;
                    [
                        (block.x as f32 + ls + 0.5) / aw,
                        (block.y as f32 + lt + 0.5) / ah,
                    ]
                } else {
                    // Center of the reserved white texel.
                    [(white_pos.0 as f32 + 0.5) / aw, (white_pos.1 as f32 + 0.5) / ah]
                };
                mesh.vertices.push(WorldVertex {
                    pos: pos.to_array(),
                    normal: g.normal.to_array(),
                    uv,
                    uv_lm,
                });
                mesh.bounds.grow(pos);
            }
            // Fan triangulation.
            let n = g.positions.len() as u32;
            for i in 1..n - 1 {
                mesh.indices.push(base);
                mesh.indices.push(base + i);
                mesh.indices.push(base + i + 1);
            }
        }
        let index_count = mesh.indices.len() as u32 - index_start;
        let name = textures.get(mat_idx).map(|t| t.name.clone()).unwrap_or_default();
        let kind = textures
            .get(mat_idx)
            .map(|t| t.kind)
            .unwrap_or(MaterialKind::BspWorldLit);
        let mut desc = MaterialDesc::new(name, kind);
        desc.base_texture = Some(mat_idx as u32);
        desc.lightmap_atlas = Some(0);
        mesh.submeshes.push(Submesh {
            material: materials.len() as u32,
            index_start,
            index_count,
        });
        materials.push(desc);
    }

    if !mesh.bounds.is_valid() {
        mesh.bounds = Aabb::from_min_max(Vec3::ZERO, Vec3::ZERO);
    }

    BuiltGeometry {
        mesh,
        materials,
        lightmap_atlas: atlas,
    }
}

fn white_atlas() -> TextureData {
    TextureData::solid(255, 255, 255, 255)
}

fn blit_white(atlas: &mut TextureData, x: u32, y: u32, w: u32, h: u32) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < atlas.width && py < atlas.height {
                let o = ((py * atlas.width + px) * 4) as usize;
                atlas.rgba[o] = 255;
                atlas.rgba[o + 1] = 255;
                atlas.rgba[o + 2] = 255;
                atlas.rgba[o + 3] = 255;
            }
        }
    }
}

fn blit_lightmap(atlas: &mut TextureData, lighting: &[u8], ofs: usize, block: &LmBlock) {
    for ly in 0..block.h {
        for lx in 0..block.w {
            let src = ofs + ((ly * block.w + lx) * 3) as usize;
            if src + 2 >= lighting.len() {
                continue;
            }
            let px = block.x + lx;
            let py = block.y + ly;
            if px < atlas.width && py < atlas.height {
                let o = ((py * atlas.width + px) * 4) as usize;
                atlas.rgba[o] = lighting[src];
                atlas.rgba[o + 1] = lighting[src + 1];
                atlas.rgba[o + 2] = lighting[src + 2];
                atlas.rgba[o + 3] = 255;
            }
        }
    }
}

/// Build static collision (triangle soup) for `model_index`.
pub fn build_collision(raw: &RawBsp, model_index: usize) -> CollisionMesh {
    let mut cm = CollisionMesh::new();
    let Some(model) = raw.models.get(model_index) else {
        return cm;
    };
    let first = model.first_face as usize;
    let count = model.num_faces as usize;
    for fi in first..first + count {
        let Some(g) = face_geom(raw, fi) else { continue };
        if g.positions.len() < 3 {
            continue;
        }
        // Fan triangulate the convex face polygon.
        for i in 1..g.positions.len() - 1 {
            cm.push_triangle(g.positions[0], g.positions[i], g.positions[i + 1]);
        }
    }
    cm
}
