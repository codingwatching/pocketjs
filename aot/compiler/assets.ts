// aot/compiler/assets.ts — Stage: parse declared tilesets/sprites into
// target-NEUTRAL pixel data (palette-index grids + RGB palettes). Replaces the
// GBA-only half of the old bake.ts; each target backend encodes these grids
// into its native tile format at lowering time.

import type { Ctx, Rgb } from "./context.ts";
import type { Registry } from "../dsl/index.ts";

export function parseRows(rows: readonly string[], w: number, h: number): number[] {
  if (rows.length !== h) throw new Error(`tile grid: expected ${h} rows, got ${rows.length}`);
  const px: number[] = [];
  for (let y = 0; y < h; y++) {
    const r = rows[y];
    if (r.length !== w) throw new Error(`tile grid row ${y}: expected ${w} cols, got ${r.length} ("${r}")`);
    for (let x = 0; x < w; x++) px.push(parseInt(r[x], 16) & 0xf);
  }
  return px;
}

export function collectAssets(ctx: Ctx, registry: Registry): void {
  const game = registry.game!;
  // v1: every map shares one tileset.
  const tilesetNames = new Set(registry.maps.map((m) => m.tileset));
  if (tilesetNames.size !== 1) {
    throw new Error(`v1 supports one tileset per game (found: ${[...tilesetNames].join(", ")})`);
  }
  const tileset = registry.tilesets.get([...tilesetNames][0]);
  if (!tileset) throw new Error(`tileset "${[...tilesetNames][0]}" not defined`);

  // --- BG palette bank 0 (RGB; targets quantize as needed) ---
  ctx.bgPaletteRgb = tileset.palette.slice(0, 16).map((c) => [c[0], c[1], c[2]] as Rgb);

  // --- BG tiles: blank + tileset tiles (neutral index grids) ---
  ctx.bgTilePx.push(new Array(64).fill(0)); // id 0 blank
  ctx.tileSolid.push(false);
  for (const [name, decl] of Object.entries(tileset.tiles)) {
    ctx.tileNameToId.set(name, ctx.bgTilePx.length);
    ctx.bgTilePx.push(parseRows(decl.px, 8, 8));
    ctx.tileSolid.push(!!decl.solid);
  }

  // --- sprites: 16x16 frame grids, dir-major (down,up,left,right) ---
  let spriteIdx = 0;
  let frameBase = 0;
  for (const [name, decl] of registry.sprites) {
    const [w, h] = decl.size;
    if (w !== 16 || h !== 16) throw new Error(`v1 sprites must be 16x16 ("${name}" is ${w}x${h})`);
    if (spriteIdx >= 16) throw new Error(`v1 supports at most 16 sprites; "${name}" is #${spriteIdx}`);
    const dirs = ["down", "up", "left", "right"] as const;
    const frames = decl.facings.down.length;
    const grids: number[][] = [];
    for (const d of dirs) {
      const fr = decl.facings[d];
      if (fr.length !== frames) throw new Error(`sprite "${name}" facing ${d}: frame count mismatch`);
      for (const frame of fr) grids.push(parseRows(frame, 16, 16));
    }
    ctx.spriteFrames16.push(grids);
    ctx.spriteProtos.push({
      name,
      id: spriteIdx,
      w,
      h,
      palbank: spriteIdx, // GBA: one OBJ palette bank per sprite
      frames,
      tileBase: frameBase, // FRAME-block base; targets scale to tile units
      palette: decl.palette.slice(0, 16).map((c) => [c[0], c[1], c[2]] as Rgb),
    });
    ctx.spriteIds.set(name, spriteIdx);
    frameBase += grids.length;
    spriteIdx++;
  }

  // Pre-seed declared flags/items/battles/vars so ids are stable.
  (game.flags ?? []).forEach((f) => ctx.flagId(f));
  (game.vars ?? []).forEach((v) => ctx.varIdOf(v));
  (game.items ?? []).forEach((it) => ctx.items.intern(it));
  (game.battles ?? []).forEach((b) => ctx.battles.intern(b));
}
