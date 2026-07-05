//! `pocket3d-physics` — engine-independent, query-only static collision
//! (DESIGN.md §14). Provides a triangle BVH built from BSP collision geometry
//! plus raycasts (bullets, ground probes) and capsule-contact queries (KCC).
//!
//! This crate has no notion of rigid bodies or dynamics — characters use the
//! KCC in `pocket3d-kcc`, and bullets are raycasts.

pub mod bvh;
pub mod geometry;

pub use bvh::Bvh;
pub use geometry::{
    capsule_triangle, closest_point_on_segment, closest_point_on_triangle, ray_triangle, Contact,
};

use pocket3d_core::{Aabb, Capsule, CollisionMesh, Ray, RayHit, Triangle};

/// Immutable static collision world: a BVH over a triangle soup.
pub struct PhysicsWorld {
    bvh: Bvh,
    scratch: std::cell::RefCell<Vec<u32>>,
}

impl PhysicsWorld {
    pub fn from_triangles(tris: Vec<Triangle>) -> Self {
        Self {
            bvh: Bvh::build(tris),
            scratch: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn from_collision_mesh(mesh: &CollisionMesh) -> Self {
        let tris: Vec<Triangle> = mesh.iter_triangles().collect();
        Self::from_triangles(tris)
    }

    pub fn triangle_count(&self) -> usize {
        self.bvh.triangle_count()
    }

    pub fn bounds(&self) -> Aabb {
        self.bvh.bounds()
    }

    /// Nearest static-world hit along `ray` within `max_t` (bullets, probes).
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        self.bvh.raycast(ray, max_t)
    }

    /// All contacts between `capsule` and nearby triangles (KCC depenetration).
    pub fn capsule_contacts(&self, capsule: &Capsule) -> Vec<Contact> {
        let query = capsule.aabb();
        let mut hits = self.scratch.borrow_mut();
        self.bvh.overlap_aabb(&query, &mut hits);
        let mut contacts = Vec::new();
        for &ti in hits.iter() {
            let tri = self.bvh.triangle(ti);
            if let Some(c) = capsule_triangle(capsule.a, capsule.b, capsule.radius, tri) {
                contacts.push(c);
            }
        }
        contacts
    }

    /// Whether `capsule` overlaps any static triangle.
    pub fn capsule_overlaps(&self, capsule: &Capsule) -> bool {
        let query = capsule.aabb();
        let mut hits = self.scratch.borrow_mut();
        self.bvh.overlap_aabb(&query, &mut hits);
        hits.iter().any(|&ti| {
            capsule_triangle(capsule.a, capsule.b, capsule.radius, self.bvh.triangle(ti)).is_some()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// A large flat floor at z=0 made of two triangles.
    fn floor() -> PhysicsWorld {
        let tris = vec![
            Triangle::new(
                Vec3::new(-1000.0, -1000.0, 0.0),
                Vec3::new(1000.0, -1000.0, 0.0),
                Vec3::new(1000.0, 1000.0, 0.0),
            ),
            Triangle::new(
                Vec3::new(-1000.0, -1000.0, 0.0),
                Vec3::new(1000.0, 1000.0, 0.0),
                Vec3::new(-1000.0, 1000.0, 0.0),
            ),
        ];
        PhysicsWorld::from_triangles(tris)
    }

    #[test]
    fn ray_hits_floor_from_above() {
        let w = floor();
        let ray = Ray::new(Vec3::new(0.0, 0.0, 100.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = w.raycast(&ray, 1000.0).expect("should hit floor");
        assert!((hit.t - 100.0).abs() < 0.01);
        assert!(hit.normal.z > 0.9, "floor normal points up");
    }

    #[test]
    fn ray_misses_when_pointing_away() {
        let w = floor();
        let ray = Ray::new(Vec3::new(0.0, 0.0, 100.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(w.raycast(&ray, 1000.0).is_none());
    }

    #[test]
    fn capsule_penetrating_floor_reports_contact() {
        let w = floor();
        // Capsule feet slightly below the floor.
        let cap = Capsule::from_base_height(Vec3::new(0.0, 0.0, -5.0), 72.0, 16.0);
        let contacts = w.capsule_contacts(&cap);
        assert!(!contacts.is_empty(), "should contact floor");
        assert!(contacts.iter().any(|c| c.normal.z > 0.5 && c.depth > 0.0));
    }

    #[test]
    fn capsule_above_floor_no_contact() {
        let w = floor();
        let cap = Capsule::from_base_height(Vec3::new(0.0, 0.0, 50.0), 72.0, 16.0);
        assert!(w.capsule_contacts(&cap).is_empty());
    }

    /// Regression (review finding): even when the floor triangles are wound so
    /// their geometric normal points DOWN, a capsule straddling them from above
    /// must be pushed UP (toward its body), not driven through the floor.
    #[test]
    fn capsule_pushed_up_from_downward_wound_floor() {
        // Same floor but reversed winding -> normal is -Z.
        let tris = vec![
            Triangle::new(
                Vec3::new(-1000.0, -1000.0, 0.0),
                Vec3::new(1000.0, 1000.0, 0.0),
                Vec3::new(1000.0, -1000.0, 0.0),
            ),
            Triangle::new(
                Vec3::new(-1000.0, -1000.0, 0.0),
                Vec3::new(-1000.0, 1000.0, 0.0),
                Vec3::new(1000.0, 1000.0, 0.0),
            ),
        ];
        let w = PhysicsWorld::from_triangles(tris);
        // Straddle the plane so the contact resolves at the surface (dist~0).
        let cap = Capsule::from_base_height(Vec3::new(0.0, 0.0, -30.0), 72.0, 16.0);
        let contacts = w.capsule_contacts(&cap);
        assert!(!contacts.is_empty());
        assert!(
            contacts.iter().all(|c| c.normal.z > 0.5),
            "push-out must be upward regardless of winding: {:?}",
            contacts.iter().map(|c| c.normal).collect::<Vec<_>>()
        );
    }

    /// Regression (review finding): a straight-down ray whose origin sits
    /// exactly on a triangle/AABB boundary must still hit (no `0*inf = NaN`
    /// slab collapse). This is the KCC ground probe's exact situation.
    #[test]
    fn axis_aligned_ray_on_boundary_still_hits() {
        // A floor triangle whose min corner is exactly at x=0, y=0.
        let tris = vec![Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        )];
        let w = PhysicsWorld::from_triangles(tris);
        // Origin x==0 and y==0 coincide with the node/triangle boundary; dir is
        // axis-aligned (straight down).
        let ray = Ray::new(Vec3::new(0.0, 0.0, 50.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = w.raycast(&ray, 1000.0);
        assert!(hit.is_some(), "boundary-origin axis-aligned ray should hit");
        assert!((hit.unwrap().t - 50.0).abs() < 0.1);
    }
}
