//! `pocket3d-app` — the application lifecycle contract (DESIGN.md §6).
//!
//! An application implements [`Pocket3dApp`]. The runtime (windowed via
//! `pocket3d-render-wgpu`, or headless in tests/CLI) drives the fixed-timestep
//! loop and calls the four hooks, handing each a context that exposes exactly
//! the subsystems that phase may touch.

use pocket3d_core::{EventQueue, InputSnapshot, TickInfo, World};
use pocket3d_render::{RenderDevice, SceneView};

/// The application lifecycle. Simulation state is authoritative and lives in
/// the app; render state is derived each frame in [`Pocket3dApp::render`].
pub trait Pocket3dApp {
    /// One-time setup: load assets, upload GPU resources, spawn initial world.
    fn init(&mut self, ctx: &mut AppInitContext<'_>) -> anyhow::Result<()>;

    /// Fixed-rate simulation step (default 60 Hz). Authoritative.
    fn fixed_update(&mut self, ctx: &mut FixedUpdateContext<'_>);

    /// Variable-rate per-frame update (input polish, interpolation prep).
    fn update(&mut self, ctx: &mut FrameUpdateContext<'_>);

    /// Build the frame's [`SceneView`] from current simulation state.
    fn render(&mut self, ctx: &mut RenderContext<'_>);
}

/// Handed to [`Pocket3dApp::init`]. Exposes the render device for resource
/// creation and the world for initial population.
pub struct AppInitContext<'a> {
    pub device: &'a mut dyn RenderDevice,
    pub world: &'a mut World,
}

/// Handed to [`Pocket3dApp::fixed_update`]. This is the authoritative step.
pub struct FixedUpdateContext<'a> {
    pub world: &'a mut World,
    pub input: &'a InputSnapshot,
    pub tick: TickInfo,
    /// Simulation events (hits, kills, round transitions) drained by the host.
    pub events: &'a mut EventQueue,
}

/// Handed to [`Pocket3dApp::update`] once per rendered frame.
pub struct FrameUpdateContext<'a> {
    pub world: &'a mut World,
    pub input: &'a InputSnapshot,
    /// Real seconds since the previous frame.
    pub dt: f32,
    /// Interpolation factor in `[0,1)` between the last two fixed ticks.
    pub alpha: f32,
}

/// Handed to [`Pocket3dApp::render`]. The app fills `scene` with draw packets.
pub struct RenderContext<'a> {
    pub scene: &'a mut SceneView,
    pub alpha: f32,
}
