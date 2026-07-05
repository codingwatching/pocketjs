//! `pocket3d-render-wgpu` — the first render backend (DESIGN.md §12).
//!
//! This crate implements the backend-agnostic [`pocket3d_render::RenderDevice`]
//! contract on top of `wgpu`, and provides a windowed runtime ([`run_app`]) on
//! top of `winit`. It is forward-rendered and intentionally simple: no PBR, no
//! dynamic shadows (DESIGN.md §12).
//!
//! Per the design, **applications never see `wgpu` types**. They upload
//! resources and receive the opaque `u32` handles from
//! [`pocket3d_core::handles`], and submit a [`pocket3d_render::SceneView`] each
//! frame. All `wgpu`/`winit` details stay inside this crate.
//!
//! ```no_run
//! use pocket3d_render_wgpu::{run_app, RunConfig};
//! # struct Game;
//! # impl pocket3d_app::Pocket3dApp for Game {
//! #     fn init(&mut self, _: &mut pocket3d_app::AppInitContext<'_>) -> anyhow::Result<()> { Ok(()) }
//! #     fn fixed_update(&mut self, _: &mut pocket3d_app::FixedUpdateContext<'_>) {}
//! #     fn update(&mut self, _: &mut pocket3d_app::FrameUpdateContext<'_>) {}
//! #     fn render(&mut self, _: &mut pocket3d_app::RenderContext<'_>) {}
//! # }
//! # fn main() -> anyhow::Result<()> {
//! run_app(Game, RunConfig::default())
//! # }
//! ```

// Several `wgpu` resource handles are retained purely to own their GPU
// lifetime and are never read back; keep the crate free of noise about them.
#![allow(dead_code)]

mod renderer;
mod runtime;
pub(crate) mod shaders;

pub use renderer::WgpuRenderer;
pub use runtime::{run_app, RunConfig};

#[cfg(test)]
mod tests {
    use super::*;

    /// The default window configuration is sane.
    #[test]
    fn run_config_default_is_reasonable() {
        let c = RunConfig::default();
        assert!(c.width > 0 && c.height > 0);
        assert!(!c.title.is_empty());
    }

    /// Every inline WGSL shader constant carries its expected entry points
    /// (they are compiled at runtime, so a blank one would be a silent bug).
    #[test]
    fn shader_sources_are_present() {
        assert!(shaders::WORLD_WGSL.contains("fs_world"));
        assert!(shaders::MESH_WGSL.contains("vs_mesh"));
        assert!(shaders::SKINNED_WGSL.contains("vs_skinned"));
        assert!(shaders::DEBUG_WGSL.contains("vs_debug"));
        assert!(shaders::HUD_WGSL.contains("vs_hud"));
    }
}
