//! Contract tests for the shared core types.

use pocket3d_core::{
    math::quat_from_goldsrc_angles, Aabb, Camera, Capsule, FixedClock, Ray, Transform,
};
use pocket3d_core::glam::Vec3;

#[test]
fn camera_forward_is_x_at_zero_yaw() {
    let cam = Camera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    };
    let f = cam.forward();
    assert!((f - Vec3::X).length() < 1e-5, "forward should be +X, got {f:?}");
    // Right of +X forward (Z-up) is -Y.
    let r = cam.right();
    assert!((r - (-Vec3::Y)).length() < 1e-5, "right should be -Y, got {r:?}");
}

#[test]
fn camera_yaw_90_faces_y() {
    let cam = Camera {
        yaw: std::f32::consts::FRAC_PI_2,
        ..Default::default()
    };
    let f = cam.forward();
    assert!((f - Vec3::Y).length() < 1e-5, "yaw 90 faces +Y, got {f:?}");
}

#[test]
fn fixed_clock_runs_expected_ticks() {
    let mut clk = FixedClock::new(60.0);
    // 1/60 s should yield exactly one tick.
    assert_eq!(clk.advance(1.0 / 60.0).len(), 1);
    // Half a tick: none yet.
    assert_eq!(clk.advance(1.0 / 120.0).len(), 0);
    // Accumulator now at half a tick -> alpha ~ 0.5.
    assert!((clk.alpha() - 0.5).abs() < 0.05);
    // A big frame is clamped so we never spiral.
    assert!(clk.advance(10.0).len() <= 8);
}

#[test]
fn transform_round_trips_point() {
    let t = Transform::from_translation_rotation(
        Vec3::new(10.0, 0.0, 0.0),
        quat_from_goldsrc_angles(0.0, 90.0, 0.0),
    );
    let p = t.transform_point(Vec3::new(1.0, 0.0, 0.0));
    // Yaw 90 about Z sends +X to +Y, then translate.
    assert!((p - Vec3::new(10.0, 1.0, 0.0)).length() < 1e-4, "got {p:?}");
}

#[test]
fn aabb_grows_and_contains() {
    let b = Aabb::from_points([Vec3::ZERO, Vec3::splat(10.0)]);
    assert!(b.contains(Vec3::splat(5.0)));
    assert!(!b.contains(Vec3::splat(11.0)));
    assert_eq!(b.center(), Vec3::splat(5.0));
}

#[test]
fn capsule_from_base_height_insets_caps() {
    let c = Capsule::from_base_height(Vec3::ZERO, 72.0, 16.0);
    assert_eq!(c.a, Vec3::new(0.0, 0.0, 16.0));
    assert_eq!(c.b, Vec3::new(0.0, 0.0, 56.0));
    let bb = c.aabb();
    assert!((bb.min.z - 0.0).abs() < 1e-5, "bottom cap reaches the feet");
    assert!((bb.max.z - 72.0).abs() < 1e-5, "top cap reaches the crown");
}

#[test]
fn ray_at_advances_along_dir() {
    let r = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0));
    assert!((r.at(3.0) - Vec3::new(0.0, 0.0, 3.0)).length() < 1e-5);
}
