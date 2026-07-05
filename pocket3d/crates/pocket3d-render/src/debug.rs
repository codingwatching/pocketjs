//! Immediate-mode debug draw buffer (DESIGN.md §23).
//!
//! Gameplay/tools push primitives here each frame; the backend flushes them in
//! the debug pass. Capsules/AABBs/spheres are decomposed into line segments so
//! the backend only needs a line pipeline plus a text pipeline.

use glam::Vec3;
use pocket3d_core::{Aabb, Capsule};

pub type Color = [f32; 4];

pub const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
pub const RED: Color = [1.0, 0.2, 0.2, 1.0];
pub const GREEN: Color = [0.2, 1.0, 0.2, 1.0];
pub const BLUE: Color = [0.3, 0.5, 1.0, 1.0];
pub const YELLOW: Color = [1.0, 1.0, 0.2, 1.0];
pub const CYAN: Color = [0.2, 1.0, 1.0, 1.0];

/// A single debug line segment.
#[derive(Clone, Copy, Debug)]
pub struct DebugLine {
    pub a: Vec3,
    pub b: Vec3,
    pub color: Color,
}

/// A world-space debug text label.
#[derive(Clone, Debug)]
pub struct DebugText {
    pub pos: Vec3,
    pub text: String,
    pub color: Color,
}

/// A buffer of debug primitives for one frame.
#[derive(Clone, Debug, Default)]
pub struct DebugDraw {
    pub lines: Vec<DebugLine>,
    pub texts: Vec<DebugText>,
}

impl DebugDraw {
    pub fn clear(&mut self) {
        self.lines.clear();
        self.texts.clear();
    }

    pub fn line(&mut self, a: Vec3, b: Vec3, color: Color) {
        self.lines.push(DebugLine { a, b, color });
    }

    /// A ray drawn as a segment of length `len`.
    pub fn ray(&mut self, origin: Vec3, dir: Vec3, len: f32, color: Color) {
        self.line(origin, origin + dir.normalize_or_zero() * len, color);
    }

    /// A small 3-axis cross marker at `p`.
    pub fn cross(&mut self, p: Vec3, size: f32, color: Color) {
        let s = size * 0.5;
        self.line(p - Vec3::X * s, p + Vec3::X * s, color);
        self.line(p - Vec3::Y * s, p + Vec3::Y * s, color);
        self.line(p - Vec3::Z * s, p + Vec3::Z * s, color);
    }

    /// The 12 edges of an axis-aligned box.
    pub fn aabb(&mut self, b: &Aabb, color: Color) {
        let c = [
            Vec3::new(b.min.x, b.min.y, b.min.z),
            Vec3::new(b.max.x, b.min.y, b.min.z),
            Vec3::new(b.max.x, b.max.y, b.min.z),
            Vec3::new(b.min.x, b.max.y, b.min.z),
            Vec3::new(b.min.x, b.min.y, b.max.z),
            Vec3::new(b.max.x, b.min.y, b.max.z),
            Vec3::new(b.max.x, b.max.y, b.max.z),
            Vec3::new(b.min.x, b.max.y, b.max.z),
        ];
        const EDGES: [(usize, usize); 12] = [
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];
        for (i, j) in EDGES {
            self.line(c[i], c[j], color);
        }
    }

    /// A wireframe axis-aligned circle (`axis`: 0=X, 1=Y, 2=Z normal).
    fn circle(&mut self, center: Vec3, radius: f32, axis: u8, segments: usize, color: Color) {
        let n = segments.max(3);
        let mut prev = None;
        for i in 0..=n {
            let a = (i as f32 / n as f32) * std::f32::consts::TAU;
            let (s, c) = a.sin_cos();
            let p = match axis {
                0 => center + Vec3::new(0.0, c * radius, s * radius),
                1 => center + Vec3::new(c * radius, 0.0, s * radius),
                _ => center + Vec3::new(c * radius, s * radius, 0.0),
            };
            if let Some(pp) = prev {
                self.line(pp, p, color);
            }
            prev = Some(p);
        }
    }

    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Color) {
        self.circle(center, radius, 0, 16, color);
        self.circle(center, radius, 1, 16, color);
        self.circle(center, radius, 2, 16, color);
    }

    /// A capsule as two end circles + a mid ring + four connecting lines.
    pub fn capsule(&mut self, cap: &Capsule, color: Color) {
        self.sphere(cap.a, cap.radius, color);
        self.sphere(cap.b, cap.radius, color);
        // vertical connectors (assumes roughly Z-up capsule)
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let off = Vec3::new(dx, dy, 0.0) * cap.radius;
            self.line(cap.a + off, cap.b + off, color);
        }
    }

    pub fn text(&mut self, pos: Vec3, text: impl Into<String>, color: Color) {
        self.texts.push(DebugText {
            pos,
            text: text.into(),
            color,
        });
    }
}
