//! Procedural, **project-owned (CC0)** placeholder content (DESIGN.md §11).
//!
//! The repository ships **no proprietary assets**. Everything a demo needs is
//! generated here from first principles: a blocky rigged humanoid "bot" for
//! OpenStrike and a static rifle viewmodel. The exact same humanoid is exposed
//! two ways so the skinned pipeline is testable end to end:
//!
//! * [`write_bot_glb`] emits a real `.glb` (JSON + BIN chunks, accessors,
//!   `JOINTS_0`/`WEIGHTS_0`, a skin with inverse bind matrices, and three
//!   named animations) that round-trips through [`crate::import::import_glb`];
//! * [`procedural_bot`] builds the identical model directly as in-memory
//!   `pocket3d` types, a guaranteed-available fallback if the GLB round-trip
//!   ever misbehaves on a target platform.
//!
//! Both paths are driven by one shared spec ([`bot_joints`], [`bot_boxes`],
//! [`bot_clip_specs`]) so they can never drift apart.

use crate::glb::{
    assemble_glb, BinBuilder, ARRAY_BUFFER,
};
use crate::import::ImportedModel;
use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3, Vec4};
use pocket3d_anim::{AnimationClip, Channel, ChannelKind, Joint, Skeleton};
use pocket3d_core::geom::Aabb;
use pocket3d_core::mesh::{MeshData, SkinnedVertex, Submesh};
use pocket3d_core::Transform;
use serde_json::json;
use std::path::Path;

// ---------------------------------------------------------------------------
// Shared bot specification
// ---------------------------------------------------------------------------

/// One joint of the placeholder humanoid, expressed by its **model-space** rest
/// origin (from which parent-relative local binds are derived).
struct JointSpec {
    name: &'static str,
    parent: Option<usize>,
    model_pos: Vec3,
}

/// A body box weighted fully (weight 1.0) to a single joint.
struct BoxSpec {
    joint: usize,
    center: Vec3,
    half: Vec3,
}

/// The bot skeleton: 7 joints (>= the 6 the design requires), topologically
/// sorted so every parent precedes its children (DESIGN.md §17).
fn bot_joints() -> Vec<JointSpec> {
    vec![
        JointSpec { name: "pelvis", parent: None, model_pos: Vec3::new(0.0, 0.0, 1.00) },
        JointSpec { name: "spine", parent: Some(0), model_pos: Vec3::new(0.0, 0.0, 1.30) },
        JointSpec { name: "head", parent: Some(1), model_pos: Vec3::new(0.0, 0.0, 1.78) },
        JointSpec { name: "left_arm", parent: Some(1), model_pos: Vec3::new(0.30, 0.0, 1.45) },
        JointSpec { name: "right_arm", parent: Some(1), model_pos: Vec3::new(-0.30, 0.0, 1.45) },
        JointSpec { name: "left_leg", parent: Some(0), model_pos: Vec3::new(0.15, 0.0, 1.00) },
        JointSpec { name: "right_leg", parent: Some(0), model_pos: Vec3::new(-0.15, 0.0, 1.00) },
    ]
}

/// One box per joint: torso/head/limbs, each rigid-weighted to its joint.
fn bot_boxes() -> Vec<BoxSpec> {
    vec![
        BoxSpec { joint: 0, center: Vec3::new(0.0, 0.0, 1.00), half: Vec3::new(0.28, 0.16, 0.14) },
        BoxSpec { joint: 1, center: Vec3::new(0.0, 0.0, 1.35), half: Vec3::new(0.26, 0.15, 0.30) },
        BoxSpec { joint: 2, center: Vec3::new(0.0, 0.0, 1.85), half: Vec3::new(0.16, 0.16, 0.16) },
        BoxSpec { joint: 3, center: Vec3::new(0.42, 0.0, 1.32), half: Vec3::new(0.09, 0.09, 0.26) },
        BoxSpec { joint: 4, center: Vec3::new(-0.42, 0.0, 1.32), half: Vec3::new(0.09, 0.09, 0.26) },
        BoxSpec { joint: 5, center: Vec3::new(0.15, 0.0, 0.55), half: Vec3::new(0.11, 0.11, 0.45) },
        BoxSpec { joint: 6, center: Vec3::new(-0.15, 0.0, 0.55), half: Vec3::new(0.11, 0.11, 0.45) },
    ]
}

