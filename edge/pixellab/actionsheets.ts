// edge/pixellab/actionsheets.ts — assemble side-view ACTION sheets from the
// generated stills.
//   bun pixellab/actionsheets.ts [--force]
//
// player  act_david.png  256x32: idle, run x4 (animate-with-text), jump, shoot, hurt
// enemies en_<who>.png   128x32: idle, walk0, walk1, attack
//         humanoids get real walk frames via animate; drone/turret get a
//         procedural hover-bob (animating a tripod reads as mush).
// boss    boss_smasher.png 256x64: idle, walk x2 (animate), attack
//
// Everything is cached: existing sheets are skipped without --force.

import { apiKey } from "./client.ts";
import { decodePng, encodePng } from "../compiler/png.ts";

const ART = new URL("../game/art/", import.meta.url).pathname;

function nn(rgba: Uint8Array, w: number, h: number, k: number): Uint8Array {
  const out = new Uint8Array(w * k * h * k * 4);
  for (let y = 0; y < h * k; y++)
    for (let x = 0; x < w * k; x++) {
      const si = (Math.floor(y / k) * w + Math.floor(x / k)) * 4;
      out.set(rgba.subarray(si, si + 4), (y * w * k + x) * 4);
    }
  return out;
}
function shrink2(rgba: Uint8Array, w: number, h: number): Uint8Array {
  const out = new Uint8Array((w / 2) * (h / 2) * 4);
  for (let y = 0; y < h / 2; y++)
    for (let x = 0; x < w / 2; x++)
      out.set(rgba.subarray((y * 2 * w + x * 2) * 4, (y * 2 * w + x * 2) * 4 + 4), (y * (w / 2) + x) * 4);
  return out;
}

async function load(name: string, size: number): Promise<Uint8Array> {
  const d = decodePng(new Uint8Array(await Bun.file(`${ART}${name}.png`).arrayBuffer()));
  if (d.width !== size || d.height !== size) throw new Error(`${name}: expected ${size}x${size}, got ${d.width}x${d.height}`);
  return d.rgba;
}

function sheet(frames: Uint8Array[], size: number): Uint8Array {
  const w = size * frames.length;
  const out = new Uint8Array(w * size * 4);
  frames.forEach((f, i) => {
    for (let y = 0; y < size; y++)
      for (let x = 0; x < size; x++)
        out.set(f.subarray((y * size + x) * 4, (y * size + x) * 4 + 4), (y * w + i * size + x) * 4);
  });
  return encodePng(out, w, size);
}

/** shift a frame vertically (positive = down), transparent fill */
function bob(rgba: Uint8Array, size: number, dy: number): Uint8Array {
  const out = new Uint8Array(rgba.length);
  for (let y = 0; y < size; y++) {
    const sy = y - dy;
    if (sy < 0 || sy >= size) continue;
    out.set(rgba.subarray(sy * size * 4, (sy + 1) * size * 4), y * size * 4);
  }
  return out;
}

async function animate(
  still: Uint8Array,
  size: number,
  look: string,
  action: string,
  nFrames = 4,
): Promise<Uint8Array[]> {
  // animate-with-text wants >= 64px canvases; 32px stills take a 2x round trip
  const k = size < 64 ? 2 : 1;
  const big = k === 2 ? encodePng(nn(still, size, size, 2), size * 2, size * 2) : encodePng(still, size, size);
  let lastErr = "";
  for (let attempt = 0; attempt < 4; attempt++) {
    const res = await fetch("https://api.pixellab.ai/v1/animate-with-text", {
      method: "POST",
      headers: { Authorization: `Bearer ${apiKey()}`, "Content-Type": "application/json" },
      body: JSON.stringify({
        image_size: { width: size * k, height: size * k },
        description: look,
        action,
        reference_image: { type: "base64", base64: Buffer.from(big).toString("base64") },
        view: "side",
        direction: "east",
        n_frames: nFrames,
      }),
    });
    if (res.ok) {
      const body = (await res.json()) as { images?: { base64?: string }[] };
      return (body.images ?? []).map((img) => {
        const d = decodePng(new Uint8Array(Buffer.from(img.base64!, "base64")));
        return k === 2 ? shrink2(d.rgba, d.width, d.height) : d.rgba;
      });
    }
    lastErr = `${res.status} ${await res.text()}`;
    if (res.status === 422 || res.status === 401) break;
    await new Promise((r) => setTimeout(r, 2000 * (attempt + 1)));
  }
  throw new Error(`animate-with-text(${action}): ${lastErr}`);
}

