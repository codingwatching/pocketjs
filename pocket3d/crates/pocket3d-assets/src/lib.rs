//! `pocket3d-assets` — the asset pipeline (DESIGN.md §9) and skinned-animation
//! import (DESIGN.md §17), plus procedural project-owned (CC0) placeholder
//! content (DESIGN.md §11).
//!
//! This crate is the one place that touches asset *formats*: it defines the
//! `.p3dpak` archive container, decodes glTF/GLB into the runtime types owned
//! by [`pocket3d_core`] / [`pocket3d_anim`], and synthesizes the demo bot and
//! weapon so the repository ships **no proprietary assets**.
//!
//! * [`pak`] — the hash-validated `.p3dpak` archive format.
//! * [`import`] — glTF/GLB → [`ImportedModel`] / [`MeshData`] decoding.
//! * [`procedural`] — CC0 bot + weapon, both as `.glb` and as in-memory types.

pub mod glb;
pub mod import;
pub mod pak;
pub mod procedural;

pub use import::{
    import_glb, import_gltf, import_slice, import_static_glb, ImportedModel,
};
pub use pak::{crc32, AssetKind, Pak, PakEntry, PakWriter};
pub use procedural::{
    procedural_bot, procedural_mesh, procedural_skeleton, write_bot_glb, write_weapon_glb,
};

use pocket3d_bsp::BspWorldAsset;
use pocket3d_render::WorldUpload;

