//! `pocket3d-render` — renderer-facing data structures and the `RenderDevice`
//! contract (DESIGN.md §12).
//!
//! This crate is backend-agnostic. It defines *what* the application submits
//! each frame (a [`SceneView`] of draw packets + a [`DebugDraw`] buffer + a
//! [`Hud`]) and *how* it registers GPU resources ([`RenderDevice`]), without
//! referencing `wgpu`. `pocket3d-render-wgpu` implements the backend.

pub mod debug;
pub mod device;
pub mod scene;

pub use debug::DebugDraw;
pub use device::{RenderDevice, WorldUpload};
pub use scene::{Hud, MeshInstance, SceneView, SkinnedInstance, ViewmodelInstance};

// Re-export the material model so `use pocket3d_render::MaterialKind` works, as
// the design (§12) locates it in the renderer.
pub use pocket3d_core::material::{MaterialDesc, MaterialKind};

/// The ordered render passes (DESIGN.md §12). Backends should execute these in
/// order; this enum documents the contract and lets tools/tests reason about it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderPass {
    WorldOpaque,
    SkinnedActor,
    AlphaTranslucent,
    Viewmodel,
    Debug,
    Hud,
}

impl RenderPass {
    pub const ORDER: [RenderPass; 6] = [
        RenderPass::WorldOpaque,
        RenderPass::SkinnedActor,
        RenderPass::AlphaTranslucent,
        RenderPass::Viewmodel,
        RenderPass::Debug,
        RenderPass::Hud,
    ];
}

/// Per-frame renderer statistics for the debug overlay.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub draw_calls: u32,
    pub triangles: u32,
    pub world_triangles: u32,
    pub skinned_triangles: u32,
    pub debug_lines: u32,
}
