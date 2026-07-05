// aot/compiler/index.ts — the @pocketjs/aot compile pipeline (design §11),
// now target-parameterized (gba / gb / nes).
//   Source TS/TSX
//     -> evaluate (static JSX zone)   -> Registry + script ASTs
//     -> collectAssets                -> target-neutral tiles/sprites/palettes
//     -> script residualizer          -> bytecode (text pre-wrapped per target)
//     -> model                        -> concrete maps/actors/warps
//     -> warp fixups                  -> patch OP_WARP operands
//     -> target backend               -> ROM (see targets/*)

import { evaluateGame } from "./evaluate.ts";
import { Ctx } from "./context.ts";
import { collectAssets } from "./assets.ts";
import { compileScript, type TextMode } from "./script.ts";
import { buildModel, type GameModel } from "./model.ts";
import { DBG, TARGETS, type TargetName } from "../spec/pjgb.ts";
import type { GameDecl } from "../dsl/index.ts";

export interface CompileOutput {
  target: TargetName;
  mode: TextMode;
  ctx: Ctx;
  model: GameModel;
  game: GameDecl;
}

export function effectiveTextMode(game: GameDecl, target: TargetName): TextMode {
  if (target !== "gba") return "cjk16"; // 8-bit runtimes only implement cjk16
  return game.textMode ?? "ascii8";
}

export async function compile(entry: string, target: TargetName = "gba"): Promise<CompileOutput> {
  const ev = await evaluateGame(entry);
  const game = ev.registry.game!;
  const mode = effectiveTextMode(game, target);
  const ctx = new Ctx(target);
  collectAssets(ctx, ev.registry);

  // AST scripts occupy ids 0..N-1 (matching the ScriptRefs the actors carry).
  for (const site of ev.scripts) {
    const bc = compileScript(site, ctx, mode);
    const id = ctx.addScript(`script_${site.id}`, bc);
    if (id !== site.id) throw new Error(`internal: script id ${id} != site ${site.id}`);
  }

  const model = buildModel(ctx, ev.registry, mode); // sign scripts append at ids N+

  // Resolve warpTo("map:entrance") operands now that maps/entrances exist.
  for (const fx of ctx.warpFixups) {
    const [mapName, entName] = fx.dest.split(":");
    const m = model.maps.find((mm) => mm.name === mapName);
    if (!m) throw new Error(`warpTo: unknown map "${mapName}"`);
    const ent = m.entrances.get(entName ?? "spawn");
    if (!ent) throw new Error(`warpTo: map "${mapName}" has no entrance "${entName}"`);
    const bc = ctx.scripts[fx.scriptId].bytecode;
    bc[fx.at] = m.index & 0xff;
    bc[fx.at + 1] = ent.x & 0xff;
    bc[fx.at + 2] = (ent.x >> 8) & 0xff;
    bc[fx.at + 3] = ent.y & 0xff;
    bc[fx.at + 4] = (ent.y >> 8) & 0xff;
    bc[fx.at + 5] = ent.dir & 0xff;
  }

  return { target, mode, ctx, model, game };
}

/** Debug map for the emulator test harnesses: names -> ids/addresses. */
export function debugInfo(out: CompileOutput): unknown {
  const { ctx, model } = out;
  const debugAddr = TARGETS[out.target].debugAddr;
  const flags: Record<string, { id: number; byteAddr: number; bit: number }> = {};
  ctx.flags.list().forEach((name, id) => {
    flags[name] = { id, byteAddr: debugAddr + DBG.FLAGS + (id >> 3), bit: id & 7 };
  });
  const vars: Record<string, { id: number; addr: number }> = {};
  ctx.vars.list().forEach((name, id) => {
    vars[name] = { id, addr: debugAddr + DBG.VARS + id * 2 };
  });
  const maps: Record<string, number> = {};
  ctx.mapIndex.forEach((i, name) => (maps[name] = i));
  return {
    title: out.game.title,
    target: out.target,
    textMode: out.mode,
    start: model.start,
    debugAddr,
    fields: DBG,
    flags,
    vars,
    maps,
    texts: ctx.texts.list(),
    scripts: ctx.scripts.map((s) => ({ id: s.id, name: s.name, bytes: s.bytecode.length })),
    sprites: ctx.spriteProtos.map((s) => ({ name: s.name, id: s.id, frames: s.frames })),
    bgTiles: ctx.bgTilePx.length,
    fullGlyphs: ctx.fullGlyphs.size,
  };
}

/** Compact IR snapshot for `dist/game.ir.json` (design §11.8). */
export function irJson(out: CompileOutput): unknown {
  return {
    title: out.game.title,
    target: out.target,
    textMode: out.mode,
    start: out.model.start,
    maps: out.model.maps.map((m) => ({
      name: m.name,
      index: m.index,
      size: [m.w, m.h],
      onEnter: m.onEnter === 0xff ? null : m.onEnter,
      actors: m.actors.map((a) => ({ name: a.name, at: [a.x, a.y], sprite: a.spriteId, onTalk: a.onTalk })),
      warps: m.warps.map((w) => ({ at: [w.x, w.y], to: `${w.destMap}:${w.destEntrance}`, dest: [w.destMapIdx, w.destX, w.destY] })),
      entrances: [...m.entrances.entries()],
    })),
    scripts: out.ctx.scripts.map((s) => ({ id: s.id, name: s.name, bytes: s.bytecode.length })),
    texts: out.ctx.texts.list(),
    flags: out.ctx.flags.list(),
    glyphs: out.ctx.fullGlyphs.list().join(""),
  };
}
