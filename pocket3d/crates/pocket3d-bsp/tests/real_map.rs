//! Integration test against a real GoldSrc map, if one is staged locally.
//!
//! The map/WADs are dev-only and gitignored (DESIGN.md §11), so this test skips
//! gracefully when they are absent. Run with the maps staged under
//! `examples/openstrike/maps` + `examples/openstrike/assets/wads`.

use pocket3d_bsp::{load_bsp, Team};
use std::path::PathBuf;

fn map_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/openstrike/maps/de_dust2.bsp")
}

fn wad_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/openstrike/assets/wads")
}

#[test]
fn loads_real_dust2() {
    let path = map_path();
    if !path.exists() {
        eprintln!("skipping: {} not staged", path.display());
        return;
    }
    let asset = load_bsp(&path, &[wad_dir()]).expect("load de_dust2");

    // Header + counts validated against direct byte inspection of the file.
    assert_eq!(asset.info.version, 30, "GoldSrc BSP v30");
    assert_eq!(asset.info.texture_count, 44, "de_dust2 has 44 textures");
    assert!(asset.info.world_triangles > 5000, "world mesh generated");
    assert!(
        asset.info.collision_triangles > 5000,
        "collision generated"
    );

    // Spawns: 20 CT (info_player_start) + 20 T (info_player_deathmatch).
    let ct = asset.spawns_for(Team::Ct).count();
    let t = asset.spawns_for(Team::T).count();
    assert_eq!(ct, 20, "20 CT spawns");
    assert_eq!(t, 20, "20 T spawns");

    // Entities preserved verbatim, including bomb targets / buy zones.
    assert!(asset
        .entities
        .iter()
        .any(|e| e.class_name == "func_bomb_target"));
    assert!(asset
        .entities
        .iter()
        .any(|e| e.class_name == "func_buyzone"));

    // Lightmap atlas was produced.
    assert!(asset.lightmap_atlas.width >= 1024);
    assert!(asset.lightmap_atlas.height >= 1);

    // Most textures should resolve from the staged WADs.
    let missing = asset.info.missing_textures.len();
    assert!(
        missing < asset.info.texture_count,
        "at least some textures resolved ({} missing of {})",
        missing,
        asset.info.texture_count
    );

    eprintln!(
        "de_dust2: {} verts, {} tris, {} coll tris, atlas {}x{}, {} missing textures, bounds {:?}",
        asset.info.world_vertices,
        asset.info.world_triangles,
        asset.info.collision_triangles,
        asset.lightmap_atlas.width,
        asset.lightmap_atlas.height,
        missing,
        asset.bounds,
    );
}
