// cine/compiler/index.ts — compileFilm(entry): evaluate the declaration zone,
// bake assets, residualize cues, and assemble the CompiledFilm model that
// emit.ts turns into gen_data.c.

import { dirname, resolve } from "node:path";
import { evaluateFilm } from "./evaluate.ts";
import { residualizeCue, type CueCtx } from "./residualize.ts";
import { TextBank } from "./text.ts";
import {
  loadPng, quantize, tileLayer, buildMap, tileObjSheet, gradientTable,
  uiPalette, uiBgTiles, uiObjTiles, type Quantized,
} from "./assets.ts";
import {
  FARSKY_BASE, FARSKY_MAX, MAIN_TILE_MAX, GLYPH_SLOTS,
  PALBANK_SKY, PALBANK_FAR, PALBANK_MAIN, PALBANK_UI, PALBANK_OBJ_UI,
  MAX_SPRITES, MAX_SCENES, hex555, UI_INK, UI_BOX,
} from "../spec/cine.ts";
import type { ActorDecl, LayerDecl, SceneDecl } from "../dsl/index.ts";

export interface CompiledProto {
  tileBase: number;
  w: number;
  h: number;
  frames: number;
  palbank: number;
  fps: number;
}

export interface CompiledScene {
  id: string;
  palBg: Uint16Array; // 256
  palObj: Uint16Array; // 256
  tilesMain: Uint8Array;
  nMain: number;
  tilesShared: Uint8Array;
  nShared: number;
  mapMain: Uint16Array;
  mapFar: Uint16Array | null;
  mapSky: Uint16Array | null;
  wide: boolean;
  farFacQ8: number;
  skyFacQ8: number;
  farVxQ8: number;
  skyVxQ8: number;
  gradient: Uint16Array | null;
  objTiles: Uint8Array;
  protos: CompiledProto[];
  cue: Uint8Array;
  cam0: number;
  camMin: number;
  camMax: number;
  rasterMode: number;
  rasterAmp: number;
  letterbox0: number;
  backdrop: number;
}

export interface CompiledFilm {
  title: string;
  scenes: CompiledScene[];
  textOffs: number[];
  textBlob: Uint8Array;
  glyphs: Uint8Array;
  nHalfcells: number;
  uiBg: Uint8Array;
  uiObj: Uint8Array;
  debug: {
    sceneIds: Record<string, number>;
    texts: string[];
    vars: Record<string, number>;
    flags: Record<string, number>;
  };
}

const RASTER_OFF = 0, RASTER_GRADIENT = 1, RASTER_WAVE_MAIN = 2, RASTER_WAVE_FAR = 3;

export async function compileFilm(entryPath: string): Promise<CompiledFilm> {
  const entry = resolve(entryPath);
  const base = dirname(entry);
  const { registry, cues } = await evaluateFilm(entry);
  const film = registry.film!;
  if (film.scenes.length === 0) throw new Error("film has no scenes");
  if (film.scenes.length > MAX_SCENES) throw new Error(`too many scenes (max ${MAX_SCENES})`);

  const sceneIndex = new Map<string, number>();
  film.scenes.forEach((s, i) => {
    if (sceneIndex.has(s.id)) throw new Error(`duplicate scene id ${s.id}`);
    sceneIndex.set(s.id, i);
  });

  const texts = new TextBank();
  const vars = new Map<string, number>();
  const flags = new Map<string, number>();

  const scenes: CompiledScene[] = [];
  for (const decl of film.scenes) {
    scenes.push(await compileScene(decl, base, { texts, vars, flags, sceneIndex, cues }));
  }

  const { offs, blob } = texts.buildBlob();
  const glyphs = texts.bakeGlyphStore(UI_INK, UI_BOX);

  return {
    title: film.title,
    scenes,
    textOffs: offs,
    textBlob: blob,
    glyphs,
    nHalfcells: glyphs.length / 64,
    uiBg: uiBgTiles(),
    uiObj: uiObjTiles(),
    debug: {
      sceneIds: Object.fromEntries(sceneIndex),
      texts: texts.entries.map((e) => e.raw),
      vars: Object.fromEntries(vars),
      flags: Object.fromEntries(flags),
    },
  };
}

