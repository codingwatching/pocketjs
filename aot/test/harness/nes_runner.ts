#!/usr/bin/env bun
// aot/test/harness/nes_runner.ts — headless NES runner (jsnes) speaking the
// same scenario JSON protocol as mgba_runner:
//
//   bun nes_runner.ts <rom.nes> <scenario.json>
//
// Steps: advance / press / read / screenshot. Reads address the CPU bus
// (RAM 0x0000-0x07FF holds the PJ debug block at 0x0700). Output JSON on
// stdout: {"reads": {...}, "ok": true}.

import { NES, Controller } from "jsnes";

const [romPath, scenarioPath] = process.argv.slice(2);
if (!romPath || !scenarioPath) {
  console.log(JSON.stringify({ ok: false, error: "usage: nes_runner <rom.nes> <scenario.json>" }));
  process.exit(1);
}

type Step =
  | { op: "advance"; frames: number }
  | { op: "press"; buttons: string[]; frames: number; release?: number }
  | { op: "read"; name: string; addr: number; size: 1 | 2 | 4 }
  | { op: "screenshot"; path: string };

const BTN: Record<string, number> = {
  A: Controller.BUTTON_A,
  B: Controller.BUTTON_B,
  SELECT: Controller.BUTTON_SELECT,
  START: Controller.BUTTON_START,
  UP: Controller.BUTTON_UP,
  DOWN: Controller.BUTTON_DOWN,
  LEFT: Controller.BUTTON_LEFT,
  RIGHT: Controller.BUTTON_RIGHT,
};

let frameBuffer: number[] | null = null;
const nes = new NES({
  onFrame: (fb: number[]) => {
    frameBuffer = fb;
  },
  onAudioSample: () => {},
});

const rom = await Bun.file(romPath).arrayBuffer();
const bytes = new Uint8Array(rom);
let romStr = "";
for (const b of bytes) romStr += String.fromCharCode(b);

try {
  nes.loadROM(romStr);
} catch (e) {
  console.log(JSON.stringify({ ok: false, error: `loadROM failed: ${e}` }));
  process.exit(1);
}

const scenario = (await Bun.file(scenarioPath).json()) as { steps: Step[] };
const reads: Record<string, number> = {};

function busRead(addr: number, size: number): number {
  let v = 0;
  for (let i = 0; i < size; i++) {
    // CPU RAM (with mirroring) lives in nes.cpu.mem.
    const a = addr + i;
    const byte = a < 0x2000 ? nes.cpu.mem[a & 0x7ff] : nes.cpu.mem[a];
    v |= (byte & 0xff) << (8 * i);
  }
  return v >>> 0;
}

function frames(n: number): void {
  for (let i = 0; i < n; i++) nes.frame();
}

for (const step of scenario.steps) {
  if (step.op === "advance") {
    frames(step.frames);
  } else if (step.op === "press") {
    for (const b of step.buttons) nes.buttonDown(1, BTN[b]);
    frames(step.frames);
    for (const b of step.buttons) nes.buttonUp(1, BTN[b]);
    frames(step.release ?? 0);
  } else if (step.op === "read") {
    reads[step.name] = busRead(step.addr, step.size);
  } else if (step.op === "screenshot") {
    if (frameBuffer) {
      const W = 256;
      const H = 240;
      const head = `P6\n${W} ${H}\n255\n`;
      const out = new Uint8Array(head.length + W * H * 3);
      for (let i = 0; i < head.length; i++) out[i] = head.charCodeAt(i);
      for (let i = 0; i < W * H; i++) {
        const c = frameBuffer[i]; // jsnes: 0xBBGGRR
        out[head.length + i * 3] = c & 0xff;
        out[head.length + i * 3 + 1] = (c >> 8) & 0xff;
        out[head.length + i * 3 + 2] = (c >> 16) & 0xff;
      }
      await Bun.write(step.path, out);
    }
  }
}

console.log(JSON.stringify({ reads, ok: true }));
