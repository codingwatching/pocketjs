// edge/pixellab/cleanup.ts — deterministic post-process for animate-with-text
// frames already assembled into sheets. No API calls; safe to re-run.
//
//   bun pixellab/cleanup.ts
//
// animate-with-text drifts: hue shifts (a yellow jacket comes back teal) and
// motion-streak halos, sometimes attached to the figure. Treatment:
//   - animated frames get the largest-4-connected-component filter, then a
//     LUMA REMAP: every pixel is replaced by the still-palette color of the
//     nearest brightness, so whole frames are forced into the character's
//     own colors (shape survives, hue drift cannot),
//   - frames listed in REPLACE are beyond saving (halo fused to the body):
//     they are rebuilt as ±1px step-bobs of the row's generated still,
//   - stills get a crisp binary-alpha pass.

import { decodePng, encodePng } from "../compiler/png.ts";

const ART = new URL("../game/art/", import.meta.url).pathname;

interface Sheet {
  rgba: Uint8Array;
  w: number;
  h: number;
  size: number; // frame is size x size
  n: number;
}

async function loadSheet(name: string, size: number): Promise<Sheet> {
  const d = decodePng(new Uint8Array(await Bun.file(`${ART}${name}.png`).arrayBuffer()));
  if (d.height !== size || d.width % size !== 0) throw new Error(`${name}: bad sheet ${d.width}x${d.height}`);
  return { rgba: d.rgba, w: d.width, h: d.height, size, n: d.width / size };
}

function framePixels(s: Sheet, f: number): Uint8Array {
  const out = new Uint8Array(s.size * s.size * 4);
  for (let y = 0; y < s.size; y++)
    for (let x = 0; x < s.size; x++)
      out.set(s.rgba.subarray((y * s.w + f * s.size + x) * 4, (y * s.w + f * s.size + x) * 4 + 4), (y * s.size + x) * 4);
  return out;
}

function writeFrame(s: Sheet, f: number, px: Uint8Array): void {
  for (let y = 0; y < s.size; y++)
    for (let x = 0; x < s.size; x++)
      s.rgba.set(px.subarray((y * s.size + x) * 4, (y * s.size + x) * 4 + 4), (y * s.w + f * s.size + x) * 4);
}

/** luma-sorted unique palette (deduped by luma bucket) from still frames */
function lumaPalette(frames: Uint8Array[]): number[][] {
  const byLuma = new Map<number, number[]>();
  for (const px of frames)
    for (let i = 0; i < px.length; i += 4) {
      if (px[i + 3] < 128) continue;
      const luma = (px[i] * 5 + px[i + 1] * 9 + px[i + 2] * 2) >> 4;
      if (!byLuma.has(luma)) byLuma.set(luma, [px[i], px[i + 1], px[i + 2], luma]);
    }
  return [...byLuma.values()].sort((a, b) => a[3] - b[3]);
}

function largestComponent(px: Uint8Array, size: number): Uint8Array {
  const solid = new Uint8Array(size * size);
  for (let i = 0; i < size * size; i++) solid[i] = px[i * 4 + 3] >= 128 ? 1 : 0;
  const comp = new Int32Array(size * size).fill(-1);
  const sizes: number[] = [];
  const stack: number[] = [];
  for (let i = 0; i < size * size; i++) {
    if (!solid[i] || comp[i] >= 0) continue;
    const id = sizes.length;
    let count = 0;
    stack.push(i);
    comp[i] = id;
    while (stack.length) {
      const c = stack.pop()!;
      count++;
      const cx = c % size, cy = (c / size) | 0;
      for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]] as const) {
        const nx = cx + dx, ny = cy + dy;
        if (nx < 0 || ny < 0 || nx >= size || ny >= size) continue;
        const nn = ny * size + nx;
        if (solid[nn] && comp[nn] < 0) {
          comp[nn] = id;
          stack.push(nn);
        }
      }
    }
    sizes.push(count);
  }
  let best = 0;
  for (let i = 1; i < sizes.length; i++) if (sizes[i] > sizes[best]) best = i;
  const keep = new Uint8Array(size * size);
  for (let i = 0; i < size * size; i++) keep[i] = solid[i] && comp[i] === best ? 1 : 0;
  return keep;
}

