//! Geometry primitive **types** shared across physics, KCC, and BSP.
//!
//! Heavy intersection algorithms (BVH traversal, capsule sweeps) live in
//! `pocket3d-physics`; this module keeps the shared data types plus cheap,
//! dependency-free helpers so every crate agrees on the same primitives.

use glam::Vec3;

/// An axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb {
    /// The empty box, which grows to fit any points added to it.
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb {
    /// An empty (inverted) box that grows to fit points added to it.
    pub const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut b = Self::EMPTY;
        for p in points {
            b.grow(p);
        }
        b
    }

    pub fn grow(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn merge(&mut self, other: &Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    pub fn contains(&self, p: Vec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    /// Expand the box by `r` on every axis (Minkowski inflation).
    pub fn expanded(&self, r: f32) -> Aabb {
        Aabb {
            min: self.min - Vec3::splat(r),
            max: self.max + Vec3::splat(r),
        }
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }
}

/// An infinite plane `dot(normal, x) = d`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(normal: Vec3, d: f32) -> Self {
        Self { normal, d }
    }

    /// Signed distance from `p` to the plane (positive on the normal side).
    pub fn distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) - self.d
    }
}

/// A ray with a normalized direction.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self {
            origin,
            dir: dir.normalize_or_zero(),
        }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// The result of a successful ray/shape cast.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    /// Distance along the ray to the hit point.
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

/// A triangle in world space (CCW winding gives the front-face normal).
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self { a, b, c }
    }

    pub fn normal(&self) -> Vec3 {
        (self.b - self.a).cross(self.c - self.a).normalize_or_zero()
    }

    pub fn centroid(&self) -> Vec3 {
        (self.a + self.b + self.c) / 3.0
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::from_points([self.a, self.b, self.c])
    }
}

/// A vertical-ish capsule defined by its two segment endpoints and a radius.
/// For a standing character, `a` is the lower sphere center and `b` the upper.
#[derive(Clone, Copy, Debug)]
pub struct Capsule {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}

impl Capsule {
    pub fn new(a: Vec3, b: Vec3, radius: f32) -> Self {
        Self { a, b, radius }
    }

    /// Build a Z-up capsule from a base (feet) point, total height, and radius.
    /// The segment endpoints are inset by `radius` so the sphere caps reach the
    /// feet and the crown.
    pub fn from_base_height(base: Vec3, height: f32, radius: f32) -> Self {
        let a = base + Vec3::Z * radius;
        let b = base + Vec3::Z * (height - radius);
        Self { a, b, radius }
    }

    pub fn aabb(&self) -> Aabb {
        let mut b = Aabb::from_points([self.a, self.b]);
        b.min -= Vec3::splat(self.radius);
        b.max += Vec3::splat(self.radius);
        b
    }
}
