//! The `wgpu` render backend (DESIGN.md §12).
//!
//! [`WgpuRenderer`] owns the `wgpu` `Device`/`Queue`/surface configuration plus
//! `Vec`-indexed resource arenas keyed by the `u32` handles from
//! [`pocket3d_core::handles`]. It implements [`pocket3d_render::RenderDevice`]
//! for resource upload and draws a [`pocket3d_render::SceneView`] through the
//! ordered passes of [`pocket3d_render::RenderPass`].
//!
//! Per the design, **applications never see `wgpu` types**: every `wgpu`
//! handle stays private to this module; callers hold only the opaque
//! `pocket3d_core` handles.

use bytemuck::{Pod, Zeroable};
use pocket3d_core::{
    material::{MaterialDesc, MaterialKind},
    mesh::{MeshData, SkinnedVertex, StaticVertex, WorldVertex},
    texture::TextureData,
    Camera, Mat4, MaterialHandle, MeshHandle, SkinnedMeshHandle, TextureHandle, WorldHandle,
};
use pocket3d_render::{device::WorldUpload, RenderDevice, SceneView};
use wgpu::util::DeviceExt;

use crate::shaders;

/// Depth buffer format used by every 3D pass (DESIGN.md §12).
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Fixed scene exposure (`final_rgb *= exposure`). Simple tone control, no HDR.
const EXPOSURE: f32 = 1.0;
/// Ambient light floor for the lit mesh/skinned passes.
const AMBIENT: f32 = 0.35;
/// A single hard-coded directional light for actors (points down and forward).
const LIGHT_DIR: [f32; 4] = [-0.3, -0.4, -0.85, 0.0];

// ---------------------------------------------------------------------------
// GPU-facing POD types
// ---------------------------------------------------------------------------

/// Per-pass frame uniform. Layout mirrors the `Frame` struct in every shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    light_dir: [f32; 4],
    /// `x = ambient`, `y = exposure`, `zw` unused.
    params: [f32; 4],
}

/// Per-instance model matrix uniform (static/skinned/viewmodel meshes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceUniform {
    model: [[f32; 4]; 4],
}

/// Debug line vertex: world-space position + RGBA color.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DebugVertex {
    pos: [f32; 3],
    color: [f32; 4],
}

/// HUD vertex: NDC position + RGBA color.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HudVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

// Vertex attribute layouts, kept as `'static` consts so `VertexBufferLayout`s
// can borrow them when building pipelines.
const WORLD_ATTRS: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2];
const STATIC_ATTRS: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
const SKINNED_ATTRS: [wgpu::VertexAttribute; 5] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32x4, 4 => Float32x4];
const DEBUG_ATTRS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
const HUD_ATTRS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

// ---------------------------------------------------------------------------
// Resource-arena entries. The `wgpu` handles here are retained for their GPU
// lifetime; some are never read back after creation (bind groups keep their
// own strong references), hence the `dead_code` allowances below.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[allow(dead_code)]
struct GpuMaterial {
    kind: MaterialKind,
    base: TextureHandle,
    /// `mesh_material_bgl` bind group: base texture + sampler.
    bind_group: wgpu::BindGroup,
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// One material-batched draw range of a compiled world.
struct WorldBatch {
    index_start: u32,
    index_count: u32,
    kind: MaterialKind,
    /// `world_material_bgl` bind group: base texture + lightmap + sampler.
    material_bg: wgpu::BindGroup,
}

#[allow(dead_code)]
struct GpuWorld {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    batches: Vec<WorldBatch>,
    /// Base textures owned by this world (kept alive for the bind groups).
    textures: Vec<GpuTexture>,
    /// The single lightmap atlas for this world (DESIGN.md §12).
    lightmap: GpuTexture,
}

// A prepared per-instance draw, built fresh each frame before the render pass
// (its bind groups must outlive the pass that references them).
struct MeshDraw {
    mesh: MeshHandle,
    material: MaterialHandle,
    instance_bg: wgpu::BindGroup,
}

struct SkinDraw {
    mesh: SkinnedMeshHandle,
    material: MaterialHandle,
    skin_bg: wgpu::BindGroup,
}

// ---------------------------------------------------------------------------
// WgpuRenderer
// ---------------------------------------------------------------------------

/// The `wgpu` renderer: owns the device/queue/surface plus every pipeline,
/// bind-group layout, and resource arena (DESIGN.md §12).
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,

    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    sampler: wgpu::Sampler,
    white_texture: GpuTexture,