/// Borrow a compiled [`BspWorldAsset`]'s render-ready pieces as a
/// [`pocket3d_render::WorldUpload`], ready to hand to
/// [`pocket3d_render::RenderDevice::upload_world`] (DESIGN.md §9/§12).
///
/// The lightmap atlas is passed as `Some` only when the map actually carried
/// lighting (a non-empty atlas); otherwise `None`.
pub fn world_upload(bsp: &BspWorldAsset) -> WorldUpload<'_> {
    let lightmap_atlas = if bsp.lightmap_atlas.width > 0 && bsp.lightmap_atlas.height > 0 {
        Some(&bsp.lightmap_atlas)
    } else {
        None
    };
    WorldUpload {
        mesh: &bsp.mesh,
        materials: &bsp.materials,
        textures: &bsp.textures,
        lightmap_atlas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A unique scratch path under the OS temp dir, to keep parallel tests from
    /// colliding.
    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pocket3d_{tag}_{}_{nanos}.glb", std::process::id()))
    }

    /// (DESIGN.md §9) Round-trip three blobs of different kinds through a pak,
    /// read each back byte-identical, and confirm corruption fails CRC.
    #[test]
    fn pak_round_trip_and_crc() {
        let world = b"WORLD-blob-bytes-0123".to_vec();
        let mesh = b"a completely different mesh payload".to_vec();
        let tex = vec![0u8, 255, 128, 64, 32, 16, 8, 4, 2, 1];

        let mut w = PakWriter::new();
        w.add("maps/arena", AssetKind::World, &world);
        w.add("mesh/bot", AssetKind::Mesh, &mesh);
        w.add("tex/wall", AssetKind::Texture, &tex);
        let bytes = w.write_to_vec();

        let pak = Pak::parse(bytes.clone()).expect("parse");
        assert_eq!(pak.entries().len(), 3);
        assert!(pak.contains("maps/arena"));
        assert!(!pak.contains("nope"));

        // Byte-identical read-back.
        assert_eq!(pak.get("maps/arena").unwrap(), &world[..]);
        assert_eq!(pak.get("mesh/bot").unwrap(), &mesh[..]);
        assert_eq!(pak.get("tex/wall").unwrap(), &tex[..]);

        // Kinds preserved.
        let kinds: Vec<_> = pak.entries().iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![AssetKind::World, AssetKind::Mesh, AssetKind::Texture]);
        assert!(pak.verify().is_ok());

        // Corrupt one byte inside the first blob and confirm CRC verification
        // fails: `get` returns None, `get_checked` errors, `verify` errors.
        let mut corrupt = bytes.clone();
        let off = pak.entries()[0].offset as usize;
        corrupt[off] ^= 0xFF;
        let bad = Pak::parse(corrupt).expect("still structurally valid");
        assert!(bad.get("maps/arena").is_none(), "CRC mismatch must yield None");
        assert!(bad.get_checked("maps/arena").is_err());
        assert!(bad.verify().is_err());
        // Untouched blobs still read fine.
        assert_eq!(bad.get("mesh/bot").unwrap(), &mesh[..]);
    }

    /// The CRC-32 implementation must match the well-known IEEE check value:
    /// `crc32("123456789") == 0xCBF43926`.
    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    /// Assert a decoded bot has the shape the whole skinned pipeline promises.
    fn assert_bot_shape(m: &ImportedModel) {
        assert!(
            m.skeleton.joint_count() >= 6,
            "expected >= 6 joints, got {}",
            m.skeleton.joint_count()
        );
        assert_eq!(m.clips.len(), 3, "expected 3 clips");
        let names: Vec<&str> = m.clips.iter().map(|c| c.name.as_str()).collect();
        for want in ["Idle", "Walk", "Death"] {
            assert!(names.contains(&want), "missing clip {want}; have {names:?}");
        }
        for c in &m.clips {
            assert!(c.duration > 0.0, "clip {} has zero duration", c.name);
        }
        assert!(!m.skinned_mesh.vertices.is_empty(), "bot mesh has no vertices");
        for (i, v) in m.skinned_mesh.vertices.iter().enumerate() {
            let sum: f32 = v.weights.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-3,
                "vertex {i} weights sum to {sum}, expected ~1.0"
            );
        }
    }

    /// CRITICAL (DESIGN.md §9/§11/§17): the full skinned pipeline works —
    /// `write_bot_glb` produces a `.glb` that `import_glb` decodes into a
    /// well-formed rigged, animated bot.
    #[test]
    fn bot_glb_round_trip() {
        let path = tmp_path("bot");
        write_bot_glb(&path).expect("write bot glb");
        let model = import_glb(&path).expect("import bot glb");
        assert_bot_shape(&model);

        // The bind-pose skinning palette should be (near) identity, proving the
        // inverse bind matrices survived the round-trip (DESIGN.md §17).
        let palette = pocket3d_anim::compute_joint_matrices(
            &model.skeleton,
            &model.skeleton.bind_pose(),
        );
        for m in &palette {
            assert!(m.abs_diff_eq(glam::Mat4::IDENTITY, 1e-3), "non-identity bind palette");
        }
        let _ = std::fs::remove_file(&path);
    }

    /// The in-memory fallback bot must have the identical shape (DESIGN.md §11).
    #[test]
    fn procedural_bot_shape() {
        let model = procedural_bot();
        assert_bot_shape(&model);

        // Same joint count as the GLB path (parity check).
        assert_eq!(model.skeleton.joint_count(), 7);
        // Bind palette identity check for the directly-built skeleton too.
        let palette = pocket3d_anim::compute_joint_matrices(
            &model.skeleton,
            &model.skeleton.bind_pose(),
        );
        for m in &palette {
            assert!(m.abs_diff_eq(glam::Mat4::IDENTITY, 1e-3));
        }
    }

    /// The static weapon `.glb` round-trips through `import_static_glb`
    /// (DESIGN.md §11).
    #[test]
    fn weapon_glb_round_trip() {
        let path = tmp_path("weapon");
        write_weapon_glb(&path).expect("write weapon glb");
        let mesh = import_static_glb(&path).expect("import weapon glb");
        assert!(!mesh.vertices.is_empty());
        assert!(mesh.triangle_count() > 0);
        assert!(mesh.bounds.is_valid());
        let _ = std::fs::remove_file(&path);
    }
}
