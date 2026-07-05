//! `pocket3d-kcc` — the kinematic character controller abstraction and its v0
//! implementation (DESIGN.md §14, §15).
//!
//! Per the design, character control is game-specific, so Pocket3D wraps it
//! behind the [`CharacterController`] trait rather than exposing a physics
//! engine directly. This crate ships [`SlideController`], a capsule
//! move-and-slide controller built on `pocket3d-physics` capsule/triangle
//! queries — the design's sanctioned "small Parry-style KCC using capsule casts
//! and explicit slide-plane resolution." It is deterministic and headless-
//! testable (no Rapier dependency), and a future Rapier-backed controller can
//! drop in behind the same trait.

use glam::Vec3;
use pocket3d_core::Capsule;
use pocket3d_physics::{Contact, PhysicsWorld};

/// A character's capsule + movement limits (feet-anchored).
#[derive(Clone, Copy, Debug)]
pub struct CharacterBody {
    /// Feet position (bottom of the capsule) in world space.
    pub position: Vec3,
    pub radius: f32,
    pub height: f32,
    /// Maximum ledge height the character auto-steps over.
    pub step_height: f32,
    /// Maximum walkable slope in degrees.
    pub slope_limit_deg: f32,
}

impl Default for CharacterBody {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            radius: 16.0,
            height: 72.0,
            step_height: 18.0,
            slope_limit_deg: 45.0,
        }
    }
}

impl CharacterBody {
    fn capsule_at(&self, feet: Vec3) -> Capsule {
        Capsule::from_base_height(feet, self.height, self.radius)
    }

    fn walkable_cos(&self) -> f32 {
        self.slope_limit_deg.to_radians().cos()
    }
}

/// The outcome of a move.
#[derive(Clone, Copy, Debug)]
pub struct CharacterMoveResult {
    /// New feet position.
    pub position: Vec3,
    pub grounded: bool,
    /// Ground surface normal when grounded (else up).
    pub ground_normal: Vec3,
    /// True if a near-vertical surface was contacted this move.
    pub hit_wall: bool,
    /// Actual displacement applied.
    pub moved: Vec3,
}

/// The KCC contract (DESIGN.md §14). Applications depend on this, not on a
/// specific physics engine.
pub trait CharacterController {
    fn move_character(
        &mut self,
        world: &PhysicsWorld,
        body: CharacterBody,
        desired_delta: Vec3,
        dt: f32,
    ) -> CharacterMoveResult;
}

/// Small separation kept between the capsule and surfaces.
const SKIN: f32 = 0.05;
/// Max depenetration iterations per settle.
const MAX_DEPEN: usize = 8;
/// Downward probe distance for ground detection.
const GROUND_PROBE: f32 = 2.0;
/// Largest single collision substep, so motion can't tunnel through geometry
/// thinner than this. Sized to half a typical capsule radius.
const MAX_SUBSTEP: f32 = 8.0;

/// A capsule move-and-slide controller.
#[derive(Default)]
pub struct SlideController;

impl SlideController {
    pub fn new() -> Self {
        Self
    }