interface SceneEnv {
  texts: TextBank;
  vars: Map<string, number>;
  flags: Map<string, number>;
  sceneIndex: Map<string, number>;
  cues: import("./evaluate.ts").CueSite[];
}

async function loadLayer(base: string, layer: LayerDecl): Promise<Quantized> {
  const img = await loadPng(resolve(base, layer.png));
  return quantize(img, 15);
}

async function compileScene(decl: SceneDecl, base: string, env: SceneEnv): Promise<CompiledScene> {
  const palBg = new Uint16Array(256);
  const palObj = new Uint16Array(256);

  // UI palettes (BG bank 15 + OBJ bank 15)
  uiPalette().forEach((c, i) => {
    palBg[PALBANK_UI * 16 + i] = c;
    palObj[PALBANK_OBJ_UI * 16 + i] = c;
  });

  const backdrop = hex555(decl.backdrop ?? "#000000");
  palBg[0] = backdrop;

  // --- main layer -> charblock 0 ------------------------------------------------
  let tilesMain = new Uint8Array(32); // tile 0 blank
  let nMain = 1;
  let mapMain = new Uint16Array(1024);
  let wide = false;
  let imgW = 240;
  if (decl.main) {
    const q = await loadLayer(base, decl.main);
    imgW = q.w;
    wide = q.w > 240;
    if (wide && q.w > 512) throw new Error(`[${decl.id}] main image too wide (max 512): ${q.w}`);
    const tl = tileLayer(q);
    if (1 + tl.tiles.length > MAIN_TILE_MAX) throw new Error(`[${decl.id}] main tiles ${tl.tiles.length} > budget`);
    tilesMain = new Uint8Array((1 + tl.tiles.length) * 32);
    tl.tiles.forEach((t, i) => tilesMain.set(t, (1 + i) * 32));
    nMain = 1 + tl.tiles.length;
    mapMain = buildMap(tl, wide, 1, PALBANK_MAIN);
    q.pal555.forEach((c, i) => {
      if (i > 0) palBg[PALBANK_MAIN * 16 + i] = c;
    });
  }

  // --- far + sky -> shared charblock -----------------------------------------------
  const sharedTiles: Uint8Array[] = [];
  let mapFar: Uint16Array | null = null;
  let mapSky: Uint16Array | null = null;
  let gradient: Uint16Array | null = null;
  let farFacQ8 = Math.round((decl.far?.scroll ?? 0.4) * 256);
  let skyFacQ8 = 0;
  const farVxQ8 = Math.round((decl.far?.vx ?? 0) * 256);
  let skyVxQ8 = 0;

  if (decl.far) {
    const q = await loadLayer(base, decl.far);
    const tl = tileLayer(q);
    const tileBase = FARSKY_BASE + sharedTiles.length;
    sharedTiles.push(...tl.tiles);
    mapFar = buildMap(tl, false, tileBase, PALBANK_FAR, (decl.far.y ?? 0) >> 3);
    q.pal555.forEach((c, i) => {
      if (i > 0) palBg[PALBANK_FAR * 16 + i] = c;
    });
  }
  if (decl.sky) {
    if (decl.sky.kind === "gradient") {
      gradient = gradientTable(decl.sky.stops);
    } else {
      const q = await loadPng(resolve(base, decl.sky.png)).then((img) => quantize(img, 15));
      const tl = tileLayer(q);
      const tileBase = FARSKY_BASE + sharedTiles.length;
      sharedTiles.push(...tl.tiles);
      mapSky = buildMap(tl, false, tileBase, PALBANK_SKY, (decl.sky.y ?? 0) >> 3);
      q.pal555.forEach((c, i) => {
        if (i > 0) palBg[PALBANK_SKY * 16 + i] = c;
      });
      skyFacQ8 = Math.round((decl.sky.scroll ?? 0.15) * 256);
      skyVxQ8 = Math.round((decl.sky.vx ?? 0) * 256);
    }
  }
  if (sharedTiles.length > FARSKY_MAX) throw new Error(`[${decl.id}] far+sky tiles ${sharedTiles.length} > ${FARSKY_MAX}`);
  const tilesShared = new Uint8Array(sharedTiles.length * 32);
  sharedTiles.forEach((t, i) => tilesShared.set(t, i * 32));

  // --- actors -> protos + OBJ sheet ----------------------------------------------------
  const actorEntries = Object.entries(decl.actors ?? {});
  if (actorEntries.length > MAX_SPRITES) throw new Error(`[${decl.id}] too many actors (max ${MAX_SPRITES})`);
  const protos: CompiledProto[] = [];
  const protoByPng = new Map<string, number>();
  const objParts: Uint8Array[] = [];
  let objTileCursor = 0;
  const actors: CueCtx["actors"] = new Map();
  let nextObjBank = 0;

  for (const [name, a] of actorEntries) {
    const key = `${a.png}|${a.w}x${a.h}x${a.frames ?? 1}`;
    let protoIdx = protoByPng.get(key);
    if (protoIdx === undefined) {
      const img = await loadPng(resolve(base, a.png));
      const frames = a.frames ?? 1;
      if (img.width < a.w * frames || img.height < a.h) {
        throw new Error(`[${decl.id}] sprite ${a.png}: ${img.width}x${img.height} < ${a.w * frames}x${a.h}`);
      }
      const q = quantize(img, 15);
      if (nextObjBank >= PALBANK_OBJ_UI) throw new Error(`[${decl.id}] too many OBJ palettes`);
      const bank = nextObjBank++;
      q.pal555.forEach((c, i) => {
        if (i > 0) palObj[bank * 16 + i] = c;
      });
      const sheet = tileObjSheet(q, a.w, a.h, frames);
      protoIdx = protos.length;
      protos.push({
        tileBase: objTileCursor,
        w: a.w,
        h: a.h,
        frames,
        palbank: bank,
        fps: a.fps ?? 10,
      });
      objTileCursor += sheet.length / 32;
      objParts.push(sheet);
      protoByPng.set(key, protoIdx);
    }
    actors.set(name, { slot: actors.size, proto: protoIdx, decl: a });
  }
  if (objTileCursor > 1000) throw new Error(`[${decl.id}] OBJ tiles ${objTileCursor} > 1000 budget`);
  const objTiles = new Uint8Array(objParts.reduce((n, p) => n + p.length, 0));
  {
    let o = 0;
    for (const p of objParts) {
      objTiles.set(p, o);
      o += p.length;
    }
  }

  // --- cue ------------------------------------------------------------------------------
  const cueRef = decl.play;
  if (!cueRef || typeof (cueRef as { __cue?: number }).__cue !== "number") {
    throw new Error(`[${decl.id}] scene has no play: cue(...)`);
  }
  const site = env.cues.find((c) => c.id === (cueRef as { __cue: number }).__cue);
  if (!site) throw new Error(`[${decl.id}] cue site not found`);
  const cue = residualizeCue(site.body, {
    texts: env.texts,
    vars: env.vars,
    flags: env.flags,
    sceneIndex: env.sceneIndex,
    actors,
    cueName: decl.id,
  });

  // --- camera + raster defaults ----------------------------------------------------------
  const camMin = decl.camera?.min ?? 0;
  const camMax = decl.camera?.max ?? Math.max(0, imgW - 240);
  const cam0 = decl.camera?.start ?? camMin;
  let rasterMode = RASTER_OFF;
  let rasterAmp = 0;
  if (gradient) rasterMode = RASTER_GRADIENT;
  if (decl.wave) {
    rasterMode = decl.wave.layer === "far" ? RASTER_WAVE_FAR : RASTER_WAVE_MAIN;
    rasterAmp = decl.wave.amp;
  }

  return {
    id: decl.id,
    palBg,
    palObj,
    tilesMain,
    nMain,
    tilesShared,
    nShared: sharedTiles.length,
    mapMain,
    mapFar,
    mapSky,
    wide,
    farFacQ8,
    skyFacQ8,
    farVxQ8,
    skyVxQ8,
    gradient,
    objTiles,
    protos,
    cue,
    cam0,
    camMin,
    camMax,
    rasterMode,
    rasterAmp,
    letterbox0: decl.letterbox ?? 0,
    backdrop,
  };
}
