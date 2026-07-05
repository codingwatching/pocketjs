//! Inline WGSL shader sources for each render pass (DESIGN.md §12).
//!
//! One shader module per pass family. They intentionally implement the
//! "forward-rendered and intentionally simple" model from the design: no PBR,
//! no dynamic shadows. All shaders share the same `Frame` uniform layout
//! (mirrored by [`crate::renderer::FrameUniform`]): a `view_proj` matrix, the
//! camera position, a single directional light, and packed `params`
//! (`x = ambient`, `y = exposure`).

/// World opaque / alpha-tested / water surfaces: `base_texture * lightmap *
/// exposure` (DESIGN.md §12 "World opaque pass").
pub const WORLD_WGSL: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    params: vec4<f32>, // x = ambient, y = exposure
};
@group(0) @binding(0) var<uniform> frame: Frame;

@group(1) @binding(0) var base_tex: texture_2d<f32>;
@group(1) @binding(1) var lm_tex: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) uv_lm: vec2<f32>,
};

@vertex
fn vs_world(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_lm: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    // World geometry is already in world space, so only view/projection apply.
    out.clip = frame.view_proj * vec4<f32>(pos, 1.0);
    out.uv = uv;
    out.uv_lm = uv_lm;
    return out;
}

@fragment
fn fs_world(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(base_tex, samp, in.uv);
    let lm = textureSample(lm_tex, samp, in.uv_lm);
    let rgb = base.rgb * lm.rgb * frame.params.y;
    return vec4<f32>(rgb, base.a);
}

@fragment
fn fs_world_alpha(in: VsOut) -> @location(0) vec4<f32> {
    // Sample everything before the discard so texture reads stay in uniform
    // control flow (WGSL requirement).
    let base = textureSample(base_tex, samp, in.uv);
    let lm = textureSample(lm_tex, samp, in.uv_lm);
    if (base.a < 0.5) {
        discard;
    }
    let rgb = base.rgb * lm.rgb * frame.params.y;
    return vec4<f32>(rgb, 1.0);
}
"#;

/// Static/prop/viewmodel meshes: simple N·L directional + ambient lighting
/// (DESIGN.md §12 "Static prop" / "Viewmodel pass").
pub const MESH_WGSL: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    params: vec4<f32>, // x = ambient, y = exposure
};
@group(0) @binding(0) var<uniform> frame: Frame;

@group(1) @binding(0) var base_tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct Instance { model: mat4x4<f32> };
@group(2) @binding(0) var<uniform> inst: Instance;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_mesh(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    let world = inst.model * vec4<f32>(pos, 1.0);
    out.clip = frame.view_proj * world;
    out.uv = uv;
    out.normal = (inst.model * vec4<f32>(normal, 0.0)).xyz;
    return out;
}

fn shade(normal: vec3<f32>) -> f32 {
    let n = normalize(normal);
    let l = normalize(-frame.light_dir.xyz);
    let ndl = max(dot(n, l), 0.0);
    // Ambient floor plus a diffuse term, then exposure.
    return (frame.params.x + (1.0 - frame.params.x) * ndl) * frame.params.y;
}

@fragment
fn fs_mesh(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(base_tex, samp, in.uv);
    return vec4<f32>(base.rgb * shade(in.normal), base.a);
}

@fragment
fn fs_mesh_alpha(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(base_tex, samp, in.uv);
    if (base.a < 0.5) {
        discard;
    }
    return vec4<f32>(base.rgb * shade(in.normal), base.a);
}
"#;

/// Skinned actors: linear-blend skinning by a joint-matrix storage buffer, then
/// simple directional + ambient lighting (DESIGN.md §12 "Skinned actor pass").
pub const SKINNED_WGSL: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    params: vec4<f32>, // x = ambient, y = exposure
};
@group(0) @binding(0) var<uniform> frame: Frame;

@group(1) @binding(0) var base_tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

// `joint_matrices` are `joint_world * inverse_bind` (see pocket3d-anim).
@group(2) @binding(0) var<storage, read> joints: array<mat4x4<f32>>;
struct Instance { model: mat4x4<f32> };
@group(2) @binding(1) var<uniform> inst: Instance;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_skinned(
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) jidx: vec4<u32>,
    @location(4) jw: vec4<f32>,
) -> VsOut {
    // Weighted sum of the four influencing joint matrices (weights sum to 1).
    let skin = joints[jidx.x] * jw.x
        + joints[jidx.y] * jw.y
        + joints[jidx.z] * jw.z
        + joints[jidx.w] * jw.w;

    let skinned_pos = skin * vec4<f32>(pos, 1.0);
    let world = inst.model * skinned_pos;

    var out: VsOut;
    out.clip = frame.view_proj * world;
    out.uv = uv;
    let skinned_n = (skin * vec4<f32>(normal, 0.0)).xyz;
    out.normal = (inst.model * vec4<f32>(skinned_n, 0.0)).xyz;
    return out;
}

@fragment
fn fs_skinned(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(base_tex, samp, in.uv);
    let n = normalize(in.normal);
    let l = normalize(-frame.light_dir.xyz);
    let ndl = max(dot(n, l), 0.0);
    let light = (frame.params.x + (1.0 - frame.params.x) * ndl) * frame.params.y;
    return vec4<f32>(base.rgb * light, base.a);
}
"#;

/// Debug line list: world-space positions with per-vertex color (DESIGN.md §12
/// "Debug draw pass" / §23).
pub const DEBUG_WGSL: &str = r#"
struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    params: vec4<f32>,
};
@group(0) @binding(0) var<uniform> frame: Frame;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_debug(@location(0) pos: vec3<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.clip = frame.view_proj * vec4<f32>(pos, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_debug(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// HUD overlay: 2D positions already in NDC with per-vertex color (DESIGN.md
/// §12 "UI / HUD overlay pass" / §21).
pub const HUD_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_hud(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.clip = vec4<f32>(pos, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_hud(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
