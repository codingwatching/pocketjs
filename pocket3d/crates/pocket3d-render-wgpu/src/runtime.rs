//! Windowed runtime entry point (DESIGN.md §6, §15).
//!
//! [`run_app`] opens a `winit` window, brings up `wgpu`, and drives the
//! fixed-timestep application loop: it samples input into a
//! [`pocket3d_core::InputSnapshot`], runs `fixed_update` for each accumulated
//! 60 Hz tick, runs `update` once, then builds a [`SceneView`] via `render` and
//! submits it through [`WgpuRenderer`].
//!
//! This module is display-gated: it needs a real window and GPU, so it is only
//! meant to run on a developer machine (not headless CI).

use std::sync::Arc;
use std::time::Instant;

use pocket3d_app::{
    AppInitContext, FixedUpdateContext, FrameUpdateContext, Pocket3dApp, RenderContext,
};
use pocket3d_core::{
    time::FixedClock, Button, Camera, EventQueue, InputSnapshot, Key, World,
};
use pocket3d_render::SceneView;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::renderer::WgpuRenderer;

/// Simulation tick rate: 60 Hz fixed timestep (DESIGN.md §6).
const TICK_HZ: f32 = 60.0;

/// Configuration for the windowed runtime.
#[derive(Clone, Debug)]
pub struct RunConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            title: "Pocket3D".to_string(),
            width: 1280,
            height: 720,
        }
    }
}

/// Open a window and run `app` through the fixed-timestep loop until the window
/// closes (DESIGN.md §6). Uses `winit` 0.30's [`ApplicationHandler`] event loop.
///
/// This cannot be exercised headlessly, but it compiles and links against the
/// same `RenderDevice` contract the headless path uses.
pub fn run_app<A: Pocket3dApp + 'static>(app: A, config: RunConfig) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut state = AppState::new(app, config);
    event_loop.run_app(&mut state)?;
    Ok(())
}

/// Holds everything the event loop needs across callbacks. The window, surface,
/// and renderer are created lazily in [`ApplicationHandler::resumed`].
struct AppState<A: Pocket3dApp> {
    app: A,
    config: RunConfig,
    world: World,
    renderer: Option<WgpuRenderer>,
    window: Option<Arc<Window>>,
    clock: FixedClock,
    input: InputSnapshot,
    events: EventQueue,
    last_frame: Option<Instant>,
    initialized: bool,
}

impl<A: Pocket3dApp> AppState<A> {
    fn new(app: A, config: RunConfig) -> Self {
        Self {
            app,
            config,
            world: World::new(),
            renderer: None,
            window: None,
            clock: FixedClock::new(TICK_HZ),
            input: InputSnapshot::new(),
            events: EventQueue::new(),
            last_frame: None,
            initialized: false,
        }
    }

    /// Create the window + wgpu context and call `app.init` (once).
    fn bootstrap(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("failed to create window"),
        );

        // FPS-style look: grab and hide the cursor (best effort).
        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);

        // --- wgpu bring-up (DESIGN.md §12 backend) ------------------------
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface: wgpu::Surface<'static> = instance
            .create_surface(window.clone())
            .expect("failed to create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("p3d.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("failed to create device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        self.renderer = Some(WgpuRenderer::new(device, queue, surface, surface_config));
        self.window = Some(window);

        // One-time application setup with access to the render device + world.
        {
            let renderer = self.renderer.as_mut().unwrap();
            let mut ctx = AppInitContext {
                device: renderer,
                world: &mut self.world,
            };
            if let Err(e) = self.app.init(&mut ctx) {
                log::error!("app init failed: {e}");
            }
        }

        self.initialized = true;
        self.last_frame = Some(Instant::now());
    }

    /// Run one rendered frame: advance the fixed clock, step simulation, build
    /// and submit the scene (DESIGN.md §6).
    fn frame(&mut self) {
        if !self.initialized {
            return;
        }

        let now = Instant::now();
        let dt = self
            .last_frame
            .map(|t| (now - t).as_secs_f32())
            .unwrap_or(0.0);
        self.last_frame = Some(now);

        // Fixed-rate simulation (authoritative).
        let ticks = self.clock.advance(dt);
        for tick in ticks {
            let mut ctx = FixedUpdateContext {
                world: &mut self.world,
                input: &self.input,
                tick,
                events: &mut self.events,
            };
            self.app.fixed_update(&mut ctx);
        }
        // The windowed host doesn't consume events itself yet; drain to keep
        // the queue bounded.
        let _ = self.events.drain();

        let alpha = self.clock.alpha();

        // Variable-rate per-frame update.
        {
            let mut ctx = FrameUpdateContext {
                world: &mut self.world,
                input: &self.input,
                dt,
                alpha,
            };
            self.app.update(&mut ctx);
        }

        // Build the scene view and render it.
        let (w, h) = self
            .renderer
            .as_ref()
            .map(|r| r.surface_size())
            .unwrap_or((1, 1));
        let aspect = w as f32 / h.max(1) as f32;
        let mut scene = SceneView::new(Camera::default(), aspect);
        {
            let mut ctx = RenderContext {
                scene: &mut scene,
                alpha,
            };
            self.app.render(&mut ctx);
        }
        scene.aspect = aspect;

        if let Some(renderer) = self.renderer.as_mut() {
            if let Err(e) = renderer.render(&scene) {
                log::warn!("render error: {e}");
            }
        }

        // Roll input state for next frame's edge detection + reset mouse delta.
        self.input.begin_frame();
    }
}

impl<A: Pocket3dApp> ApplicationHandler for AppState<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.bootstrap(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if let Some(k) = map_key(code) {
                        self.input.set_key(k, pressed);
                    }
                    // Release the cursor on Escape so the user can leave.
                    if code == KeyCode::Escape && pressed {
                        if let Some(w) = &self.window {
                            let _ = w.set_cursor_grab(CursorGrabMode::None);
                            w.set_cursor_visible(true);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(b) = map_button(button) {
                    self.input.set_button(b, state == ElementState::Pressed);
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        // Raw mouse motion drives FPS look (DESIGN.md §15).
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.add_mouse_delta(delta.0 as f32, delta.1 as f32);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Continuously drive frames while the loop is in `Poll` mode.
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

/// Map a physical key code to the engine's logical [`Key`] (DESIGN.md §15).
fn map_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::KeyW => Key::W,
        KeyCode::KeyA => Key::A,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyD => Key::D,
        KeyCode::Space => Key::Space,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::Ctrl,
        KeyCode::KeyR => Key::R,
        KeyCode::Escape => Key::Escape,
        KeyCode::F1 => Key::F1,
        KeyCode::F3 => Key::F3,
        _ => return None,
    })
}

/// Map a `winit` mouse button to the engine's [`Button`].
fn map_button(button: MouseButton) -> Option<Button> {
    Some(match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
        _ => return None,
    })
}