    /// Push the capsule out of any overlapping geometry, resolving the deepest
    /// contact each iteration. Returns the settled feet position and the list
    /// of resolved contacts.
    fn depenetrate(
        &self,
        world: &PhysicsWorld,
        body: &CharacterBody,
        mut feet: Vec3,
    ) -> (Vec3, Vec<Contact>) {
        let mut resolved = Vec::new();
        for _ in 0..MAX_DEPEN {
            let cap = body.capsule_at(feet);
            let contacts = world.capsule_contacts(&cap);
            let deepest = contacts.iter().copied().max_by(|a, b| {
                a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal)
            });
            match deepest {
                Some(c) if c.depth > 1e-4 => {
                    feet += c.normal * (c.depth + SKIN);
                    resolved.push(c);
                }
                _ => break,
            }
        }
        (feet, resolved)
    }

    /// Move by `motion`, substepping so we can't tunnel through thin geometry,
    /// depenetrating after each substep. Sliding is implicit: the pushout
    /// cancels the into-surface component, preserving the tangential part of the
    /// motion. Returns the final feet position and every contact encountered.
    fn slide(
        &self,
        world: &PhysicsWorld,
        body: &CharacterBody,
        mut feet: Vec3,
        motion: Vec3,
    ) -> (Vec3, Vec<Contact>) {
        let dist = motion.length();
        if dist < 1e-6 {
            return self.depenetrate(world, body, feet);
        }
        let steps = (dist / MAX_SUBSTEP).ceil().max(1.0) as usize;
        let step = motion / steps as f32;
        let mut all = Vec::new();
        for _ in 0..steps {
            let (f, c) = self.depenetrate(world, body, feet + step);
            feet = f;
            all.extend(c);
        }
        (feet, all)
    }

    fn horizontal_progress(from: Vec3, to: Vec3) -> f32 {
        let d = to - from;
        Vec3::new(d.x, d.y, 0.0).length()
    }

    /// Probe just below the feet for walkable ground.
    fn ground_probe(&self, world: &PhysicsWorld, body: &CharacterBody, feet: Vec3) -> Option<Vec3> {
        let cap = body.capsule_at(feet - Vec3::Z * (SKIN + GROUND_PROBE));
        let contacts = world.capsule_contacts(&cap);
        contacts
            .iter()
            .filter(|c| c.normal.z > body.walkable_cos())
            .max_by(|a, b| a.normal.z.partial_cmp(&b.normal.z).unwrap_or(std::cmp::Ordering::Equal))
            .map(|c| c.normal)
    }
}