function lumaRemap(px: Uint8Array, size: number, pal: number[][]): Uint8Array {
  const keep = largestComponent(px, size);
  const out = new Uint8Array(px.length);
  for (let i = 0; i < size * size; i++) {
    if (!keep[i]) continue;
    const luma = (px[i * 4] * 5 + px[i * 4 + 1] * 9 + px[i * 4 + 2] * 2) >> 4;
    // binary search-ish: nearest luma in sorted palette
    let lo = 0, hi = pal.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (pal[mid][3] < luma) lo = mid + 1;
      else hi = mid;
    }
    const cand = lo > 0 && Math.abs(pal[lo - 1][3] - luma) < Math.abs(pal[lo][3] - luma) ? pal[lo - 1] : pal[lo];
    out[i * 4] = cand[0];
    out[i * 4 + 1] = cand[1];
    out[i * 4 + 2] = cand[2];
    out[i * 4 + 3] = 255;
  }
  return out;
}

/** vertical step-bob of a still (feet-safe: bottom row preserved) */
function bobFrame(still: Uint8Array, size: number, dy: number): Uint8Array {
  const out = new Uint8Array(still.length);
  for (let y = 0; y < size; y++) {
    const sy = Math.min(size - 1, Math.max(0, y - dy));
    out.set(still.subarray(sy * size * 4, (sy * size + size) * 4), y * size * 4);
  }
  return out;
}

interface CleanSpec {
  name: string;
  size: number;
  stills: number[];
  anims: number[];
  /** frame -> [stillFrame, dy]: rebuild as a bob of that still */
  replace?: Record<number, [number, number]>;
}

async function cleanSheet(spec: CleanSpec): Promise<void> {
  const s = await loadSheet(spec.name, spec.size);
  const stills = spec.stills.map((f) => framePixels(s, f));
  const pal = lumaPalette(stills);
  for (const f of spec.anims) {
    const rep = spec.replace?.[f];
    if (rep) {
      writeFrame(s, f, bobFrame(framePixels(s, rep[0]), spec.size, rep[1]));
    } else {
      writeFrame(s, f, lumaRemap(framePixels(s, f), spec.size, pal));
    }
  }
  for (const f of spec.stills) {
    const px = framePixels(s, f);
    for (let i = 3; i < px.length; i += 4) px[i] = px[i] >= 128 ? 255 : 0;
    writeFrame(s, f, px);
  }
  await Bun.write(`${ART}${spec.name}.png`, encodePng(s.rgba, s.w, s.h));
  console.log(`cleaned ${spec.name} (${spec.anims.length} frames, ${pal.length} luma colors)`);
}

await cleanSheet({ name: "act_david", size: 32, stills: [0, 5, 6, 7], anims: [1, 2, 3, 4] });
await cleanSheet({ name: "en_thug", size: 32, stills: [0, 3], anims: [1, 2] });
await cleanSheet({ name: "en_guard", size: 32, stills: [0, 3], anims: [1, 2] });
await cleanSheet({ name: "en_cop", size: 32, stills: [0, 3], anims: [1, 2] });
await cleanSheet({ name: "boss_smasher", size: 64, stills: [0, 3], anims: [1, 2] });
// walkers: rows DOWN,UP,SIDE x4; frames 0/4/8 are the generated stills.
// david's UP row and lucy's DOWN row came back with fused halos — step-bobs.
await cleanSheet({
  name: "wd_david",
  size: 32,
  stills: [0, 4, 8],
  anims: [1, 2, 3, 5, 6, 7, 9, 10, 11],
  replace: { 5: [4, -1], 6: [4, 0], 7: [4, 1] },
});
await cleanSheet({
  name: "wd_lucy",
  size: 32,
  stills: [0, 4, 8],
  anims: [1, 2, 3, 5, 6, 7, 9, 10, 11],
  replace: { 1: [0, -1], 2: [0, 0], 3: [0, 1] },
});
console.log("done.");