/// Build the bot [`Skeleton`] from the joint spec: local binds are the
/// parent-relative rest translations, and inverse binds are the inverse of the
/// accumulated model-space bind matrices (identity skinning palette at rest,
/// DESIGN.md §17).
pub fn procedural_skeleton() -> Skeleton {
    let specs = bot_joints();
    let mut skeleton = Skeleton { joints: Vec::with_capacity(specs.len()) };
    for (i, s) in specs.iter().enumerate() {
        let parent_pos = s.parent.map(|p| specs[p].model_pos).unwrap_or(Vec3::ZERO);
        let local = s.model_pos - parent_pos;
        skeleton.joints.push(Joint {
            name: s.name.to_string(),
            parent: s.parent,
            local_bind: Transform::from_translation(local),
            inverse_bind: Mat4::IDENTITY, // filled below
        });
        debug_assert!(s.parent.map(|p| p < i).unwrap_or(true), "joints must be topo-sorted");
    }
    // inverse_bind = inverse of each joint's model-space bind matrix.
    let model = skeleton.bind_model_matrices();
    for (joint, m) in skeleton.joints.iter_mut().zip(model) {
        joint.inverse_bind = m.inverse();
    }
    skeleton
}

// ---------------------------------------------------------------------------
// Shared box geometry
// ---------------------------------------------------------------------------

/// Interleaved-free vertex arrays for the bot mesh, in glTF-friendly layout.
struct MeshArrays {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

/// Per-face UVs for a unit quad.
const FACE_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// The six faces of a box as `(outward normal, four corner sign-triples)`, wound
/// counter-clockwise seen from outside.
const BOX_FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
    ([1.0, 0.0, 0.0], [[1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]]),
    ([-1.0, 0.0, 0.0], [[-1.0, 1.0, -1.0], [-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]]),
    ([0.0, 1.0, 0.0], [[1.0, 1.0, -1.0], [-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0]]),
    ([0.0, -1.0, 0.0], [[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0]]),
    ([0.0, 0.0, 1.0], [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]]),
    ([0.0, 0.0, -1.0], [[-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, -1.0, -1.0], [-1.0, -1.0, -1.0]]),
];

