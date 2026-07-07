// cine/dsl/index.ts — the @pocketjs/cine authoring surface.
//
// Two zones, same discipline as @pocketjs/aot:
//  - the DECLARATION zone (defineFilm/defineScene/image/gradient/sprite) is
//    executed at build time and fills a registry;
//  - the RESIDUAL zone (`play: cue(function* () { ... })`) is never executed —
//    the compiler lowers the generator's AST to cue bytecode. The op functions
//    exported here are compile-time vocabulary; calling them outside a cue()
//    body is an authoring error.

export type Ease = "linear" | "in" | "out" | "inout";
export type CaptionStyle = "chip" | "sub" | "card";

export interface GradientDecl {
  kind: "gradient";
  stops: string[];
}
export interface LayerDecl {
  kind: "image";
  png: string;
  scroll?: number; // parallax factor vs camera (default: far .4, sky .15)
  vx?: number; // autoscroll px/frame (fractions fine)
  wide?: boolean; // main only: image wider than 240 -> 64x32 map
}
export interface ActorDecl {
  png: string;
  w: number;
  h: number;
  frames?: number; // horizontal strip
  fps?: number; // anim frame period (frames per cell)
  at?: [number, number];
  show?: boolean;
  flip?: boolean;
  ghost?: boolean; // OBJ semi-transparency
  behind?: boolean; // prio 3, behind the main stage
  screen?: boolean; // screen-space (HUD-like)
}

export interface SceneDecl {
  id: string;
  sky?: GradientDecl | LayerDecl;
  far?: LayerDecl;
  main?: LayerDecl;
  backdrop?: string; // hex color when no gradient (default #000000)
  camera?: { start?: number; min?: number; max?: number };
  letterbox?: number; // initial bar height px
  wave?: { layer: "main" | "far"; amp: number }; // raster sine on from scene start
  actors?: Record<string, ActorDecl>;
  play: CueRef;
}

export interface FilmDecl {
  title: string;
  scenes: SceneDecl[];
}

export interface CueRef {
  __cue: number;
}

export interface Registry {
  film: FilmDecl | null;
  scenes: SceneDecl[];
}

const REGISTRY: Registry = { film: null, scenes: [] };

export function __getRegistry(): Registry {
  return REGISTRY;
}
export function __resetRegistry(): void {
  REGISTRY.film = null;
  REGISTRY.scenes = [];
}

// --- declaration zone ---------------------------------------------------------

export function defineScene(decl: SceneDecl): SceneDecl {
  REGISTRY.scenes.push(decl);
  return decl;
}

export function defineFilm(decl: FilmDecl): FilmDecl {
  if (REGISTRY.film) throw new Error("defineFilm() called twice");
  REGISTRY.film = decl;
  return decl;
}

export function image(png: string, opts: Omit<LayerDecl, "kind" | "png"> = {}): LayerDecl {
  return { kind: "image", png, ...opts };
}

export function gradient(...stops: string[]): GradientDecl {
  if (stops.length < 2) throw new Error("gradient() needs at least 2 stops");
  return { kind: "gradient", stops };
}

export function sprite(png: string, opts: Omit<ActorDecl, "png">): ActorDecl {
  return { png, ...opts };
}

/** Residual generator marker. The compiler replaces the argument with an id. */
export function cue(fn: number | (() => Generator<unknown, unknown, unknown>)): CueRef {
  if (typeof fn === "number") return { __cue: fn };
  throw new Error("cue() bodies are residual-only; compile with cine/compiler");
}

// --- residual zone vocabulary ---------------------------------------------------
// Every function below may ONLY appear inside cue(function* () { ... }) as
// `yield op(...)` (or in if/while conditions where noted). Bodies never run.

const residual = (name: string) => (): never => {
  throw new Error(`${name}() is residual-only — use it inside cue(function* () {...})`);
};

type R = unknown;

// blocking
export const fadeIn = residual("fadeIn") as (frames?: number, color?: "black" | "white") => R;
export const fadeOut = residual("fadeOut") as (frames?: number, color?: "black" | "white") => R;
export const wait = residual("wait") as (frames: number) => R;
export const waitA = residual("waitA") as () => R;
export const waitTweens = residual("waitTweens") as () => R;
export const caption = residual("caption") as (style: CaptionStyle, text: string) => R;
export const dialog = residual("dialog") as (speaker: string, text: string) => R;
export const choice = residual("choice") as (options: string[]) => number;
export const walkTo = residual("walkTo") as (actor: string, x: number, frames: number) => R;
export const control = residual("control") as (actor: string, exitX: number, speed?: number) => R;
export const mash = residual("mash") as (varName: string, target: number) => R;

// non-blocking
export const captionClear = residual("captionClear") as (style?: CaptionStyle | "all") => R;
export const pan = residual("pan") as (x: number, frames: number, ease?: Ease) => R;
export const panY = residual("panY") as (y: number, frames: number, ease?: Ease) => R;
export const alpha = residual("alpha") as (eva: number, evb: number, frames: number) => R;
export const mosaicTo = residual("mosaicTo") as (level: number, frames: number) => R;
export const shake = residual("shake") as (amp: number, frames: number) => R;
export const autoScroll = residual("autoScroll") as (layer: "far" | "sky", vx: number, frames?: number) => R;
export const zoom = residual("zoom") as (scale: number, frames: number, ease?: Ease) => R;
export const spinTo = residual("spinTo") as (angle: number, frames: number, ease?: Ease) => R;
export const letterbox = residual("letterbox") as (px: number, frames?: number) => R;
export const rasterWave = residual("rasterWave") as (layer: "main" | "far", amp: number) => R;
export const rasterGradient = residual("rasterGradient") as () => R;
export const rasterOff = residual("rasterOff") as () => R;
export const show = residual("show") as (
  actor: string,
  x?: number,
  y?: number,
  opts?: { flip?: boolean },
) => R;
export const hide = residual("hide") as (actor: string) => R;
export const animate = residual("animate") as (actor: string, mode: "loop" | number, fps?: number) => R;
export const moveTo = residual("moveTo") as (actor: string, x: number, y: number, frames: number, ease?: Ease) => R;
export const affineOn = residual("affineOn") as (actor: string) => R;
export const affineOff = residual("affineOff") as (actor: string) => R;
export const counter = residual("counter") as (varName: string, x: number, y: number) => R;
export const counterHide = residual("counterHide") as () => R;
export const sfx = residual("sfx") as (id: "blip" | "confirm" | "whoosh" | "star") => R;
export const gotoScene = residual("gotoScene") as (sceneId: string) => R;

// state (usable in conditions)
export const setFlag = residual("setFlag") as (name: string) => R;
export const clrFlag = residual("clrFlag") as (name: string) => R;
export const hasFlag = residual("hasFlag") as (name: string) => number;
export const setVar = residual("setVar") as (name: string, v: number) => R;
export const addVar = residual("addVar") as (name: string, d: number) => R;
export const varEq = residual("varEq") as (name: string, v: number) => number;
export const varNe = residual("varNe") as (name: string, v: number) => number;
export const varLt = residual("varLt") as (name: string, v: number) => number;
export const varGt = residual("varGt") as (name: string, v: number) => number;
export const varLe = residual("varLe") as (name: string, v: number) => number;
export const varGe = residual("varGe") as (name: string, v: number) => number;
export const rnd = residual("rnd") as (n: number) => number;
