// playset/modules/world/environment/terrain-mesh-factory.ts — bakes a
// terrain sampler into a scene3d heightfield mesh and registers the sampler
// as the CollisionWorld ground authority.
//
// Ported from GameBlocks (github.com/xt4d/GameBlocks, MIT © 2026 Weihao
// Cheng) — modules/world/environment/TerrainMeshFactory.js. Deliberate
// changes for the scene3d surface:
//   - The original built an indexed (segments+1)² BufferGeometry; the same
//     regular grid maps directly onto geomHeightfield(size, size, cols,
//     rows, heights, colors) — the host owns tessellation/normals, so the
//     index buffer and computeVertexNormals are gone. Heights are sampled in
//     the original's row/col order (forward, then right, both from -size/2).
//   - Heightfields live in world axes (+Y up); non-default bases would need
//     a node rotation — GameBlocks only ever uses the default basis here.
//   - materialOptions (roughness/metalness) has no fixed-function analog and
//     is accepted but ignored; the material is vertex-colored.
//   - createTerrainTrimeshCollider(world, rapier, mesh) is replaced by
//     registerTerrainCollider(world, sampler): the CollisionWorld's ground
//     authority is the SAMPLER (world.setTerrain), not a trimesh — exact
//     heights instead of triangle interpolation, and no Rapier. It returns
//     nothing (there is no body/collider pair to hand back).

import { MAT, type Scene3D, type SceneNode } from "../../../scene3d/client.ts";
import type { CollisionWorld, TerrainLike } from "../../physics/collision-world.ts";
import type { WorldBasis } from "../../math/world-basis.ts";

/** spec ABGR byte order: (a<<24)|(b<<16)|(g<<8)|r. Local on purpose. */
function rgbToAbgr(hex: number, alpha = 255): number {
  const r = (hex >> 16) & 255;
  const g = (hex >> 8) & 255;
  const b = hex & 255;
  return (((alpha & 255) << 24) | (b << 16) | (g << 8) | r) >>> 0;
}

/** What createTerrainMesh needs from a sampler (all three samplers qualify). */
export interface MeshTerrainSampler {
  basis?: WorldBasis;
  sample(
    right: number,
    forward: number,
  ): { height: number; color?: { r: number; g: number; b: number } | null } | null | undefined;
  noise2D?(right: number, forward: number, seedOffset?: number): number;
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function defaultTerrainColor(height = 0, colorNoise = 0): { r: number; g: number; b: number } {
  const grassMix = clamp01((height + 4) / 12);
  const ridgeMix = clamp01((height - 8) / 18);
  return {
    r: 0.22 + grassMix * 0.1 + ridgeMix * 0.16 + colorNoise,
    g: 0.32 + grassMix * 0.18 - ridgeMix * 0.02 + colorNoise * 0.6,
    b: 0.16 + grassMix * 0.08 + ridgeMix * 0.09 + colorNoise * 0.35,
  };
}

export interface CreateTerrainMeshOptions {
  scene: Scene3D;
  terrainSampler: MeshTerrainSampler;
  size?: number;
  segments?: number;
  /** Accepted for API compatibility; ignored (see header). */
  materialOptions?: Record<string, unknown>;
}

export function createTerrainMesh({
  scene,
  terrainSampler,
  size = 184,
  segments = 220,
  materialOptions = {},
}: CreateTerrainMeshOptions): SceneNode {
  void materialOptions;
  if (!terrainSampler || typeof terrainSampler.sample !== "function") {
    throw new Error("createTerrainMesh: terrainSampler.sample(right, forward) is required");
  }

  const safeSize = Math.max(0.001, size);
  const safeSegments = Math.max(1, Math.floor(segments));
  const vertexSide = safeSegments + 1;
  const vertexCount = vertexSide * vertexSide;
  const heights = new Float32Array(vertexCount);
  const colors = new Float32Array(vertexCount * 3);
  const halfSize = safeSize * 0.5;
  const step = safeSize / safeSegments;

  for (let row = 0; row <= safeSegments; row += 1) {
    for (let col = 0; col <= safeSegments; col += 1) {
      const i = row * vertexSide + col;
      const right = -halfSize + col * step;
      const forward = -halfSize + row * step;
      const sample = terrainSampler.sample(right, forward) ?? { height: 0 };
      const height = sample.height;
      heights[i] = height;

      let colorValue = "color" in sample ? sample.color : null;
      if (!colorValue) {
        const colorNoise = typeof terrainSampler.noise2D === "function"
          ? terrainSampler.noise2D(right * 0.21 + 13, forward * 0.21 - 5, 103) * 0.08
          : 0;
        colorValue = defaultTerrainColor(height, colorNoise);
      }

      colors[i * 3 + 0] = colorValue.r;
      colors[i * 3 + 1] = colorValue.g;
      colors[i * 3 + 2] = colorValue.b;
    }
  }

  const geomId = scene.heightfield(safeSize, safeSize, vertexSide, vertexSide, heights, colors);
  const matId = scene.material(rgbToAbgr(0xffffff), MAT.vertexColors);
  return scene.mesh(geomId, matId);
}

/**
 * The CollisionWorld replacement for the Rapier terrain trimesh: the sampler
 * itself becomes the ground authority (world.setTerrain). Friction and
 * restitution have no CollisionWorld analog.
 */
export function registerTerrainCollider(world: CollisionWorld, sampler: TerrainLike): void {
  if (!world) {
    throw new Error("registerTerrainCollider: a CollisionWorld is required");
  }
  if (!sampler || typeof sampler.heightAt !== "function") {
    throw new Error("registerTerrainCollider: sampler.heightAt(right, forward) is required");
  }
  world.setTerrain(sampler);
}
