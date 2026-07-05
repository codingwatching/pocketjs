# Pocket3D / OpenStrike

A Rust-first 3D runtime for constrained, code-driven games — and **OpenStrike**,
its first application: a BSP-based single-player FPS vertical slice.

Pocket3D is an **independent runtime** in the Pocket repository. It does **not**
depend on the PocketJS 2D/PSP core; it has its own rendering (`wgpu`), asset,
simulation, physics, animation, input, and scripting layers. See
[`DESIGN.md`](DESIGN.md) for the full design.

> This is its own Cargo workspace on purpose — it must never be folded into a
> repo-root workspace, because the PocketJS PSP EBOOT crate must live outside
> any workspace.

## Crates

| Crate | Role |
| --- | --- |
| `pocket3d-core` | Math (Z-up), geometry, entity arena, handles, time, input, camera, events, shared mesh/texture/material payloads |
| `pocket3d-app` | Application lifecycle contract (`Pocket3dApp` + contexts) |
| `pocket3d-render` | Backend-agnostic renderer contract: `SceneView`, `DebugDraw`, `RenderDevice` |
| `pocket3d-render-wgpu` | The first render backend (`wgpu` + `winit`), display-gated |
| `pocket3d-bsp` | GoldSrc **BSP v30** + **WAD3** loader → world mesh, lightmap atlas, collision, entities |
| `pocket3d-physics` | Triangle **BVH**: raycasts + capsule-contact queries (query-only) |
| `pocket3d-kcc` | `CharacterController` trait + capsule **move-and-slide** KCC |
| `pocket3d-anim` | Skeletons, clips, pose evaluation, crossfade state machine, joint matrices |
| `pocket3d-assets` | `.p3dpak` archive, glTF/GLB import, procedural CC0 bot/weapon |
| `pocket3d-audio` | `AudioBackend` trait + null/recording backends |
| `pocket3d-script` | QuickJS (`rquickjs`) config + event-callback bridge |
| `pocket3d-tools` | The `p3d` CLI |
| `examples/openstrike` | The FPS: player, hitscan weapon, waypoint bots, round loop |

## Coordinate system

Z-up, right-handed: `+X` right, `+Y` forward, `+Z` up, `1 unit = 1 BSP unit`.

## Build & test

```sh
cd pocket3d
cargo test                      # entire headless-testable runtime (skips the wgpu backend)
cargo build -p pocket3d-render-wgpu   # the graphics backend (heavy; needs a GPU to *run*)
```

The default build/test path deliberately excludes the `wgpu` backend so the
whole gameplay runtime is verifiable without a GPU.

## Maps & assets policy

The repository ships **no** proprietary Counter-Strike / Valve content
(DESIGN.md §11). GoldSrc `.bsp`/`.wad` files are **dev-only** and gitignored.
Stage them locally for development:

```sh
# maps/ and assets/wads/ are gitignored
cp /path/to/de_dust2.bsp   examples/openstrike/maps/
cp /path/to/*.wad          examples/openstrike/assets/wads/
```

The bot and weapon models (`examples/openstrike/assets/models/*.glb`) are
**project-owned, procedurally generated** and safe to commit.

## The `p3d` tool

```sh
p3d bsp inspect  <map.bsp> --wad-path <dir>          # version, lumps, textures, entities, spawns
p3d bsp build    <map.bsp> --wad-path <dir> --out <map.p3dworld>
p3d asset build  <asset-dir> --out <game.p3dpak>
p3d gen-bot      --out bot.glb                        # regenerate the CC0 bot
p3d gen-weapon   --out weapon.glb
```

## Running OpenStrike

Headless (deterministic, no GPU — drives the full round loop with an autopilot):

```sh
cargo run -p openstrike -- sim --script examples/openstrike/scripts/openstrike.js
cargo run -p openstrike -- check-assets
```

Windowed (requires a display + GPU):

```sh
cargo run -p openstrike --features window -- run
```

Controls (windowed): **WASD** move, **mouse** look, **space** jump,
**left-click** fire, **F1/F3** debug overlays, **Esc** release mouse.

## Status

Milestones 0–6 of the design are implemented. The BSP path, physics/KCC,
weapon hit detection, waypoint bots, round loop, and scripting are verified
headlessly against a real `de_dust2.bsp`. The `wgpu` renderer and windowed
runtime compile; running them requires a display, so on-screen rendering is
unverified in headless CI (analogous to the PocketJS PSP-hardware path).
