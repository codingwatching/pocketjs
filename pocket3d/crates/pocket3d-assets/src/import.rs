//! glTF / GLB import (DESIGN.md §17 "Skinned Animation import").
//!
//! Decodes the first skinned mesh of a glTF asset — plus its skin, inverse bind
//! matrices, and animation clips — into the runtime types defined by
//! [`pocket3d_core`] and [`pocket3d_anim`]. A separate [`import_static_glb`]
//! path handles non-skinned models (e.g. the weapon viewmodel).
//!
//! The importer is deliberately tolerant: missing normals default to the world
//! up axis, missing UVs to `(0, 0)`, missing joints/weights to a single rigid
//! influence, and missing inverse bind matrices are recomputed from the bind
//! pose. glTF skin joints are re-sorted topologically (parent before child) as
//! [`pocket3d_anim::Skeleton`] requires (DESIGN.md §17), remapping vertex joint
//! indices and animation targets to match.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3, Vec4};
use pocket3d_anim::{AnimationClip, Channel, ChannelKind, Joint, Skeleton};
use pocket3d_core::geom::Aabb;
use pocket3d_core::mesh::{MeshData, SkinnedVertex, StaticVertex, Submesh};
use pocket3d_core::texture::TextureData;
use pocket3d_core::Transform;
use std::path::Path;

/// A fully decoded skinned model (DESIGN.md §17).
pub struct ImportedModel {
    /// The skinned render mesh (positions/normals/uv + joint influences).
    pub skinned_mesh: MeshData<SkinnedVertex>,
    /// The joint hierarchy + inverse bind matrices.
    pub skeleton: Skeleton,
    /// Every animation clip found in the source asset.
    pub clips: Vec<AnimationClip>,
    /// Base color texture of the first material, if any.
    pub base_texture: Option<TextureData>,
}

/// Import a skinned model from a binary `.glb` file (DESIGN.md §17).
pub fn import_glb(path: impl AsRef<Path>) -> Result<ImportedModel> {
    let (doc, buffers, images) = gltf::import(path.as_ref())
        .with_context(|| format!("importing glb {}", path.as_ref().display()))?;
    build_skinned(&doc, &buffers, &images)
}

/// Import a skinned model from a `.gltf` file (JSON + external/embedded
/// buffers). Identical decode path to [`import_glb`].
pub fn import_gltf(path: impl AsRef<Path>) -> Result<ImportedModel> {
    let (doc, buffers, images) = gltf::import(path.as_ref())
        .with_context(|| format!("importing gltf {}", path.as_ref().display()))?;
    build_skinned(&doc, &buffers, &images)
}

/// Import a skinned model from in-memory glTF/GLB bytes (DESIGN.md §17).
pub fn import_slice(bytes: &[u8]) -> Result<ImportedModel> {
    let (doc, buffers, images) =
        gltf::import_slice(bytes).context("importing gltf/glb slice")?;
    build_skinned(&doc, &buffers, &images)
}

/// Import a **static** (non-skinned) mesh from a `.glb`, e.g. the weapon
/// viewmodel (DESIGN.md §11).
pub fn import_static_glb(path: impl AsRef<Path>) -> Result<MeshData<StaticVertex>> {
    let (doc, buffers, _images) = gltf::import(path.as_ref())
        .with_context(|| format!("importing static glb {}", path.as_ref().display()))?;
    build_static(&doc, &buffers)
}

// ---------------------------------------------------------------------------
// Skinned decode
// ---------------------------------------------------------------------------

