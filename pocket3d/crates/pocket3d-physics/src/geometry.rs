//! Closest-point and intersection primitives used by raycasts and the KCC.
//!
//! Algorithms follow Ericson, *Real-Time Collision Detection*.

use glam::Vec3;
use pocket3d_core::{Ray, RayHit, Triangle};

const EPS: f32 = 1e-6;

/// Closest point on segment `ab` to point `p`.
pub fn closest_point_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let denom = ab.dot(ab);
    if denom < EPS {
        return a;
    }
    let t = (p - a).dot(ab) / denom;
    a + ab * t.clamp(0.0, 1.0)
}

/// Closest point on triangle `tri` to point `p` (Ericson §5.1.5).
pub fn closest_point_on_triangle(p: Vec3, tri: &Triangle) -> Vec3 {
    let (a, b, c) = (tri.a, tri.b, tri.c);
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a; // vertex region A
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b; // vertex region B
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v; // edge AB
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c; // vertex region C
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w; // edge AC
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w; // edge BC
    }
    // Inside face region.
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    a + ab * v + ac * w
}

/// Möller–Trumbore ray/triangle intersection (double-sided).
pub fn ray_triangle(ray: &Ray, tri: &Triangle, max_t: f32) -> Option<RayHit> {
    let e1 = tri.b - tri.a;
    let e2 = tri.c - tri.a;
    let pvec = ray.dir.cross(e2);
    let det = e1.dot(pvec);
    if det.abs() < EPS {
        return None; // parallel
    }
    let inv_det = 1.0 / det;
    let tvec = ray.origin - tri.a;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(e1);
    let v = ray.dir.dot(qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(qvec) * inv_det;
    if t < EPS || t > max_t {
        return None;
    }
    let mut normal = e1.cross(e2).normalize_or_zero();
    // Face the normal toward the incoming ray.
    if normal.dot(ray.dir) > 0.0 {
        normal = -normal;
    }
    Some(RayHit {
        t,
        point: ray.at(t),
        normal,
    })
}

/// Reciprocal of a ray direction where zero components map to a large finite
/// value. This keeps [`ray_aabb`]'s slab test from forming `0 * inf = NaN` for
/// axis-aligned rays (e.g. a straight-down ground probe), which would otherwise
/// collapse the interval and silently skip nodes.
pub fn safe_inv_dir(dir: Vec3) -> Vec3 {
    fn inv(x: f32) -> f32 {
        if x.abs() < 1e-8 {
            1e30_f32.copysign(x)
        } else {
            1.0 / x
        }
    }
    Vec3::new(inv(dir.x), inv(dir.y), inv(dir.z))
}

/// Slab test: does `ray` (over `[0, max_t]`) intersect AABB `[min,max]`?
/// `inv_dir` should come from [`safe_inv_dir`].
pub fn ray_aabb(origin: Vec3, inv_dir: Vec3, min: Vec3, max: Vec3, max_t: f32) -> bool {
    let t0 = (min - origin) * inv_dir;
    let t1 = (max - origin) * inv_dir;
    let tmin = t0.min(t1);
    let tmax = t0.max(t1);
    let enter = tmin.x.max(tmin.y).max(tmin.z);
    let exit = tmax.x.min(tmax.y).min(tmax.z);
    exit >= enter.max(0.0) && enter <= max_t
}

/// The result of a capsule overlapping a triangle: how to push it out.
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub point: Vec3,
    /// Unit push-out direction (from surface toward the capsule).
    pub normal: Vec3,
    /// Penetration depth (how far to move along `normal` to separate).
    pub depth: f32,
}

/// Test a capsule (segment `a`–`b`, `radius`) against a triangle. Returns a
/// contact if they overlap. Uses the standard "reference point" reduction to a
/// sphere-vs-triangle test.
pub fn capsule_triangle(a: Vec3, b: Vec3, radius: f32, tri: &Triangle) -> Option<Contact> {
    let n = tri.normal();
    let seg = b - a;
    // Find the segment point nearest the triangle plane.
    let center = if n.length_squared() < EPS {
        closest_point_on_segment(tri.centroid(), a, b)
    } else {
        let denom = n.dot(seg);
        let reference = if denom.abs() < EPS {
            // Segment parallel to plane: use its midpoint projected onto the tri.
            closest_point_on_triangle((a + b) * 0.5, tri)
        } else {
            let t = (n.dot(tri.a - a) / denom).clamp(0.0, 1.0);
            let line_point = a + seg * t;
            closest_point_on_triangle(line_point, tri)
        };
        closest_point_on_segment(reference, a, b)
    };

    // Sphere(center, radius) vs triangle.
    let cp = closest_point_on_triangle(center, tri);
    let delta = center - cp;
    let dist2 = delta.length_squared();
    if dist2 >= radius * radius {
        return None;
    }
    let dist = dist2.sqrt();
    let normal = if dist > EPS {
        // From the surface toward the capsule — correct regardless of winding
        // or which side the capsule is on.
        delta / dist
    } else {
        // The reference point lies exactly on the triangle, so `delta` gives no
        // direction. Orient the (winding-dependent) plane normal toward the
        // capsule's body so we push it OUT rather than through the surface.
        let side = n.dot((a + b) * 0.5 - cp);
        if side.abs() > EPS {
            n * side.signum()
        } else {
            n
        }
    };
    Some(Contact {
        point: cp,
        normal,
        depth: radius - dist,
    })
}
