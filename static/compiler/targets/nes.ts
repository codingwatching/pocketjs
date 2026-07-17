// static/compiler/targets/nes.ts — NES packager. (Being built.)
import type { LinkedGame } from "../link.ts";

export async function buildNes(_linked: LinkedGame, _outRom: string): Promise<void> {
  throw new Error("nes target: not implemented yet");
}
