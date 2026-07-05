//! Shared CPU-side geometry payloads.
//!
//! These live in `core` so the BSP importer, physics, animation, and renderer
//! all agree on one vertex/mesh format without depending on each other.

use crate::geom::{Aabb, Triangle};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// A vertex of BSP world geometry: base texture UV plus lightmap UV.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct WorldVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    /// Base (diffuse) texture coordinates.
    pub uv: [f32; 2],
    /// Lightmap atlas coordinates.
    pub uv_lm: [f32; 2],
}

/// A vertex of a static (unlit/lit) mesh, e.g. a weapon viewmodel or prop.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct StaticVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// A vertex of a skinned mesh: up to four joint influences.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SkinnedVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

/// A contiguous run of indices that share one material (draw batch).
#[derive(Clone, Debug)]
pub struct Submesh {
    /// Index into the owning asset's material table.
    pub material: u32,
    /// Range into the shared index buffer.
    pub index_start: u32,
    pub index_count: u32,
}

/// A renderable mesh: interleaved vertices, a shared index buffer, and the
/// per-material submeshes that slice it.
#[derive(Clone, Debug, Default)]
pub struct MeshData<V> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub submeshes: Vec<Submesh>,
    pub bounds: Aabb,
}

impl<V> MeshData<V> {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            submeshes: Vec::new(),
            bounds: Aabb::EMPTY,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Static collision geometry: a triangle soup with a precomputed bounds box.
/// Physics builds a BVH over this; KCC and hitscan query against it.
#[derive(Clone, Debug, Default)]
pub struct CollisionMesh {
    pub positions: Vec<Vec3>,
    /// Triangle index triples into `positions`.
    pub indices: Vec<[u32; 3]>,
    pub bounds: Aabb,
}

impl CollisionMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    /// Materialize the `i`-th triangle in world space.
    pub fn triangle(&self, i: usize) -> Triangle {
        let [a, b, c] = self.indices[i];
        Triangle::new(
            self.positions[a as usize],
            self.positions[b as usize],
            self.positions[c as usize],
        )
    }

    pub fn iter_triangles(&self) -> impl Iterator<Item = Triangle> + '_ {
        (0..self.indices.len()).map(move |i| self.triangle(i))
    }

    /// Append a triangle by its three world-space corners.
    pub fn push_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let base = self.positions.len() as u32;
        self.positions.push(a);
        self.positions.push(b);
        self.positions.push(c);
        self.indices.push([base, base + 1, base + 2]);
        self.bounds.grow(a);
        self.bounds.grow(b);
        self.bounds.grow(c);
    }
}
