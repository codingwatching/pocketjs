// cine/test/gen-placeholder-art.ts — procedural PNGs for the smoke film so the
// pipeline is testable without PixelLab.

import { encodePng } from "../compiler/png.ts";

const DIR = new URL("./art/", import.meta.url).pathname;

function img(w: number, h: number, fn: (x: number, y: number) => [number, number, number, number]): Uint8Array {
  const rgba = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const [r, g, b, a] = fn(x, y);
      const i = (y * w + x) * 4;
      rgba[i] = r;
      rgba[i + 1] = g;
      rgba[i + 2] = b;
      rgba[i + 3] = a;
    }
  return encodePng(rgba, w, h);
}

// main stage: 384x160 "street": ground + building blocks + lamp posts
await Bun.write(
  DIR + "street.png",
  img(384, 160, (x, y) => {
    if (y > 128) return [70, 60, 56, 255]; // ground
    if (y > 124) return [110, 100, 90, 255]; // curb
    const block = Math.floor(x / 48);
    const inBuilding = y > 40 + (block % 3) * 16 && x % 48 < 40;
    if (inBuilding) {
      const win = x % 8 < 4 && y % 12 < 6 && y > 56;
      return win ? [230, 200, 120, 255] : [40 + (block % 4) * 12, 44, 60 + (block % 3) * 10, 255];
    }
    return [0, 0, 0, 0]; // transparent -> sky shows
  }),
);

// far layer: rolling hill silhouettes (transparent above)
await Bun.write(
  DIR + "hills.png",
  img(240, 160, (x, y) => {
    const ridge = 90 + Math.round(18 * Math.sin(x / 25) + 8 * Math.sin(x / 7));
    return y > ridge ? [24, 34, 52, 255] : [0, 0, 0, 0];
  }),
);

// walker sprite: 32x32, 2 frames side by side
await Bun.write(
  DIR + "walker.png",
  img(64, 32, (x, y) => {
    const f = x >= 32 ? 1 : 0;
    const lx = x % 32;
    // head
    if (Math.hypot(lx - 16, y - 8) < 5) return [240, 200, 160, 255];
    // body
    if (y >= 13 && y < 24 && lx >= 12 && lx < 20) return [66, 184, 131, 255];
    // legs alternate by frame
    if (y >= 24 && y < 30) {
      const spread = f ? 3 : 1;
      if (Math.abs(lx - (16 - spread)) < 2 || Math.abs(lx - (16 + spread)) < 2) return [40, 48, 60, 255];
    }
    return [0, 0, 0, 0];
  }),
);

// emblem: 32x32 V mark
await Bun.write(
  DIR + "emblem.png",
  img(32, 32, (x, y) => {
    const d1 = Math.abs(x - 6 - y * 0.4);
    const d2 = Math.abs(x - 26 + y * 0.4);
    if (y < 26 && (d1 < 2.5 || d2 < 2.5)) return [66, 184, 131, 255];
    if (y < 26 && (d1 < 4 || d2 < 4)) return [53, 73, 94, 255];
    return [0, 0, 0, 0];
  }),
);

console.log("placeholder art written to cine/test/art/");
