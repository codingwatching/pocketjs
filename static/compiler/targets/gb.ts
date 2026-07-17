// static/compiler/targets/gb.ts — Game Boy packager. (Being built.)
import type { LinkedGame } from "../link.ts";

export async function buildGb(_linked: LinkedGame, _outRom: string): Promise<void> {
  throw new Error("gb target: not implemented yet");
}
