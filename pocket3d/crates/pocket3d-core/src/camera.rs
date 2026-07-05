//! First-person / free-fly camera (DESIGN.md §13).
//!
//! The camera lives in Pocket3D's Z-up world space and is responsible for
//! producing the view/projection matrices. Mapping into `wgpu` clip-space
//! conventions (reverse-Y is not used; depth is `[0,1]`) happens here via
//! `glam`'s `perspective_rh`, which targets Vulkan/Metal/D3D/WebGPU depth.

use glam::{Mat4, Vec3};

/// A camera positioned by an eye point and yaw/pitch angles.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: Vec3,
    /// Yaw in radians, measured in the XY plane from +X toward +Y.
    pub yaw: f32,
    /// Pitch in radians, positive tilts the view up toward +Z.
    pub pitch: f32,
    /// Vertical field of view in degrees.
    pub fov_y_deg: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov_y_deg: 80.0,
            near: 0.03,
            far: 8192.0,
        }
    }
}

impl Camera {
    /// Clamp pitch to just under straight up/down to avoid gimbal flip.
    pub const PITCH_LIMIT: f32 = 1.5533; // ~89 degrees

    pub fn clamp_pitch(&mut self) {
        self.pitch = self.pitch.clamp(-Self::PITCH_LIMIT, Self::PITCH_LIMIT);
    }

    /// World-space forward direction from yaw/pitch (Z-up).
    pub fn forward(&self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(cp * cy, cp * sy, sp)
    }

    /// World-space right direction (horizontal).
    pub fn right(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(sy, -cy, 0.0)
    }

    /// Horizontal forward direction (movement plane, Z component zeroed).
    pub fn forward_horizontal(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(cy, sy, 0.0)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.eye, self.forward(), Vec3::Z)
    }

    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y_deg.to_radians(), aspect.max(0.0001), self.near, self.far)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect) * self.view_matrix()
    }
}
