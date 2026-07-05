//! `p3d` — Pocket3D command-line tools (DESIGN.md §23).
//!
//! ```text
//! p3d bsp inspect <map.bsp> [--wad-path <dir>]
//! p3d bsp build   <map.bsp> --wad-path <dir> --out <map.p3dworld>   (feature: pipeline)
//! p3d asset build <asset-dir> --out <game.p3dpak>                   (feature: pipeline)
//! p3d gen-bot     --out <bot.glb>                                   (feature: pipeline)
//! p3d gen-weapon  --out <weapon.glb>                                (feature: pipeline)
//! ```
//! (`openstrike run` / `check-assets` live on the `openstrike` binary.)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "p3d", about = "Pocket3D tools")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// BSP inspection and compilation.
    #[command(subcommand)]
    Bsp(BspCmd),
    /// Build a .p3dpak archive from a directory of assets.
    #[cfg(feature = "pipeline")]
    AssetBuild {
        dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Generate the project-owned placeholder bot model (CC0) as a .glb.
    #[cfg(feature = "pipeline")]
    GenBot {
        #[arg(long)]
        out: PathBuf,
    },
    /// Generate the project-owned placeholder weapon viewmodel as a .glb.
    #[cfg(feature = "pipeline")]
    GenWeapon {
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum BspCmd {
    /// Print a BSP's version, lumps, textures, entities, spawns, etc.
    Inspect {
        map: PathBuf,
        #[arg(long)]
        wad_path: Vec<PathBuf>,
    },
    /// Compile a BSP into a .p3dworld pack (geometry + lightmap + collision).
    #[cfg(feature = "pipeline")]
    Build {
        map: PathBuf,
        #[arg(long)]
        wad_path: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Bsp(BspCmd::Inspect { map, wad_path }) => bsp_inspect(&map, &wad_path),
        #[cfg(feature = "pipeline")]
        Cmd::Bsp(BspCmd::Build { map, wad_path, out }) => pipeline::bsp_build(&map, &wad_path, &out),
        #[cfg(feature = "pipeline")]
        Cmd::AssetBuild { dir, out } => pipeline::asset_build(&dir, &out),
        #[cfg(feature = "pipeline")]
        Cmd::GenBot { out } => pipeline::gen_bot(&out),
        #[cfg(feature = "pipeline")]
        Cmd::GenWeapon { out } => pipeline::gen_weapon(&out),
    }
}

/// Print the required inspector output (DESIGN.md §23).
fn bsp_inspect(map: &PathBuf, wad_path: &[PathBuf]) -> Result<()> {
    let info = pocket3d_bsp::inspect(map, wad_path).context("inspecting BSP")?;
    println!("BSP: {}", map.display());
    println!("  version           : {}", info.version);
    println!("  lumps:");
    for l in &info.lumps {
        println!(
            "    {:<12} offset {:>9}  len {:>9}  count {:>7}",
            l.name, l.offset, l.length, l.count
        );
    }
    println!("  textures          : {}", info.texture_count);
    println!("  missing textures  : {}", info.missing_textures.len());
    for name in info.missing_textures.iter().take(16) {
        println!("      - {name}");
    }
    println!("  world vertices    : {}", info.world_vertices);
    println!("  world triangles   : {}", info.world_triangles);
    println!(
        "  lightmap atlas    : {}x{}",
        info.lightmap_atlas_size.0, info.lightmap_atlas_size.1
    );
    println!("  collision triangles: {}", info.collision_triangles);
    println!("  spawn points      : {}", info.spawn_count);
    println!(
        "  bounds            : {:?} .. {:?}",
        info.bounds.min, info.bounds.max
    );
    println!("  entities by classname:");
    for (class, n) in &info.entity_count_by_classname {
        println!("    {:>4}  {}", n, class);
    }
    Ok(())
}

#[cfg(feature = "pipeline")]
mod pipeline {
    use super::*;
    use pocket3d_assets::{AssetKind, PakWriter};

    /// Compile a BSP into a `.p3dworld` pak bundling its layers.
    pub fn bsp_build(map: &PathBuf, wad_path: &[PathBuf], out: &PathBuf) -> Result<()> {
        let asset = pocket3d_bsp::load_bsp(map, wad_path).context("loading BSP")?;
        let mut pak = PakWriter::new();

        pak.add(
            "world.mesh.vertices",
            AssetKind::World,
            world_vertex_bytes(&asset.mesh.vertices),
        );
        pak.add(
            "world.mesh.indices",
            AssetKind::World,
            u32_bytes(&asset.mesh.indices),
        );
        pak.add(
            "world.collision.positions",
            AssetKind::World,
            &vec3_bytes(&asset.collision.positions),
        );
        pak.add(
            "world.lightmap.rgba",
            AssetKind::Texture,
            &atlas_bytes(&asset.lightmap_atlas),
        );

        pak.write_to_file(out)
            .with_context(|| format!("writing {}", out.display()))?;
        println!(
            "Wrote {} ({} world tris, {} collision tris, atlas {}x{})",
            out.display(),
            asset.mesh.triangle_count(),
            asset.collision.triangle_count(),
            asset.lightmap_atlas.width,
            asset.lightmap_atlas.height,
        );
        Ok(())
    }

    /// Bundle a directory of files into a .p3dpak (DESIGN.md §9).
    pub fn asset_build(dir: &PathBuf, out: &PathBuf) -> Result<()> {
        let mut pak = PakWriter::new();
        let mut count = 0;
        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .flatten()
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let kind = kind_for(&name);
            let bytes = std::fs::read(&path)?;
            pak.add(&name, kind, &bytes);
            count += 1;
        }
        pak.write_to_file(out)?;
        println!("Packed {count} assets into {}", out.display());
        Ok(())
    }

    pub fn gen_bot(out: &PathBuf) -> Result<()> {
        pocket3d_assets::write_bot_glb(out).context("generating bot GLB")?;
        let model = pocket3d_assets::import_glb(out)?;
        println!(
            "Wrote {} ({} joints, {} clips, {} skinned verts)",
            out.display(),
            model.skeleton.joint_count(),
            model.clips.len(),
            model.skinned_mesh.vertices.len(),
        );
        Ok(())
    }

    pub fn gen_weapon(out: &PathBuf) -> Result<()> {
        pocket3d_assets::write_weapon_glb(out).context("generating weapon GLB")?;
        println!("Wrote {}", out.display());
        Ok(())
    }

    fn kind_for(name: &str) -> AssetKind {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".glb") || lower.ends_with(".gltf") {
            AssetKind::Skin
        } else if lower.ends_with(".png") {
            AssetKind::Texture
        } else if lower.ends_with(".p3dworld") {
            AssetKind::World
        } else {
            AssetKind::Raw
        }
    }

    fn world_vertex_bytes(v: &[pocket3d_core::WorldVertex]) -> &[u8] {
        bytemuck::cast_slice(v)
    }
    fn u32_bytes(v: &[u32]) -> &[u8] {
        bytemuck::cast_slice(v)
    }
    fn vec3_bytes(v: &[pocket3d_core::glam::Vec3]) -> Vec<u8> {
        let mut out = Vec::with_capacity(v.len() * 12);
        for p in v {
            out.extend_from_slice(&p.x.to_le_bytes());
            out.extend_from_slice(&p.y.to_le_bytes());
            out.extend_from_slice(&p.z.to_le_bytes());
        }
        out
    }
    fn atlas_bytes(t: &pocket3d_core::TextureData) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + t.rgba.len());
        out.extend_from_slice(&t.width.to_le_bytes());
        out.extend_from_slice(&t.height.to_le_bytes());
        out.extend_from_slice(&t.rgba);
        out
    }
}
