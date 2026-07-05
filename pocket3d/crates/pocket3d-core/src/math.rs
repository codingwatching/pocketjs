//! Math conventions and the [`Transform`] type.

use glam::{Mat4, Quat, Vec3};

/// World up axis (Z-up, DESIGN.md §8).
pub const UP: Vec3 = Vec3::Z;
/// World forward axis (+Y = north).
pub const FORWARD: Vec3 = Vec3::Y;
/// World right axis (+X = east).
pub const RIGHT: Vec3 = Vec3::X;

/// A position/rotation/scale transform in Pocket3D world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// Local-to-world matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Transform a point from local to world space.
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.rotation * (p * self.scale) + self.translation
    }

    /// Rotate a direction from local to world space (ignores translation/scale).
    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        self.rotation * d
    }

    /// Compose two transforms (`self` then `parent`): `parent * self`.
    pub fn mul(&self, child: &Transform) -> Transform {
        Transform {
            translation: self.transform_point(child.translation),
            rotation: self.rotation * child.rotation,
            scale: self.scale * child.scale,
        }
    }
}

/// Convert Hammer/GoldSrc Euler `angles` `[pitch, yaw, roll]` (degrees) into a
/// quaternion in Pocket3D's Z-up convention. GoldSrc stores entity orientation
/// as `(pitch, yaw, roll)` where yaw rotates about +Z.
pub fn quat_from_goldsrc_angles(pitch_deg: f32, yaw_deg: f32, roll_deg: f32) -> Quat {
    let (p, y, r) = (
        pitch_deg.to_radians(),
        yaw_deg.to_radians(),
        roll_deg.to_radians(),
    );
    // yaw about Z, then pitch about local Y (nose down positive in GoldSrc),
    // then roll about local X.
    Quat::from_rotation_z(y) * Quat::from_rotation_y(-p) * Quat::from_rotation_x(r)
}

/// Linear interpolation for scalars.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