fn build_skinned(
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
) -> Result<ImportedModel> {
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|d| d.0.as_slice());

    // Pick the first node that carries both a skin and a mesh; fall back to the
    // first skin + first mesh in the document.
    let (skin, mesh) = doc
        .nodes()
        .find_map(|n| match (n.skin(), n.mesh()) {
            (Some(s), Some(m)) => Some((s, m)),
            _ => None,
        })
        .or_else(|| Some((doc.skins().next()?, doc.meshes().next()?)))
        .context("glTF has no skinned mesh (need a skin + a mesh)")?;

    // --- Joint order + hierarchy --------------------------------------------
    // glTF joint index i -> node index.
    let gltf_joint_nodes: Vec<usize> = skin.joints().map(|j| j.index()).collect();
    let n_joints = gltf_joint_nodes.len();
    if n_joints == 0 {
        bail!("glTF skin has zero joints");
    }
    // node index -> gltf joint index.
    let mut node_to_joint = vec![usize::MAX; doc.nodes().count()];
    for (ji, &node) in gltf_joint_nodes.iter().enumerate() {
        if node < node_to_joint.len() {
            node_to_joint[node] = ji;
        }
    }
    // child node -> parent node, by scanning every node's children.
    let mut parent_node = vec![usize::MAX; doc.nodes().count()];
    for node in doc.nodes() {
        for child in node.children() {
            if child.index() < parent_node.len() {
                parent_node[child.index()] = node.index();
            }
        }
    }
    // parent gltf joint (or None) for each gltf joint.
    let parent_joint: Vec<Option<usize>> = gltf_joint_nodes
        .iter()
        .map(|&node| {
            let pn = parent_node[node];
            if pn != usize::MAX && node_to_joint[pn] != usize::MAX {
                Some(node_to_joint[pn])
            } else {
                None
            }
        })
        .collect();

    // Topologically sort so parents precede children (Skeleton invariant).
    // `order[k]` = gltf joint emitted at new index k; `remap[g]` = new index.
    let (order, remap) = topo_sort(&parent_joint);

    // --- Inverse bind matrices (indexed by gltf joint) ----------------------
    let ibm: Option<Vec<Mat4>> = skin.reader(get).read_inverse_bind_matrices().map(|it| {
        it.map(|m| Mat4::from_cols_array_2d(&m)).collect::<Vec<_>>()
    });

    // --- Build skeleton in topo order ---------------------------------------
    let node_matrix =
        |idx: usize| Mat4::from_cols_array_2d(&doc.nodes().nth(idx).unwrap().transform().matrix());

    let mut joints = Vec::with_capacity(n_joints);
    for &g in &order {
        let node_idx = gltf_joint_nodes[g];
        let node = doc.nodes().nth(node_idx).unwrap();
        // Fold in any NON-joint ancestor nodes (e.g. a Blender 'Armature' node
        // carrying the Y-up->Z-up / global transform) up to the nearest joint
        // ancestor or the scene root, so that transform is not dropped and the
        // bind palette stays correct (DESIGN.md §17).
        let mut m = node_matrix(node_idx);
        let mut cur = parent_node[node_idx];
        while cur != usize::MAX && node_to_joint[cur] == usize::MAX {
            m = node_matrix(cur) * m;
            cur = parent_node[cur];
        }
        let (s, r, t) = m.to_scale_rotation_translation();
        let local_bind = Transform {
            translation: t,
            rotation: r,
            scale: s,
        };
        let inverse_bind = ibm
            .as_ref()
            .and_then(|v| v.get(g).copied())
            .unwrap_or(Mat4::IDENTITY);
        joints.push(Joint {
            name: node.name().map(str::to_string).unwrap_or_else(|| format!("joint_{g}")),
            parent: parent_joint[g].map(|p| remap[p]),
            local_bind,
            inverse_bind,
        });
    }
    let mut skeleton = Skeleton { joints };
    // If the asset lacked inverse bind matrices, derive them from the bind pose
    // so the rest palette is identity (DESIGN.md §17).
    if ibm.is_none() {
        let model = skeleton.bind_model_matrices();
        for (joint, m) in skeleton.joints.iter_mut().zip(model) {
            joint.inverse_bind = m.inverse();
        }
    }

    // --- Mesh ----------------------------------------------------------------
    let skinned_mesh = read_skinned_mesh(&mesh, buffers, &remap)?;

    // --- Animations ----------------------------------------------------------
    let clips = read_clips(doc, buffers, &node_to_joint, &remap);

    // --- Base color texture --------------------------------------------------
    let base_texture = read_base_texture(&mesh, images);

    Ok(ImportedModel { skinned_mesh, skeleton, clips, base_texture })
}

/// Kahn-style topological sort of joints so each parent precedes its children.
/// Returns `(order, remap)` where `order[k]` is the gltf joint at new index `k`
/// and `remap[g]` is the new index of gltf joint `g`. Robust to dangling
/// parents / cycles (leftovers are appended in original order).
fn topo_sort(parent_joint: &[Option<usize>]) -> (Vec<usize>, Vec<usize>) {
    let n = parent_joint.len();
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut remap = vec![usize::MAX; n];

    loop {
        let mut progressed = false;
        for i in 0..n {
            if visited[i] {
                continue;
            }
            let ready = match parent_joint[i] {
                None => true,
                Some(p) => visited[p],
            };
            if ready {
                visited[i] = true;
                remap[i] = order.len();
                order.push(i);
                progressed = true;
            }
        }
        if order.len() == n {
            break;
        }
        if !progressed {
            // Cycle or unreachable parent: emit the rest verbatim.
            for i in 0..n {
                if !visited[i] {
                    visited[i] = true;
                    remap[i] = order.len();
                    order.push(i);
                }
            }
            break;
        }
    }
    (order, remap)
}