const force = process.argv.includes("--force");
const exists = async (n: string): Promise<boolean> => !force && (await Bun.file(ART + n).exists());

import { DAVID, THUG, GUARD, COP, SMASHER } from "./generate.ts";

// --- player: idle, run x4, jump, shoot, hurt --------------------------------------
if (await exists("act_david.png")) {
  console.log("skip act_david (cached)");
} else {
  const idle = await load("act_david_idle", 32);
  process.stdout.write("  animate david run... ");
  const run = await animate(idle, 32, DAVID, "run", 4);
  if (run.length < 4) throw new Error(`david run: only ${run.length} frames`);
  console.log("ok");
  const jump = await load("act_david_jump", 32);
  const shoot = await load("act_david_shoot", 32);
  const hurt = await load("act_david_hurt", 32);
  await Bun.write(ART + "act_david.png", sheet([idle, ...run.slice(0, 4), jump, shoot, hurt], 32));
  console.log("wrote act_david.png (8 frames)");
}

// --- humanoid enemies: idle, walk x2 (animated), attack ---------------------------
for (const [who, look] of [
  ["thug", THUG],
  ["guard", GUARD],
  ["cop", COP],
] as const) {
  if (await exists(`en_${who}.png`)) {
    console.log(`skip en_${who} (cached)`);
    continue;
  }
  const idle = await load(`en_${who}_idle`, 32);
  const attack = await load(`en_${who}_attack`, 32);
  process.stdout.write(`  animate ${who} walk... `);
  const walk = await animate(idle, 32, look, "walk", 4);
  console.log("ok");
  await Bun.write(ART + `en_${who}.png`, sheet([idle, walk[1] ?? bob(idle, 32, 1), walk[2] ?? bob(idle, 32, -1), attack], 32));
  console.log(`wrote en_${who}.png (4 frames)`);
}

// --- drone + turret: procedural hover-bob ------------------------------------------
for (const who of ["drone", "turret"] as const) {
  if (await exists(`en_${who}.png`)) {
    console.log(`skip en_${who} (cached)`);
    continue;
  }
  const idle = await load(`en_${who}_idle`, 32);
  const attack = await load(`en_${who}_attack`, 32);
  const f1 = who === "drone" ? bob(idle, 32, -1) : idle;
  const f2 = who === "drone" ? bob(idle, 32, 1) : idle;
  await Bun.write(ART + `en_${who}.png`, sheet([idle, f1, f2, attack], 32));
  console.log(`wrote en_${who}.png (4 frames)`);
}

// --- boss ---------------------------------------------------------------------------
if (await exists("boss_smasher.png")) {
  console.log("skip boss_smasher (cached)");
} else {
  const idle = await load("boss_smasher_idle", 64);
  const attack = await load("boss_smasher_attack", 64);
  process.stdout.write("  animate smasher walk... ");
  const walk = await animate(idle, 64, SMASHER, "walk", 4);
  console.log("ok");
  await Bun.write(
    ART + "boss_smasher.png",
    sheet([idle, walk[1] ?? bob(idle, 64, 1), walk[2] ?? bob(idle, 64, -1), attack], 64),
  );
  console.log("wrote boss_smasher.png (4 frames)");
}

console.log("action sheets done.");
