// aot/compiler/context.ts — shared compile state (interned banks + id maps)
// threaded through assets -> script -> model -> target lowering.
//
// Pixel data is stored target-NEUTRALLY here (palette-index grids + RGB
// palettes); each target backend encodes it into its native tile format
// (GBA 4bpp / GB interleaved 2bpp / NES planar 2bpp) at lowering time.

import { TARGETS, type TargetName, type TargetSpec } from "../spec/pjgb.ts";

class NameInterner {
  private m = new globalThis.Map<string, number>();
  private _list: string[] = [];
  intern(name: string): number {
    let id = this.m.get(name);
    if (id === undefined) {
      id = this._list.length;
      this.m.set(name, id);
      this._list.push(name);
    }
    return id;
  }
  get(name: string): number | undefined {
    return this.m.get(name);
  }
  list(): readonly string[] {
    return this._list;
  }
  get size(): number {
    return this._list.length;
  }
}

export type Rgb = readonly [number, number, number];

/** A target-neutral 8x8 tile: 64 palette indices (0-15), row-major. */
export type TilePx = number[];

export interface SpriteProto {
  name: string;
  id: number;
  w: number;
  h: number;
  palbank: number;
  frames: number;
  tileBase: number; // frame-block index (4 8x8 tiles per 16x16 frame)
  palette: Rgb[]; // sprite's own 16-color palette (index 0 = transparent)
}

export interface ScriptOut {
  id: number;
  name: string;
  bytecode: number[];
}

/** A WARP operand awaiting map-index resolution (patched after buildModel). */
export interface WarpFixup {
  scriptId: number;
  /** Offset of the OP_WARP operand block within the script's bytecode. */
  at: number;
  dest: string; // "map:entrance"
}

export class Ctx {
  readonly target: TargetSpec;

  constructor(target: TargetName = "gba") {
    this.target = TARGETS[target];
  }

  texts = new NameInterner();
  flags = new NameInterner();
  vars = new NameInterner();
  items = new NameInterner();
  battles = new NameInterner();

  // filled by assets.ts (target-neutral)
  bgPaletteRgb: Rgb[] = []; // 16 entries, bank 0 (map tileset)
  bgTilePx: TilePx[] = []; // blank + tileset tiles (indices into bgPaletteRgb)
  tileNameToId = new globalThis.Map<string, number>();
  tileSolid: boolean[] = []; // parallel to bgTilePx
  spriteFramePx: TilePx[][] = []; // per sprite: 16x16 frames as 4-tile blocks? no — raw 16x16 grids
  spriteFrames16: number[][][] = []; // [sprite][frame(dir-major)] -> 256 px indices
  spriteProtos: SpriteProto[] = [];
  spriteIds = new globalThis.Map<string, number>();

  // cjk16 glyph interning (dense fullwidth glyph ids, in first-use order)
  fullGlyphs = new NameInterner();
  fullGlyphId = (ch: string): number => this.fullGlyphs.intern(ch);

  // legacy ascii8 (GBA) font/box tile ids, filled by the GBA backend
  fontBase = 0;
  boxTile = 0;
  glyphSlotBase = 0;

  // filled by model
  mapIndex = new globalThis.Map<string, number>();

  // filled by script (+ synthetic sign scripts)
  scripts: ScriptOut[] = [];
  warpFixups: WarpFixup[] = [];

  internText(s: string): number {
    return this.texts.intern(s);
  }
  flagId(name: string): number {
    return this.flags.intern(name);
  }
  varIdOf(name: string): number {
    return this.vars.intern(name);
  }
  spriteId(name: string): number {
    const id = this.spriteIds.get(name);
    if (id === undefined) throw new Error(`unknown sprite "${name}"`);
    return id;
  }
  /** Allocate the next script id (dense, appended after AST scripts). */
  addScript(name: string, bytecode: number[]): number {
    const id = this.scripts.length;
    this.scripts.push({ id, name, bytecode });
    return id;
  }
}