    // Shared frame uniforms (main camera + narrow-FOV viewmodel camera).
    frame_uniform_buffer: wgpu::Buffer,
    viewmodel_uniform_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    viewmodel_frame_bind_group: wgpu::BindGroup,

    // Bind-group layouts retained to build runtime bind groups.
    #[allow(dead_code)]
    frame_bgl: wgpu::BindGroupLayout,
    world_material_bgl: wgpu::BindGroupLayout,
    mesh_material_bgl: wgpu::BindGroupLayout,
    instance_bgl: wgpu::BindGroupLayout,
    skin_bgl: wgpu::BindGroupLayout,

    // Pipelines, one per pass variant.
    world_opaque_pipeline: wgpu::RenderPipeline,
    world_water_pipeline: wgpu::RenderPipeline,
    world_alpha_test_pipeline: wgpu::RenderPipeline,
    mesh_opaque_pipeline: wgpu::RenderPipeline,
    mesh_translucent_pipeline: wgpu::RenderPipeline,
    skinned_pipeline: wgpu::RenderPipeline,
    debug_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,

    // Resource arenas indexed by the `u32` inside each handle.
    textures: Vec<GpuTexture>,
    materials: Vec<GpuMaterial>,
    static_meshes: Vec<GpuMesh>,
    skinned_meshes: Vec<GpuMesh>,
    worlds: Vec<GpuWorld>,
}

impl WgpuRenderer {
    /// Build the renderer from an already-configured surface. Creates every
    /// pipeline, bind-group layout, sampler, and the depth buffer.
    ///
    /// `run_app` (see `crate::runtime`) is responsible for creating the
    /// `Instance`/`Adapter`/`Device`/`Queue` and configuring `surface`.
    ///
    /// Scoped to the crate so no public signature ever mentions a `wgpu` type —
    /// applications reach the renderer only through the wgpu-free
    /// [`pocket3d_render::RenderDevice`] trait and [`crate::run_app`]
    /// (DESIGN.md §12).
    pub(crate) fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let format = config.format;