/// Build every body box into flat vertex/index arrays. Each vertex is rigid-
/// weighted (`[1,0,0,0]`) to its box's joint (DESIGN.md §11/§17).
fn build_bot_mesh_arrays() -> MeshArrays {
    let boxes = bot_boxes();
    let mut a = MeshArrays {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        joints: Vec::new(),
        weights: Vec::new(),
        indices: Vec::new(),
    };
    for b in &boxes {
        for (normal, corners) in BOX_FACES.iter() {
            let base = a.positions.len() as u32;
            for (k, sign) in corners.iter().enumerate() {
                a.positions.push([
                    b.center.x + b.half.x * sign[0],
                    b.center.y + b.half.y * sign[1],
                    b.center.z + b.half.z * sign[2],
                ]);
                a.normals.push(*normal);
                a.uvs.push(FACE_UV[k]);
                a.joints.push([b.joint as u16, 0, 0, 0]);
                a.weights.push([1.0, 0.0, 0.0, 0.0]);
            }
            a.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    a
}

/// Build the bot mesh as an in-memory [`MeshData<SkinnedVertex>`] with a single
/// submesh (material 0).
pub fn procedural_mesh() -> MeshData<SkinnedVertex> {
    let a = build_bot_mesh_arrays();
    let mut vertices = Vec::with_capacity(a.positions.len());
    for i in 0..a.positions.len() {
        vertices.push(SkinnedVertex {
            pos: a.positions[i],
            normal: a.normals[i],
            uv: a.uvs[i],
            joints: [
                a.joints[i][0] as u32,
                a.joints[i][1] as u32,
                a.joints[i][2] as u32,
                a.joints[i][3] as u32,
            ],
            weights: a.weights[i],
        });
    }
    let bounds = Aabb::from_points(vertices.iter().map(|v| Vec3::from(v.pos)));
    let index_count = a.indices.len() as u32;
    MeshData {
        vertices,
        indices: a.indices,
        submeshes: vec![Submesh { material: 0, index_start: 0, index_count }],
        bounds,
    }
}

// ---------------------------------------------------------------------------
// Shared animation specs
// ---------------------------------------------------------------------------

/// One animation track in the neutral spec used by both the in-memory and GLB
/// builders. Rotations are stored `xyzw`; translations/scales use `.xyz`.
enum TrackSpec {
    Translation { joint: usize, times: Vec<f32>, values: Vec<[f32; 3]> },
    Rotation { joint: usize, times: Vec<f32>, values: Vec<[f32; 4]> },
}

/// A named clip spec: shared source of truth for both output paths.
struct ClipSpec {
    name: &'static str,
    tracks: Vec<TrackSpec>,
}

/// Quaternion (as `xyzw`) for a rotation of `deg` degrees about +X.
fn quat_x(deg: f32) -> [f32; 4] {
    Quat::from_rotation_x(deg.to_radians()).to_array()
}

/// The three bot clips required by DESIGN.md §11/§16: `Idle`, `Walk`, `Death`.
fn bot_clip_specs() -> Vec<ClipSpec> {
    let specs = bot_joints();
    let pelvis_local = specs[0].model_pos; // pelvis is the root

    // Idle: a subtle vertical (Z) bob of the whole body via the pelvis.
    let idle = ClipSpec {
        name: "Idle",
        tracks: vec![TrackSpec::Translation {
            joint: 0,
            times: vec![0.0, 0.5, 1.0],
            values: vec![
                [pelvis_local.x, pelvis_local.y, pelvis_local.z],
                [pelvis_local.x, pelvis_local.y, pelvis_local.z + 0.04],
                [pelvis_local.x, pelvis_local.y, pelvis_local.z],
            ],
        }],
    };

    // Walk: legs and arms swing about X, arms counter-phased to the legs.
    let s = 25.0;
    let walk = ClipSpec {
        name: "Walk",
        tracks: vec![
            TrackSpec::Rotation {
                joint: 5, // left_leg
                times: vec![0.0, 0.5, 1.0],
                values: vec![quat_x(s), quat_x(-s), quat_x(s)],
            },
            TrackSpec::Rotation {
                joint: 6, // right_leg
                times: vec![0.0, 0.5, 1.0],
                values: vec![quat_x(-s), quat_x(s), quat_x(-s)],
            },
            TrackSpec::Rotation {
                joint: 3, // left_arm (opposite the left leg)
                times: vec![0.0, 0.5, 1.0],
                values: vec![quat_x(-s), quat_x(s), quat_x(-s)],
            },
            TrackSpec::Rotation {
                joint: 4, // right_arm
                times: vec![0.0, 0.5, 1.0],
                values: vec![quat_x(s), quat_x(-s), quat_x(s)],
            },
        ],
    };

    // Death: rotate the whole body ~90deg about X (fall over) over one second.
    let death = ClipSpec {
        name: "Death",
        tracks: vec![TrackSpec::Rotation {
            joint: 0, // pelvis / whole body
            times: vec![0.0, 1.0],
            values: vec![quat_x(0.0), quat_x(-90.0)],
        }],
    };

    vec![idle, walk, death]
}

/// Convert the neutral clip specs into runtime [`AnimationClip`]s. Clip
/// duration is the maximum keyframe time (DESIGN.md §17).
fn procedural_clips() -> Vec<AnimationClip> {
    bot_clip_specs()
        .into_iter()
        .map(|c| {
            let mut duration = 0.0f32;
            let mut channels = Vec::with_capacity(c.tracks.len());
            for track in c.tracks {
                match track {
                    TrackSpec::Translation { joint, times, values } => {
                        duration = duration.max(times.iter().copied().fold(0.0, f32::max));
                        channels.push(Channel {
                            target_joint: joint,
                            kind: ChannelKind::Translation,
                            times,
                            values: values
                                .into_iter()
                                .map(|v| Vec4::new(v[0], v[1], v[2], 0.0))
                                .collect(),
                        });
                    }
                    TrackSpec::Rotation { joint, times, values } => {
                        duration = duration.max(times.iter().copied().fold(0.0, f32::max));
                        channels.push(Channel {
                            target_joint: joint,
                            kind: ChannelKind::Rotation,
                            times,
                            values: values
                                .into_iter()
                                .map(|q| Vec4::new(q[0], q[1], q[2], q[3]))
                                .collect(),
                        });
                    }
                }
            }
            AnimationClip { name: c.name.to_string(), duration, channels }
        })
        .collect()
}

/// Construct the placeholder bot **directly** as in-memory `pocket3d` types
/// (skeleton + skinned mesh + Idle/Walk/Death clips), bypassing glTF entirely
/// (DESIGN.md §11). This is OpenStrike's guaranteed-available fallback.
pub fn procedural_bot() -> ImportedModel {
    ImportedModel {
        skinned_mesh: procedural_mesh(),
        skeleton: procedural_skeleton(),
        clips: procedural_clips(),
        base_texture: None,
    }
}

// ---------------------------------------------------------------------------
// GLB emission
// ---------------------------------------------------------------------------

/// Write a valid skinned `.glb` of the placeholder humanoid to `path`
/// (DESIGN.md §9 asset path, §11 project-owned content). The result round-trips
/// through [`crate::import::import_glb`].
pub fn write_bot_glb(path: impl AsRef<Path>) -> Result<()> {
    let joints = bot_joints();
    let skeleton = procedural_skeleton();
    let arrays = build_bot_mesh_arrays();

    let mut bin = BinBuilder::new();

    // --- Mesh attribute + index accessors ---
    let a_pos = bin.accessor_vec3(&arrays.positions, Some(ARRAY_BUFFER));
    let a_nrm = bin.accessor_vec3(&arrays.normals, Some(ARRAY_BUFFER));
    let a_uv = bin.accessor_vec2(&arrays.uvs, Some(ARRAY_BUFFER));
    let a_joints = bin.accessor_vec4_u16(&arrays.joints, Some(ARRAY_BUFFER));
    let a_weights = bin.accessor_vec4_f32(&arrays.weights, Some(ARRAY_BUFFER));
    let a_indices = bin.accessor_indices(&arrays.indices);

    // --- Inverse bind matrices (column-major MAT4) ---
    let ibms: Vec<[f32; 16]> = skeleton
        .joints
        .iter()
        .map(|j| j.inverse_bind.to_cols_array())
        .collect();
    let a_ibm = bin.accessor_mat4(&ibms);

    // --- Animation sampler accessors ---
    // Collect (name, [(sampler_input, sampler_output, node, path)]).
    struct GlbTrack {
        input: usize,
        output: usize,
        node: usize,
        path: &'static str,
    }
    let mut anim_tracks: Vec<(&'static str, Vec<GlbTrack>)> = Vec::new();
    for clip in bot_clip_specs() {
        let mut tracks = Vec::new();
        for track in clip.tracks {
            match track {
                TrackSpec::Translation { joint, times, values } => {
                    let input = bin.accessor_scalar_f32(&times);
                    let output = bin.accessor_vec3(&values, None);
                    tracks.push(GlbTrack { input, output, node: joint, path: "translation" });
                }
                TrackSpec::Rotation { joint, times, values } => {
                    let input = bin.accessor_scalar_f32(&times);
                    let output = bin.accessor_vec4_f32(&values, None);
                    tracks.push(GlbTrack { input, output, node: joint, path: "rotation" });
                }
            }
        }
        anim_tracks.push((clip.name, tracks));
    }

    // --- Nodes: joints 0..N-1 occupy node indices 0..N-1, mesh node is last ---
    let mesh_node = joints.len();
    let mut nodes = Vec::with_capacity(joints.len() + 1);
    for (i, spec) in joints.iter().enumerate() {
        // Children = every joint whose parent is `i`.
        let children: Vec<usize> = joints
            .iter()
            .enumerate()
            .filter(|(_, c)| c.parent == Some(i))
            .map(|(ci, _)| ci)
            .collect();
        let t = skeleton.joints[i].local_bind.translation;
        let mut node = json!({
            "name": spec.name,
            "translation": [t.x, t.y, t.z],
        });
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
    }
    // The skinned-mesh node.
    nodes.push(json!({
        "name": "bot_mesh",
        "mesh": 0,
        "skin": 0,
    }));

    // --- Assemble animations JSON ---
    let animations: Vec<_> = anim_tracks
        .iter()
        .map(|(name, tracks)| {
            let samplers: Vec<_> = tracks
                .iter()
                .map(|t| json!({ "input": t.input, "output": t.output, "interpolation": "LINEAR" }))
                .collect();
            let channels: Vec<_> = tracks
                .iter()
                .enumerate()
                .map(|(si, t)| json!({ "sampler": si, "target": { "node": t.node, "path": t.path } }))
                .collect();
            json!({ "name": name, "samplers": samplers, "channels": channels })
        })
        .collect();

    let root = json!({
        "asset": { "version": "2.0", "generator": "pocket3d-assets (CC0 placeholder)" },
        "scene": 0,
        "scenes": [ { "nodes": [0, mesh_node] } ],
        "nodes": nodes,
        "meshes": [ {
            "name": "bot",
            "primitives": [ {
                "attributes": {
                    "POSITION": a_pos,
                    "NORMAL": a_nrm,
                    "TEXCOORD_0": a_uv,
                    "JOINTS_0": a_joints,
                    "WEIGHTS_0": a_weights,
                },
                "indices": a_indices,
                "mode": 4,
            } ]
        } ],
        "skins": [ {
            "joints": (0..joints.len()).collect::<Vec<_>>(),
            "inverseBindMatrices": a_ibm,
            "skeleton": 0,
        } ],
        "animations": animations,
        "buffers": [ { "byteLength": bin.bin_len() } ],
        "bufferViews": bin.buffer_views(),
        "accessors": bin.accessors(),
    });

    let glb = assemble_glb(&root, &bin.into_bin());
    std::fs::write(path.as_ref(), &glb)
        .with_context(|| format!("writing bot glb to {}", path.as_ref().display()))?;
    Ok(())
}

/// Write a simple **static** rifle silhouette (three boxes: body, barrel,
/// magazine) as a non-skinned `.glb` for the first-person viewmodel
/// (DESIGN.md §11). Read back with [`crate::import::import_static_glb`].
pub fn write_weapon_glb(path: impl AsRef<Path>) -> Result<()> {
    // Boxes: (center, half-extents). A long thin body, a thinner barrel out
    // front (+Y), and a magazine hanging below.
    let boxes = [
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.04, 0.30, 0.06)),
        (Vec3::new(0.0, 0.45, 0.02), Vec3::new(0.02, 0.20, 0.02)),
        (Vec3::new(0.0, -0.05, -0.12), Vec3::new(0.03, 0.06, 0.08)),
    ];

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (center, half) in boxes {
        for (normal, corners) in BOX_FACES.iter() {
            let base = positions.len() as u32;
            for (k, sign) in corners.iter().enumerate() {
                positions.push([
                    center.x + half.x * sign[0],
                    center.y + half.y * sign[1],
                    center.z + half.z * sign[2],
                ]);
                normals.push(*normal);
                uvs.push(FACE_UV[k]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    let mut bin = BinBuilder::new();
    let a_pos = bin.accessor_vec3(&positions, Some(ARRAY_BUFFER));
    let a_nrm = bin.accessor_vec3(&normals, Some(ARRAY_BUFFER));
    let a_uv = bin.accessor_vec2(&uvs, Some(ARRAY_BUFFER));
    let a_indices = bin.accessor_indices(&indices);

    let root = json!({
        "asset": { "version": "2.0", "generator": "pocket3d-assets (CC0 placeholder)" },
        "scene": 0,
        "scenes": [ { "nodes": [0] } ],
        "nodes": [ { "name": "weapon", "mesh": 0 } ],
        "meshes": [ {
            "name": "rifle",
            "primitives": [ {
                "attributes": { "POSITION": a_pos, "NORMAL": a_nrm, "TEXCOORD_0": a_uv },
                "indices": a_indices,
                "mode": 4,
            } ]
        } ],
        "buffers": [ { "byteLength": bin.bin_len() } ],
        "bufferViews": bin.buffer_views(),
        "accessors": bin.accessors(),
    });

    let glb = assemble_glb(&root, &bin.into_bin());
    std::fs::write(path.as_ref(), &glb)
        .with_context(|| format!("writing weapon glb to {}", path.as_ref().display()))?;
    Ok(())
}
