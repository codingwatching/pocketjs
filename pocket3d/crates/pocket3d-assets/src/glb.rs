//! A minimal, dependency-light **GLB writer** used only to emit our procedural
//! placeholder content (DESIGN.md §9 asset path, §11 project-owned content).
//!
//! This is intentionally *not* a general glTF exporter: it accumulates raw
//! binary attribute/index/animation buffers, tracks the `bufferViews` and
//! `accessors` that slice them, and finally assembles a valid single-`.glb`
//! (binary glTF) — a 12-byte header followed by a JSON chunk and a BIN chunk.
//! The JSON is built with `serde_json`; the caller supplies the meshes, skins,
//! nodes and animations that reference the accessors produced here.

use serde_json::{json, Value};

// glTF accessor `componentType` values.
pub const F32: u32 = 5126;
pub const U16: u32 = 5123;
pub const U32: u32 = 5125;

// glTF `bufferView` targets.
pub const ARRAY_BUFFER: u32 = 34962;
pub const ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// Accumulates the BIN chunk plus the `bufferViews`/`accessors` that index it.
#[derive(Default)]
pub struct BinBuilder {
    data: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl BinBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append raw bytes as a new `bufferView`, 4-byte aligned. Returns its index.
    fn push_view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        // glTF requires bufferView byteOffsets usable by accessors to be
        // aligned; we align every view to 4 bytes.
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
        let offset = self.data.len();
        self.data.extend_from_slice(bytes);
        let mut view = json!({
            "buffer": 0,
            "byteOffset": offset,
            "byteLength": bytes.len(),
        });
        if let Some(t) = target {
            view["target"] = json!(t);
        }
        let idx = self.views.len();
        self.views.push(view);
        idx
    }

    /// Push a f32 VEC3 accessor (positions/normals/translations/scales).
    pub fn accessor_vec3(&mut self, values: &[[f32; 3]], target: Option<u32>) -> usize {
        let mut bytes = Vec::with_capacity(values.len() * 12);
        for v in values {
            for c in v {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.push_view(&bytes, target);
        // POSITION accessors must carry min/max per the glTF spec; it is cheap
        // and harmless to always include them for VEC3.
        let (min, max) = min_max_vec3(values);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": F32,
            "count": values.len(),
            "type": "VEC3",
            "min": min,
            "max": max,
        }))
    }

    /// Push a f32 VEC2 accessor (texture coordinates).
    pub fn accessor_vec2(&mut self, values: &[[f32; 2]], target: Option<u32>) -> usize {
        let mut bytes = Vec::with_capacity(values.len() * 8);
        for v in values {
            for c in v {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.push_view(&bytes, target);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": F32,
            "count": values.len(),
            "type": "VEC2",
        }))
    }

    /// Push a f32 VEC4 accessor (weights / rotation quaternions).
    pub fn accessor_vec4_f32(&mut self, values: &[[f32; 4]], target: Option<u32>) -> usize {
        let mut bytes = Vec::with_capacity(values.len() * 16);
        for v in values {
            for c in v {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.push_view(&bytes, target);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": F32,
            "count": values.len(),
            "type": "VEC4",
        }))
    }

    /// Push a u16 VEC4 accessor (JOINTS_0).
    pub fn accessor_vec4_u16(&mut self, values: &[[u16; 4]], target: Option<u32>) -> usize {
        let mut bytes = Vec::with_capacity(values.len() * 8);
        for v in values {
            for c in v {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.push_view(&bytes, target);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": U16,
            "count": values.len(),
            "type": "VEC4",
        }))
    }

    /// Push a u32 SCALAR index accessor.
    pub fn accessor_indices(&mut self, indices: &[u32]) -> usize {
        let mut bytes = Vec::with_capacity(indices.len() * 4);
        for i in indices {
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        let view = self.push_view(&bytes, Some(ELEMENT_ARRAY_BUFFER));
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": U32,
            "count": indices.len(),
            "type": "SCALAR",
        }))
    }

    /// Push a f32 SCALAR accessor with min/max (animation sampler inputs/times).
    pub fn accessor_scalar_f32(&mut self, values: &[f32]) -> usize {
        let mut bytes = Vec::with_capacity(values.len() * 4);
        for v in values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let view = self.push_view(&bytes, None);
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": F32,
            "count": values.len(),
            "type": "SCALAR",
            "min": [min],
            "max": [max],
        }))
    }

    /// Push a f32 MAT4 accessor (inverseBindMatrices), column-major per glTF.
    pub fn accessor_mat4(&mut self, mats: &[[f32; 16]]) -> usize {
        let mut bytes = Vec::with_capacity(mats.len() * 64);
        for m in mats {
            for c in m {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        let view = self.push_view(&bytes, None);
        self.push_accessor(json!({
            "bufferView": view,
            "componentType": F32,
            "count": mats.len(),
            "type": "MAT4",
        }))
    }

    fn push_accessor(&mut self, accessor: Value) -> usize {
        let idx = self.accessors.len();
        self.accessors.push(accessor);
        idx
    }

    pub fn buffer_views(&self) -> &[Value] {
        &self.views
    }

    pub fn accessors(&self) -> &[Value] {
        &self.accessors
    }

    /// Total BIN byte length (used for the `buffers[0].byteLength`).
    pub fn bin_len(&self) -> usize {
        self.data.len()
    }

    pub fn into_bin(self) -> Vec<u8> {
        self.data
    }
}

fn min_max_vec3(values: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in values {
        for i in 0..3 {
            min[i] = min[i].min(v[i]);
            max[i] = max[i].max(v[i]);
        }
    }
    // Guard against an empty accessor producing infinities.
    if values.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }
    (min, max)
}

/// Assemble a complete `.glb` from a glTF JSON document and a BIN blob.
///
/// GLB layout (glTF 2.0 §"Binary glTF"): a 12-byte header (`glTF` magic,
/// version 2, total length) followed by a JSON chunk (padded with spaces to a
/// 4-byte boundary) and a BIN chunk (padded with zeros).
pub fn assemble_glb(json: &Value, bin: &[u8]) -> Vec<u8> {
    let mut json_bytes = serde_json::to_vec(json).expect("glTF json serializes");
    // Pad JSON chunk with spaces to a 4-byte boundary.
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_bytes = bin.to_vec();
    while bin_bytes.len() % 4 != 0 {
        bin_bytes.push(0);
    }

    const HEADER: usize = 12;
    const CHUNK_HEADER: usize = 8;
    let total =
        HEADER + CHUNK_HEADER + json_bytes.len() + CHUNK_HEADER + bin_bytes.len();

    let mut out = Vec::with_capacity(total);
    // Header.
    out.extend_from_slice(&0x4654_6C67u32.to_le_bytes()); // "glTF"
    out.extend_from_slice(&2u32.to_le_bytes()); // version
    out.extend_from_slice(&(total as u32).to_le_bytes());
    // JSON chunk.
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes()); // "JSON"
    out.extend_from_slice(&json_bytes);
    // BIN chunk.
    out.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes()); // "BIN\0"
    out.extend_from_slice(&bin_bytes);
    out
}