        // --- Bind-group layouts -------------------------------------------
        let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("p3d.frame-bgl"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });

        let world_material_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("p3d.world-material-bgl"),
                entries: &[
                    texture_entry(0),
                    texture_entry(1),
                    sampler_entry(2),
                ],
            });

        let mesh_material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("p3d.mesh-material-bgl"),
            entries: &[texture_entry(0), sampler_entry(1)],
        });

        let instance_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("p3d.instance-bgl"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX)],
        });

        let skin_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("p3d.skin-bgl"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::VERTEX),
                uniform_entry(1, wgpu::ShaderStages::VERTEX),
            ],
        });

        // --- Pipeline layouts ---------------------------------------------
        let world_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p3d.world-pll"),
            bind_group_layouts: &[&frame_bgl, &world_material_bgl],
            push_constant_ranges: &[],
        });
        let mesh_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p3d.mesh-pll"),
            bind_group_layouts: &[&frame_bgl, &mesh_material_bgl, &instance_bgl],
            push_constant_ranges: &[],
        });
        let skinned_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p3d.skinned-pll"),
            bind_group_layouts: &[&frame_bgl, &mesh_material_bgl, &skin_bgl],
            push_constant_ranges: &[],
        });
        let debug_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p3d.debug-pll"),
            bind_group_layouts: &[&frame_bgl],
            push_constant_ranges: &[],
        });
        let hud_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p3d.hud-pll"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        // --- Shader modules -----------------------------------------------
        let world_shader = wgsl(&device, "p3d.world", shaders::WORLD_WGSL);
        let mesh_shader = wgsl(&device, "p3d.mesh", shaders::MESH_WGSL);
        let skinned_shader = wgsl(&device, "p3d.skinned", shaders::SKINNED_WGSL);
        let debug_shader = wgsl(&device, "p3d.debug", shaders::DEBUG_WGSL);
        let hud_shader = wgsl(&device, "p3d.hud", shaders::HUD_WGSL);

        // --- Vertex buffer layouts ----------------------------------------
        let world_vbl = [vbl::<WorldVertex>(&WORLD_ATTRS)];
        let static_vbl = [vbl::<StaticVertex>(&STATIC_ATTRS)];
        let skinned_vbl = [vbl::<SkinnedVertex>(&SKINNED_ATTRS)];
        let debug_vbl = [vbl::<DebugVertex>(&DEBUG_ATTRS)];
        let hud_vbl = [vbl::<HudVertex>(&HUD_ATTRS)];

        // --- Pipelines ----------------------------------------------------
        let world_opaque_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.world-opaque",
                layout: &world_pll,
                shader: &world_shader,
                vs: "vs_world",
                fs: "fs_world",
                buffers: &world_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let world_water_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.world-water",
                layout: &world_pll,
                shader: &world_shader,
                vs: "vs_world",
                fs: "fs_world",
                buffers: &world_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                depth_write: false,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let world_alpha_test_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.world-alpha-test",
                layout: &world_pll,
                shader: &world_shader,
                vs: "vs_world",
                fs: "fs_world_alpha",
                buffers: &world_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let mesh_opaque_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.mesh-opaque",
                layout: &mesh_pll,
                shader: &mesh_shader,
                vs: "vs_mesh",
                fs: "fs_mesh",
                buffers: &static_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let mesh_translucent_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.mesh-translucent",
                layout: &mesh_pll,
                shader: &mesh_shader,
                vs: "vs_mesh",
                fs: "fs_mesh",
                buffers: &static_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                depth_write: false,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let skinned_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.skinned",
                layout: &skinned_pll,
                shader: &skinned_shader,
                vs: "vs_skinned",
                fs: "fs_skinned",
                buffers: &skinned_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                cull: None,
            },
        );
        let debug_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.debug",
                layout: &debug_pll,
                shader: &debug_shader,
                vs: "vs_debug",
                fs: "fs_debug",
                buffers: &debug_vbl,
                format,
                topology: wgpu::PrimitiveTopology::LineList,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                // Debug lines are an overlay: always visible, never occlude.
                depth_write: false,
                depth_compare: wgpu::CompareFunction::Always,
                cull: None,
            },
        );
        let hud_pipeline = make_pipeline(
            &device,
            PipelineCfg {
                label: "p3d.hud",
                layout: &hud_pll,
                shader: &hud_shader,
                vs: "vs_hud",
                fs: "fs_hud",
                buffers: &hud_vbl,
                format,
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                depth_write: false,
                depth_compare: wgpu::CompareFunction::Always,
                cull: None,
            },
        );

        // --- Sampler + shared uniforms ------------------------------------
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("p3d.sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let frame_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p3d.frame-uniform"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let viewmodel_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p3d.viewmodel-uniform"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p3d.frame-bg"),
            layout: &frame_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniform_buffer.as_entire_binding(),
            }],
        });
        let viewmodel_frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p3d.viewmodel-frame-bg"),
            layout: &frame_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewmodel_uniform_buffer.as_entire_binding(),
            }],
        });

        // --- Fallback white texture + depth buffer ------------------------
        let white_texture =
            upload_texture_data(&device, &queue, &TextureData::solid(255, 255, 255, 255));
        let (depth_texture, depth_view) = create_depth(&device, config.width, config.height);

        Self {
            device,
            queue,
            surface,
            config,
            depth_texture,
            depth_view,
            sampler,
            white_texture,
            frame_uniform_buffer,
            viewmodel_uniform_buffer,
            frame_bind_group,
            viewmodel_frame_bind_group,
            frame_bgl,
            world_material_bgl,
            mesh_material_bgl,
            instance_bgl,
            skin_bgl,
            world_opaque_pipeline,
            world_water_pipeline,
            world_alpha_test_pipeline,
            mesh_opaque_pipeline,
            mesh_translucent_pipeline,
            skinned_pipeline,
            debug_pipeline,
            hud_pipeline,
            textures: Vec::new(),
            materials: Vec::new(),
            static_meshes: Vec::new(),
            skinned_meshes: Vec::new(),
            worlds: Vec::new(),
        }
    }

    /// Current swapchain size in pixels.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Reconfigure the surface (and depth buffer) after a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (tex, view) = create_depth(&self.device, width, height);
        self.depth_texture = tex;
        self.depth_view = view;
    }

    /// Reconfigure using the current config, e.g. after a lost/outdated surface.
    fn reconfigure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Build a per-instance model-matrix bind group (mesh/viewmodel).
    fn make_instance_bg(&self, model: Mat4) -> wgpu::BindGroup {
        let buf = make_buffer(
            &self.device,
            "p3d.instance",
            &[InstanceUniform {
                model: model.to_cols_array_2d(),
            }],
            wgpu::BufferUsages::UNIFORM,
        );
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p3d.instance-bg"),
            layout: &self.instance_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    }

    /// Build a skinning bind group: a joint-matrix storage buffer plus the
    /// actor's model matrix.
    fn make_skin_bg(&self, joints: &[Mat4], model: Mat4) -> wgpu::BindGroup {
        let identity = [Mat4::IDENTITY];
        let jslice: &[Mat4] = if joints.is_empty() { &identity } else { joints };
        let jbuf = make_buffer(&self.device, "p3d.joints", jslice, wgpu::BufferUsages::STORAGE);
        let mbuf = make_buffer(
            &self.device,
            "p3d.skin-instance",
            &[InstanceUniform {
                model: model.to_cols_array_2d(),
            }],
            wgpu::BufferUsages::UNIFORM,
        );
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p3d.skin-bg"),
            layout: &self.skin_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: jbuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mbuf.as_entire_binding(),
                },
            ],
        })
    }

    /// Draw one frame of `scene` through the passes of
    /// [`pocket3d_render::RenderPass`] (DESIGN.md §12).
    pub fn render(&mut self, scene: &SceneView) -> anyhow::Result<()> {
        let aspect = if scene.aspect > 0.0 {
            scene.aspect
        } else {
            self.config.width as f32 / self.config.height.max(1) as f32
        };

        // --- Update the two frame uniforms --------------------------------
        let base = FrameUniform {
            view_proj: scene.camera.view_proj(aspect).to_cols_array_2d(),
            camera_pos: scene.camera.eye.extend(1.0).to_array(),
            light_dir: LIGHT_DIR,
            params: [AMBIENT, EXPOSURE, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.frame_uniform_buffer, 0, bytemuck::bytes_of(&base));

        // Viewmodel uses the same eye but a separate (narrower) FOV so the
        // weapon never clips into the world (DESIGN.md §12 "Viewmodel pass").
        let vm_uniform = if let Some(vm) = &scene.viewmodel {
            let vm_cam = Camera {
                fov_y_deg: vm.fov_y_deg,
                ..scene.camera
            };
            FrameUniform {
                view_proj: vm_cam.view_proj(aspect).to_cols_array_2d(),
                ..base
            }
        } else {
            base
        };
        self.queue.write_buffer(
            &self.viewmodel_uniform_buffer,
            0,
            bytemuck::bytes_of(&vm_uniform),
        );

        // --- Prepare per-frame transient resources ------------------------
        // These bind groups/buffers must outlive the render passes below, so
        // they are collected up front and dropped after submission.
        let mesh_draws: Vec<MeshDraw> = scene
            .meshes
            .iter()
            .filter(|i| i.mesh.is_valid() && i.material.is_valid())
            .map(|i| MeshDraw {
                mesh: i.mesh,
                material: i.material,
                instance_bg: self.make_instance_bg(i.transform),
            })
            .collect();

        let translucent_draws: Vec<MeshDraw> = scene
            .translucent
            .iter()
            .filter(|i| i.mesh.is_valid() && i.material.is_valid())
            .map(|i| MeshDraw {
                mesh: i.mesh,
                material: i.material,
                instance_bg: self.make_instance_bg(i.transform),
            })
            .collect();

        let skinned_draws: Vec<SkinDraw> = scene
            .skinned
            .iter()
            .filter(|i| i.mesh.is_valid() && i.material.is_valid())
            .map(|i| SkinDraw {
                mesh: i.mesh,
                material: i.material,
                skin_bg: self.make_skin_bg(&i.joint_matrices, i.transform),
            })
            .collect();

        let viewmodel_draw = scene.viewmodel.as_ref().and_then(|vm| {
            (vm.mesh.is_valid() && vm.material.is_valid()).then(|| MeshDraw {
                mesh: vm.mesh,
                material: vm.material,
                instance_bg: self.make_instance_bg(vm.transform),
            })
        });

        // Debug line vertices (two per segment).
        let mut debug_verts: Vec<DebugVertex> = Vec::with_capacity(scene.debug.lines.len() * 2);
        for line in &scene.debug.lines {
            debug_verts.push(DebugVertex {
                pos: line.a.to_array(),
                color: line.color,
            });
            debug_verts.push(DebugVertex {
                pos: line.b.to_array(),
                color: line.color,
            });
        }
        let debug_count = debug_verts.len() as u32;
        let debug_vbuf = (!debug_verts.is_empty())
            .then(|| make_buffer(&self.device, "p3d.debug-vbuf", &debug_verts, wgpu::BufferUsages::VERTEX));

        // HUD geometry: crosshair + health bar (DESIGN.md §21, minimal v0).
        let hud_verts = build_hud(&scene.hud, aspect);
        let hud_count = hud_verts.len() as u32;
        let hud_vbuf = (!hud_verts.is_empty())
            .then(|| make_buffer(&self.device, "p3d.hud-vbuf", &hud_verts, wgpu::BufferUsages::VERTEX));

        // --- Acquire the swapchain image ----------------------------------
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.reconfigure();
                self.surface.get_current_texture()?
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("p3d.encoder"),
            });

        // ===== Pass A: World opaque + Skinned + Alpha/translucent =========
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p3d.pass.world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.06,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // -- 1. World opaque (BspWorldLit / BspSky) --------------------
            if let Some(world) = scene.world.and_then(|h| self.worlds.get(h.index())) {
                rp.set_pipeline(&self.world_opaque_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                rp.set_vertex_buffer(0, world.vertex_buffer.slice(..));
                rp.set_index_buffer(world.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                for b in &world.batches {
                    if is_world_opaque(b.kind) {
                        rp.set_bind_group(1, &b.material_bg, &[]);
                        rp.draw_indexed(b.index_start..b.index_start + b.index_count, 0, 0..1);
                    }
                }
            }

            // -- 2. Skinned actors -----------------------------------------
            if !skinned_draws.is_empty() {
                rp.set_pipeline(&self.skinned_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                for d in &skinned_draws {
                    let (Some(mesh), Some(mat)) = (
                        self.skinned_meshes.get(d.mesh.index()),
                        self.materials.get(d.material.index()),
                    ) else {
                        continue;
                    };
                    rp.set_bind_group(1, &mat.bind_group, &[]);
                    rp.set_bind_group(2, &d.skin_bg, &[]);
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }

            // -- 3. Opaque static meshes (props) drawn with the world pass --
            if !mesh_draws.is_empty() {
                rp.set_pipeline(&self.mesh_opaque_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                for d in &mesh_draws {
                    let (Some(mesh), Some(mat)) = (
                        self.static_meshes.get(d.mesh.index()),
                        self.materials.get(d.material.index()),
                    ) else {
                        continue;
                    };
                    rp.set_bind_group(1, &mat.bind_group, &[]);
                    rp.set_bind_group(2, &d.instance_bg, &[]);
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }

            // -- 4. Alpha / translucent ------------------------------------
            // World alpha-tested surfaces (BspAlphaTest) first ...
            if let Some(world) = scene.world.and_then(|h| self.worlds.get(h.index())) {
                rp.set_pipeline(&self.world_alpha_test_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                rp.set_vertex_buffer(0, world.vertex_buffer.slice(..));
                rp.set_index_buffer(world.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                for b in &world.batches {
                    if b.kind.is_alpha_tested() {
                        rp.set_bind_group(1, &b.material_bg, &[]);
                        rp.draw_indexed(b.index_start..b.index_start + b.index_count, 0, 0..1);
                    }
                }
                // ... then blended water surfaces (BspWater).
                rp.set_pipeline(&self.world_water_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                rp.set_vertex_buffer(0, world.vertex_buffer.slice(..));
                rp.set_index_buffer(world.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                for b in &world.batches {
                    if b.kind.is_translucent() {
                        rp.set_bind_group(1, &b.material_bg, &[]);
                        rp.draw_indexed(b.index_start..b.index_start + b.index_count, 0, 0..1);
                    }
                }
            }

            // Translucent static-mesh instances (water/glass props).
            if !translucent_draws.is_empty() {
                rp.set_pipeline(&self.mesh_translucent_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                for d in &translucent_draws {
                    let (Some(mesh), Some(mat)) = (
                        self.static_meshes.get(d.mesh.index()),
                        self.materials.get(d.material.index()),
                    ) else {
                        continue;
                    };
                    rp.set_bind_group(1, &mat.bind_group, &[]);
                    rp.set_bind_group(2, &d.instance_bg, &[]);
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }
        }

        // ===== Pass B: Viewmodel (depth cleared to avoid clipping) ========
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p3d.pass.viewmodel"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(d) = &viewmodel_draw {
                if let (Some(mesh), Some(mat)) = (
                    self.static_meshes.get(d.mesh.index()),
                    self.materials.get(d.material.index()),
                ) {
                    rp.set_pipeline(&self.mesh_opaque_pipeline);
                    rp.set_bind_group(0, &self.viewmodel_frame_bind_group, &[]);
                    rp.set_bind_group(1, &mat.bind_group, &[]);
                    rp.set_bind_group(2, &d.instance_bg, &[]);
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }
        }

        // ===== Pass C: Debug lines + HUD overlay ==========================
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p3d.pass.overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // -- 5. Debug lines --------------------------------------------
            if let Some(vb) = &debug_vbuf {
                rp.set_pipeline(&self.debug_pipeline);
                rp.set_bind_group(0, &self.frame_bind_group, &[]);
                rp.set_vertex_buffer(0, vb.slice(..));
                rp.draw(0..debug_count, 0..1);
            }

            // -- 6. HUD ----------------------------------------------------
            if let Some(vb) = &hud_vbuf {
                rp.set_pipeline(&self.hud_pipeline);
                rp.set_vertex_buffer(0, vb.slice(..));
                rp.draw(0..hud_count, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RenderDevice implementation (DESIGN.md §12)
// ---------------------------------------------------------------------------

impl RenderDevice for WgpuRenderer {
    fn upload_world(&mut self, world: &WorldUpload<'_>) -> WorldHandle {
        // Interleaved vertex + index buffers for the whole world mesh.
        let vertex_buffer = make_buffer(
            &self.device,
            "p3d.world-vbuf",
            &world.mesh.vertices,
            wgpu::BufferUsages::VERTEX,
        );
        let index_buffer = make_buffer(
            &self.device,
            "p3d.world-ibuf",
            &world.mesh.indices,
            wgpu::BufferUsages::INDEX,
        );

        // Upload each base texture, and the single lightmap atlas as one
        // texture (DESIGN.md §12). Fall back to white when a map has no
        // lighting so the multiply is a no-op.
        let textures: Vec<GpuTexture> = world
            .textures
            .iter()
            .map(|t| upload_texture_data(&self.device, &self.queue, t))
            .collect();
        let lightmap = match world.lightmap_atlas {
            Some(t) => upload_texture_data(&self.device, &self.queue, t),
            None => upload_texture_data(&self.device, &self.queue, &TextureData::solid(255, 255, 255, 255)),
        };

        // One bind group + draw batch per submesh (batched by material).
        let mut batches = Vec::with_capacity(world.mesh.submeshes.len());
        for sm in &world.mesh.submeshes {
            let mat = world
                .materials
                .get(sm.material as usize)
                .cloned()
                .unwrap_or_else(|| MaterialDesc::new("<missing>", MaterialKind::BspWorldLit));
            let base_view = mat
                .base_texture
                .and_then(|i| textures.get(i as usize))
                .map(|g| &g.view)
                .unwrap_or(&self.white_texture.view);

            let material_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("p3d.world-material-bg"),
                layout: &self.world_material_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(base_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&lightmap.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            batches.push(WorldBatch {
                index_start: sm.index_start,
                index_count: sm.index_count,
                kind: mat.kind,
                material_bg,
            });
        }

        let handle = WorldHandle(self.worlds.len() as u32);
        self.worlds.push(GpuWorld {
            vertex_buffer,
            index_buffer,
            batches,
            textures,
            lightmap,
        });
        handle
    }

    fn upload_static_mesh(&mut self, mesh: &MeshData<StaticVertex>) -> MeshHandle {
        let gpu = self.build_gpu_mesh("p3d.static-mesh", &mesh.vertices, &mesh.indices);
        let handle = MeshHandle(self.static_meshes.len() as u32);
        self.static_meshes.push(gpu);
        handle
    }

    fn upload_skinned_mesh(&mut self, mesh: &MeshData<SkinnedVertex>) -> SkinnedMeshHandle {
        let gpu = self.build_gpu_mesh("p3d.skinned-mesh", &mesh.vertices, &mesh.indices);
        let handle = SkinnedMeshHandle(self.skinned_meshes.len() as u32);
        self.skinned_meshes.push(gpu);
        handle
    }

    fn upload_texture(&mut self, tex: &TextureData) -> TextureHandle {
        let gpu = upload_texture_data(&self.device, &self.queue, tex);
        let handle = TextureHandle(self.textures.len() as u32);
        self.textures.push(gpu);
        handle
    }

    fn create_material(&mut self, desc: &MaterialDesc, base: TextureHandle) -> MaterialHandle {
        let base_view = self
            .textures
            .get(base.index())
            .map(|g| &g.view)
            .unwrap_or(&self.white_texture.view);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("p3d.material-bg"),
            layout: &self.mesh_material_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let handle = MaterialHandle(self.materials.len() as u32);
        self.materials.push(GpuMaterial {
            kind: desc.kind,
            base,
            bind_group,
        });
        handle
    }
}

impl WgpuRenderer {
    /// Build a vertex + index buffer pair from CPU mesh data.
    fn build_gpu_mesh<V: Pod>(&self, label: &str, vertices: &[V], indices: &[u32]) -> GpuMesh {
        GpuMesh {
            vertex_buffer: make_buffer(&self.device, label, vertices, wgpu::BufferUsages::VERTEX),
            index_buffer: make_buffer(&self.device, label, indices, wgpu::BufferUsages::INDEX),
            index_count: indices.len() as u32,
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// True for world materials drawn in the opaque pass (vs. alpha/water).
fn is_world_opaque(kind: MaterialKind) -> bool {
    matches!(
        kind,
        MaterialKind::BspWorldLit
            | MaterialKind::BspSky
            | MaterialKind::StaticUnlit
            | MaterialKind::StaticLit
    )
}

/// A `BindGroupLayoutEntry` for a uniform buffer at `binding`.
fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A `BindGroupLayoutEntry` for a read-only storage buffer at `binding`.
fn storage_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A fragment-visible filterable 2D texture binding at `binding`.
fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A fragment-visible filtering sampler binding at `binding`.
fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

/// A per-vertex `VertexBufferLayout` for a POD vertex `V` and its attributes.
fn vbl<V>(attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<V>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: attrs,
    }
}

/// Compile an inline WGSL module.
fn wgsl(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

/// Create a buffer initialized from `data`, using a small non-empty fallback so
/// zero-length uploads never hit wgpu's "buffer size must be > 0" validation.
fn make_buffer<T: Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let bytes: &[u8] = bytemuck::cast_slice(data);
    let fallback = [0u8; 16];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: if bytes.is_empty() { &fallback } else { bytes },
        usage,
    })
}

/// Create the Depth32Float depth buffer for the given size.
fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("p3d.depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// Upload an RGBA8 [`TextureData`] as an sRGB 2D texture (DESIGN.md §12).
fn upload_texture_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &TextureData,
) -> GpuTexture {
    let width = tex.width.max(1);
    let height = tex.height.max(1);
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("p3d.texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Guard against short/empty payloads (e.g. a 0-sized placeholder).
    let expected = (width * height * 4) as usize;
    let mut data = tex.rgba.clone();
    if data.len() < expected {
        data.resize(expected, 255);
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data[..expected],
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuTexture { texture, view }
}

/// Build the HUD vertex list: a crosshair plus a health bar (DESIGN.md §21).
/// Kept intentionally minimal for v0 — no bitmap font.
fn build_hud(hud: &pocket3d_render::Hud, aspect: f32) -> Vec<HudVertex> {
    let mut verts: Vec<HudVertex> = Vec::new();
    let ax = if aspect > 0.0 { 1.0 / aspect } else { 1.0 };

    if hud.show_crosshair {
        let arm = 0.03_f32;
        let thick = 0.004_f32;
        let color = [1.0, 1.0, 1.0, 0.85];
        // Horizontal + vertical bars, aspect-corrected so it reads square.
        push_rect(&mut verts, 0.0, 0.0, arm * ax, thick, color);
        push_rect(&mut verts, 0.0, 0.0, thick * ax, arm, color);
    }

    // Health bar, bottom-left corner.
    let bar_w = 0.30_f32;
    let bar_h = 0.03_f32;
    let x0 = -0.95_f32;
    let y0 = -0.95_f32;
    let frac = (hud.health.clamp(0, 100) as f32) / 100.0;
    push_rect(
        &mut verts,
        x0 + bar_w * 0.5,
        y0 + bar_h * 0.5,
        bar_w * 0.5,
        bar_h * 0.5,
        [0.1, 0.1, 0.1, 0.55],
    );
    let fill_w = bar_w * frac;
    push_rect(
        &mut verts,
        x0 + fill_w * 0.5,
        y0 + bar_h * 0.5,
        fill_w * 0.5,
        bar_h * 0.5,
        [0.85, 0.2, 0.2, 0.9],
    );

    verts
}

/// Append an axis-aligned NDC rectangle (two triangles) to `out`.
fn push_rect(out: &mut Vec<HudVertex>, cx: f32, cy: f32, hw: f32, hh: f32, color: [f32; 4]) {
    let (x0, x1, y0, y1) = (cx - hw, cx + hw, cy - hh, cy + hh);
    let v = |x: f32, y: f32| HudVertex { pos: [x, y], color };
    out.extend_from_slice(&[
        v(x0, y0),
        v(x1, y0),
        v(x1, y1),
        v(x0, y0),
        v(x1, y1),
        v(x0, y1),
    ]);
}

// ---------------------------------------------------------------------------
// Pipeline builder
// ---------------------------------------------------------------------------

/// Parameters for [`make_pipeline`], keeping the many `wgpu` knobs in one place.
struct PipelineCfg<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    shader: &'a wgpu::ShaderModule,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [wgpu::VertexBufferLayout<'a>],
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    cull: Option<wgpu::Face>,
}

fn make_pipeline(device: &wgpu::Device, cfg: PipelineCfg<'_>) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(cfg.label),
        layout: Some(cfg.layout),
        vertex: wgpu::VertexState {
            module: cfg.shader,
            entry_point: cfg.vs,
            buffers: cfg.buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: cfg.shader,
            entry_point: cfg.fs,
            targets: &[Some(wgpu::ColorTargetState {
                format: cfg.format,
                blend: cfg.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: cfg.topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: cfg.cull,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: cfg.depth_write,
            depth_compare: cfg.depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}