/// Read every primitive of `mesh` into one interleaved [`MeshData<SkinnedVertex>`],
/// one submesh per primitive, remapping glTF joint indices through `remap`.
fn read_skinned_mesh(
    mesh: &gltf::Mesh,
    buffers: &[gltf::buffer::Data],
    remap: &[usize],
) -> Result<MeshData<SkinnedVertex>> {
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|d| d.0.as_slice());

    let mut vertices: Vec<SkinnedVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut submeshes: Vec<Submesh> = Vec::new();

    for prim in mesh.primitives() {
        let reader = prim.reader(get);
        let Some(positions) = reader.read_positions() else {
            continue; // no geometry
        };
        let positions: Vec<[f32; 3]> = positions.collect();
        let vcount = positions.len();

        let normals: Vec<[f32; 3]> = match reader.read_normals() {
            Some(it) => it.collect(),
            // Tolerate missing normals: default to world up (Z-up, DESIGN.md §8).
            None => vec![[0.0, 0.0, 1.0]; vcount],
        };
        let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
            Some(t) => t.into_f32().collect(),
            None => vec![[0.0, 0.0]; vcount],
        };
        let joints_in: Vec<[u16; 4]> = match reader.read_joints(0) {
            Some(j) => j.into_u16().collect(),
            None => vec![[0, 0, 0, 0]; vcount],
        };
        let weights_in: Vec<[f32; 4]> = match reader.read_weights(0) {
            Some(w) => w.into_f32().collect(),
            None => vec![[1.0, 0.0, 0.0, 0.0]; vcount],
        };

        let base_vertex = vertices.len() as u32;
        for i in 0..vcount {
            let j = joints_in.get(i).copied().unwrap_or([0, 0, 0, 0]);
            let remapped = |x: u16| remap.get(x as usize).copied().unwrap_or(0) as u32;
            vertices.push(SkinnedVertex {
                pos: positions[i],
                normal: normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]),
                uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                joints: [remapped(j[0]), remapped(j[1]), remapped(j[2]), remapped(j[3])],
                weights: weights_in.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]),
            });
        }

        // Indices: use the primitive's own, or a trivial 0..vcount fan.
        let prim_indices: Vec<u32> = match reader.read_indices() {
            Some(it) => it.into_u32().collect(),
            None => (0..vcount as u32).collect(),
        };
        let index_start = indices.len() as u32;
        indices.extend(prim_indices.iter().map(|&i| i + base_vertex));
        submeshes.push(Submesh {
            material: 0, // ImportedModel carries no material table; index 0 is fine.
            index_start,
            index_count: prim_indices.len() as u32,
        });
    }

    if vertices.is_empty() {
        bail!("glTF mesh had no readable vertex positions");
    }

    let bounds = Aabb::from_points(vertices.iter().map(|v| Vec3::from(v.pos)));
    Ok(MeshData { vertices, indices, submeshes, bounds })
}