impl CharacterController for SlideController {
    fn move_character(
        &mut self,
        world: &PhysicsWorld,
        body: CharacterBody,
        desired_delta: Vec3,
        _dt: f32,
    ) -> CharacterMoveResult {
        let start = body.position;
        // 1. Unstick.
        let (mut feet, _) = self.depenetrate(world, &body, start);

        let horiz = Vec3::new(desired_delta.x, desired_delta.y, 0.0);
        let vert = Vec3::new(0.0, 0.0, desired_delta.z);
        let mut hit_wall = false;

        // 2. Horizontal move (with slide).
        let h_start = feet;
        let (mut h_pos, hcontacts) = if horiz.length_squared() > 1e-8 {
            self.slide(world, &body, feet, horiz)
        } else {
            (feet, Vec::new())
        };
        if hcontacts.iter().any(|c| c.normal.z.abs() < 0.5) {
            hit_wall = true;
        }

        // 3. Step-up retry if horizontal progress was blocked.
        let wanted = horiz.length();
        let progress = Self::horizontal_progress(h_start, h_pos);
        if wanted > 1e-3 && progress + 0.5 < wanted {
            let up = Vec3::Z * body.step_height;
            let (up_pos, _) = self.depenetrate(world, &body, h_start + up);
            // Only attempt if we actually gained headroom.
            if (up_pos.z - h_start.z) > body.step_height * 0.5 {
                let (fwd_pos, _) = self.slide(world, &body, up_pos, horiz);
                let (down_pos, _) =
                    self.slide(world, &body, fwd_pos, -Vec3::Z * (body.step_height + SKIN));
                if Self::horizontal_progress(h_start, down_pos) > progress + 0.25 {
                    h_pos = down_pos;
                }
            }
        }
        feet = h_pos;

        // 4. Vertical move (gravity / jump), with slide.
        let (v_pos, vcontacts) = if vert.length_squared() > 1e-8 {
            self.slide(world, &body, feet, vert)
        } else {
            (feet, Vec::new())
        };
        feet = v_pos;

        // 5. Ground detection (only when not moving upward).
        let mut grounded = false;
        let mut ground_normal = Vec3::Z;
        if desired_delta.z <= 1e-4 {
            if let Some(n) = vcontacts
                .iter()
                .filter(|c| c.normal.z > body.walkable_cos())
                .map(|c| c.normal)
                .max_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal))
            {
                grounded = true;
                ground_normal = n;
            }
            if !grounded {
                if let Some(n) = self.ground_probe(world, &body, feet) {
                    grounded = true;
                    ground_normal = n;
                }
            }
        }

        CharacterMoveResult {
            position: feet,
            grounded,
            ground_normal,
            hit_wall,
            moved: feet - start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocket3d_core::Triangle;

    fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> [Triangle; 2] {
        [Triangle::new(a, b, c), Triangle::new(a, c, d)]
    }

    fn big_floor() -> Vec<Triangle> {
        quad(
            Vec3::new(-1000.0, -1000.0, 0.0),
            Vec3::new(1000.0, -1000.0, 0.0),
            Vec3::new(1000.0, 1000.0, 0.0),
            Vec3::new(-1000.0, 1000.0, 0.0),
        )
        .to_vec()
    }

    #[test]
    fn falls_and_rests_on_floor() {
        let world = PhysicsWorld::from_triangles(big_floor());
        let mut kcc = SlideController::new();
        let body = CharacterBody {
            position: Vec3::new(0.0, 0.0, 50.0),
            ..Default::default()
        };
        let r = kcc.move_character(&world, body, Vec3::new(0.0, 0.0, -100.0), 1.0 / 60.0);
        assert!(r.position.z.abs() < 1.0, "feet rest on floor, got {}", r.position.z);
        assert!(r.grounded, "should be grounded");
        assert!(r.ground_normal.z > 0.9);
    }

    #[test]
    fn stops_at_wall_and_does_not_pass_through() {
        let mut tris = big_floor();
        tris.extend(quad(
            Vec3::new(100.0, -1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 500.0),
            Vec3::new(100.0, -1000.0, 500.0),
        ));
        let world = PhysicsWorld::from_triangles(tris);
        let mut kcc = SlideController::new();
        let body = CharacterBody {
            position: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        };
        let r = kcc.move_character(&world, body, Vec3::new(400.0, 0.0, -10.0), 1.0 / 60.0);
        assert!(
            r.position.x <= 100.0 - body.radius + 1.0,
            "should stop before wall, got x={}",
            r.position.x
        );
        assert!(r.hit_wall, "should report wall contact");
    }

    #[test]
    fn slides_along_wall() {
        let mut tris = big_floor();
        tris.extend(quad(
            Vec3::new(100.0, -1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 500.0),
            Vec3::new(100.0, -1000.0, 500.0),
        ));
        let world = PhysicsWorld::from_triangles(tris);
        let mut kcc = SlideController::new();
        let body = CharacterBody {
            position: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        };
        let r = kcc.move_character(&world, body, Vec3::new(300.0, 300.0, -10.0), 1.0 / 60.0);
        assert!(r.position.x <= 100.0, "blocked in x");
        assert!(r.position.y > 100.0, "slid substantially in y, got {}", r.position.y);
    }

    #[test]
    fn steps_over_small_ledge() {
        let mut tris = big_floor();
        // step front face (x=100, z 0..16)
        tris.extend(quad(
            Vec3::new(100.0, -1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 0.0),
            Vec3::new(100.0, 1000.0, 16.0),
            Vec3::new(100.0, -1000.0, 16.0),
        ));
        // step top (z=16, x 100..1000)
        tris.extend(quad(
            Vec3::new(100.0, -1000.0, 16.0),
            Vec3::new(1000.0, -1000.0, 16.0),
            Vec3::new(1000.0, 1000.0, 16.0),
            Vec3::new(100.0, 1000.0, 16.0),
        ));
        let world = PhysicsWorld::from_triangles(tris);
        let mut kcc = SlideController::new();
        let mut body = CharacterBody {
            position: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        };
        for _ in 0..40 {
            let r = kcc.move_character(&world, body, Vec3::new(20.0, 0.0, -13.3), 1.0 / 60.0);
            body.position = r.position;
        }
        assert!(body.position.x > 120.0, "climbed onto the step (x={})", body.position.x);
        assert!(body.position.z > 12.0, "ended up on top of the step (z={})", body.position.z);
    }
}
