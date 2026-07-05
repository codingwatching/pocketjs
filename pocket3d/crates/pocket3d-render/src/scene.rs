//! The per-frame scene view the application submits to the renderer.

use crate::debug::DebugDraw;
use glam::Mat4;
use pocket3d_core::{
    Camera, MaterialHandle, MeshHandle, SkinnedMeshHandle, WorldHandle,
};

/// A static-mesh draw instance (props, decals-as-quads, weapon world model).
#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Mat4,
}

/// A skinned-actor draw instance with its evaluated joint palette.
#[derive(Clone, Debug)]
pub struct SkinnedInstance {
    pub mesh: SkinnedMeshHandle,
    pub material: MaterialHandle,
    pub transform: Mat4,
    /// `skeleton.joint_count` skinning matrices (`joint_world * inverse_bind`).
    pub joint_matrices: Vec<Mat4>,
}

/// The first-person weapon, drawn last with its own FOV and depth handling.
#[derive(Clone, Debug)]
pub struct ViewmodelInstance {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Mat4,
    pub fov_y_deg: f32,
}

/// Minimal HUD state (DESIGN.md §21). The renderer draws these with a bitmap
/// font + textured quads; no layout engine.
#[derive(Clone, Debug)]
pub struct Hud {
    pub show_crosshair: bool,
    pub health: i32,
    /// `None` renders an infinite-ammo marker.
    pub ammo: Option<i32>,
    pub round_text: Option<String>,
    pub show_debug: bool,
    /// Free-form debug lines drawn in the corner when `show_debug` is set.
    pub debug_lines: Vec<String>,
}

impl Default for Hud {
    fn default() -> Self {
        Self {
            show_crosshair: true,
            health: 100,
            ammo: None,
            round_text: None,
            show_debug: false,
            debug_lines: Vec::new(),
        }
    }
}

/// Everything the renderer needs to draw one frame. Built fresh each frame from
/// simulation state (the renderer owns no gameplay state, DESIGN.md §6).
#[derive(Clone, Debug)]
pub struct SceneView {
    pub camera: Camera,
    pub aspect: f32,
    /// The compiled BSP world to draw, if any.
    pub world: Option<WorldHandle>,
    /// Opaque static meshes.
    pub meshes: Vec<MeshInstance>,
    /// Skinned actors (bots).
    pub skinned: Vec<SkinnedInstance>,
    /// Translucent meshes (water/glass), drawn after opaque.
    pub translucent: Vec<MeshInstance>,
    /// First-person weapon.
    pub viewmodel: Option<ViewmodelInstance>,
    pub debug: DebugDraw,
    pub hud: Hud,
}

impl SceneView {
    pub fn new(camera: Camera, aspect: f32) -> Self {
        Self {
            camera,
            aspect,
            world: None,
            meshes: Vec::new(),
            skinned: Vec::new(),
            translucent: Vec::new(),
            viewmodel: None,
            debug: DebugDraw::default(),
            hud: Hud::default(),
        }
    }

    /// Clear the per-frame lists, preserving camera/world/hud config so the
    /// caller can rebuild draw packets each frame cheaply.
    pub fn begin_frame(&mut self) {
        self.meshes.clear();
        self.skinned.clear();
        self.translucent.clear();
        self.viewmodel = None;
        self.debug.clear();
    }
}