/// Decode all glTF animations into clips, mapping node targets to joint indices
/// (via `node_to_joint` then `remap`). Channels targeting non-joint nodes or
/// carrying morph weights are skipped (DESIGN.md §17).
fn read_clips(
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    node_to_joint: &[usize],
    remap: &[usize],
) -> Vec<AnimationClip> {
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|d| d.0.as_slice());

    let mut clips = Vec::new();
    for (ai, anim) in doc.animations().enumerate() {
        let mut channels = Vec::new();
        let mut duration = 0.0f32;

        for channel in anim.channels() {
            let node = channel.target().node().index();
            let Some(&gj) = node_to_joint.get(node) else { continue };
            if gj == usize::MAX {
                continue; // target is not a skin joint
            }
            let target_joint = remap[gj];

            let reader = channel.reader(get);
            let Some(times) = reader.read_inputs() else { continue };
            let times: Vec<f32> = times.collect();
            if times.is_empty() {
                continue;
            }

            use gltf::animation::util::ReadOutputs;
            let (kind, values): (ChannelKind, Vec<Vec4>) = match reader.read_outputs() {
                Some(ReadOutputs::Translations(it)) => (
                    ChannelKind::Translation,
                    it.map(|t| Vec4::new(t[0], t[1], t[2], 0.0)).collect(),
                ),
                Some(ReadOutputs::Rotations(r)) => (
                    ChannelKind::Rotation,
                    r.into_f32().map(|q| Vec4::new(q[0], q[1], q[2], q[3])).collect(),
                ),
                Some(ReadOutputs::Scales(it)) => (
                    ChannelKind::Scale,
                    it.map(|s| Vec4::new(s[0], s[1], s[2], 0.0)).collect(),
                ),
                _ => continue, // morph weights: unsupported here
            };

            if let Some(&last) = times.last() {
                duration = duration.max(last);
            }
            channels.push(Channel { target_joint, kind, times, values });
        }

        clips.push(AnimationClip {
            name: anim.name().map(str::to_string).unwrap_or_else(|| format!("clip_{ai}")),
            duration,
            channels,
        });
    }
    clips
}

/// Extract the base color texture of `mesh`'s first material, if present.
fn read_base_texture(mesh: &gltf::Mesh, images: &[gltf::image::Data]) -> Option<TextureData> {
    let prim = mesh.primitives().next()?;
    let info = prim.material().pbr_metallic_roughness().base_color_texture()?;
    let source = info.texture().source().index();
    let img = images.get(source)?;
    image_to_texture(img)
}

/// Convert a glTF decoded image into an RGBA8 [`TextureData`]. Unusual pixel
/// formats are skipped (returns `None`).
fn image_to_texture(img: &gltf::image::Data) -> Option<TextureData> {
    use gltf::image::Format;
    let (w, h) = (img.width, img.height);
    let px = &img.pixels;
    let rgba: Vec<u8> = match img.format {
        Format::R8G8B8A8 => px.clone(),
        Format::R8G8B8 => {
            let mut out = Vec::with_capacity(px.len() / 3 * 4);
            for c in px.chunks_exact(3) {
                out.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            out
        }
        Format::R8G8 => {
            let mut out = Vec::with_capacity(px.len() / 2 * 4);
            for c in px.chunks_exact(2) {
                out.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
            out
        }
        Format::R8 => {
            let mut out = Vec::with_capacity(px.len() * 4);
            for &g in px.iter() {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        _ => return None,
    };
    Some(TextureData::from_rgba(w, h, rgba))
}

// ---------------------------------------------------------------------------
// Static decode
// ---------------------------------------------------------------------------

fn build_static(
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
) -> Result<MeshData<StaticVertex>> {
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|d| d.0.as_slice());
    let mesh = doc.meshes().next().context("glTF has no mesh")?;

    let mut vertices: Vec<StaticVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut submeshes: Vec<Submesh> = Vec::new();

    for prim in mesh.primitives() {
        let reader = prim.reader(get);
        let Some(positions) = reader.read_positions() else {
            continue;
        };
        let positions: Vec<[f32; 3]> = positions.collect();
        let vcount = positions.len();
        let normals: Vec<[f32; 3]> = match reader.read_normals() {
            Some(it) => it.collect(),
            None => vec![[0.0, 0.0, 1.0]; vcount],
        };
        let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
            Some(t) => t.into_f32().collect(),
            None => vec![[0.0, 0.0]; vcount],
        };

        let base_vertex = vertices.len() as u32;
        for i in 0..vcount {
            vertices.push(StaticVertex {
                pos: positions[i],
                normal: normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]),
                uv: uvs.get(i).copied().unwrap_or([0.0, 0.0]),
            });
        }

        let prim_indices: Vec<u32> = match reader.read_indices() {
            Some(it) => it.into_u32().collect(),
            None => (0..vcount as u32).collect(),
        };
        let index_start = indices.len() as u32;
        indices.extend(prim_indices.iter().map(|&i| i + base_vertex));
        submeshes.push(Submesh {
            material: 0,
            index_start,
            index_count: prim_indices.len() as u32,
        });
    }

    if vertices.is_empty() {
        bail!("static glTF mesh had no readable vertex positions");
    }

    let bounds = Aabb::from_points(vertices.iter().map(|v| Vec3::from(v.pos)));
    Ok(MeshData { vertices, indices, submeshes, bounds })
}
