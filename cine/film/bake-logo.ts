// cine/film/bake-logo.ts — rasterize the Vue logo from its SVG path geometry
// into a native 64x64 sprite (film/art/spr_vuelogo.png).
//
// The logo is two nested polygons (official vuejs.org SVG, viewBox
// 0 0 261.76 226.69): the outer V in Vue green over the inner V in slate.
// We scanline-fill with 4x4 supersampling and snap to the two flat colors —
// crisp pixel art, no anti-aliasing, index-friendly for the 4bpp quantizer.
//
//   bun film/bake-logo.ts

import { encodePng } from "../compiler/png.ts";

type Pt = [number, number];
// path d="M161.096.001 130.871 52.352 100.647.001H-.005L130.877 226.688 261.749.001z"
const OUTER: Pt[] = [
  [161.096, 0.001], [130.871, 52.352], [100.647, 0.001], [-0.005, 0.001],
  [130.877, 226.688], [261.749, 0.001],
];
// path d="M161.096.001 130.871 52.352 100.647.001H52.346l78.526 136.01L209.398.001z"
const INNER: Pt[] = [
  [161.096, 0.001], [130.871, 52.352], [100.647, 0.001], [52.346, 0.001],
  [130.872, 136.011], [209.398, 0.001],
];

const GREEN: [number, number, number] = [0x41, 0xb8, 0x83];
const SLATE: [number, number, number] = [0x35, 0x49, 0x5e];

function inside(p: Pt[], x: number, y: number): boolean {
  let hit = false;
  for (let i = 0, j = p.length - 1; i < p.length; j = i++) {
    const [xi, yi] = p[i];
    const [xj, yj] = p[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}

const SIZE = 64;
const VB_W = 261.76;
const VB_H = 226.69;
const scale = (SIZE - 4) / VB_W; // 2px margin each side
const drawnH = VB_H * scale;
const ox = (SIZE - VB_W * scale) / 2;
const oy = (SIZE - drawnH) / 2;

const SS = 4; // supersamples per axis
const rgba = new Uint8Array(SIZE * SIZE * 4);
for (let py = 0; py < SIZE; py++) {
  for (let px = 0; px < SIZE; px++) {
    let nInner = 0;
    let nOuterOnly = 0;
    for (let sy = 0; sy < SS; sy++) {
      for (let sx = 0; sx < SS; sx++) {
        const x = (px + (sx + 0.5) / SS - ox) / scale;
        const y = (py + (sy + 0.5) / SS - oy) / scale;
        if (inside(INNER, x, y)) nInner++;
        else if (inside(OUTER, x, y)) nOuterOnly++;
      }
    }
    const covered = nInner + nOuterOnly;
    if (covered >= (SS * SS) / 2) {
      const c = nInner >= nOuterOnly ? SLATE : GREEN;
      const i = (py * SIZE + px) * 4;
      rgba[i] = c[0];
      rgba[i + 1] = c[1];
      rgba[i + 2] = c[2];
      rgba[i + 3] = 255;
    }
  }
}

const out = new URL("./art/spr_vuelogo.png", import.meta.url).pathname;
await Bun.write(out, encodePng(rgba, SIZE, SIZE));
console.log(`baked ${out} (${SIZE}x${SIZE}, green/slate from SVG geometry)`);
