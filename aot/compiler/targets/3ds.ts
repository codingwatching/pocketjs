// aot/compiler/targets/3ds.ts — the 3DS backend.
//
// The 3DS has a flat address space and a software-rendering runtime, so it
// ships the exact GBA lowering: the same PJGB chunk container, 4bpp tiles and
// BGR555 palettes (lowerGba is target-parameterized — text wrapping and glyph
// slots follow TARGETS["3ds"]). Only the packaging differs. Every build
// produces TWO artifacts from the same gen_cart.c:
//
//   <out>.3dsx        — devkitARM/libctru homebrew binary for console/emulator
//   <out>.host.dylib  — the SAME core compiled for the host, driven by the
//                       E2E harness (test/harness/3ds_runner.ts) over Bun FFI
//
// Toolchain: $DEVKITPRO or ~/.pocketjs/toolchains/devkitpro (same convention
// as GBDK). Set PJ_3DS_HOST_ONLY=1 to skip the device build (harness-only).

import { $ } from "bun";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { emitCartC } from "../pack.ts";
import { lowerGba } from "./gba.ts";
import type { CompileOutput } from "../index.ts";
import type { TargetBuildResult } from "./index.ts";

const ROOT = new URL("../../..", import.meta.url).pathname; // repo root
const RT = ROOT + "aot/runtime/3ds";

const CORE_MODULES = [
  "core",
  "cart",
  "map",
  "player",
  "actor",
  "camera",
  "script_vm",
  "textbox",
  "debug",
  "render",
] as const;

function devkitPro(): string | null {
  const cands = [process.env.DEVKITPRO, homedir() + "/.pocketjs/toolchains/devkitpro", "/opt/devkitpro"];
  for (const c of cands) {
    if (c && existsSync(c + "/devkitARM/bin/arm-none-eabi-gcc")) return c;
  }
  return null;
}

export function hostDylibPath(outPath: string): string {
  return outPath.replace(/\.3dsx$/, "") + ".host.dylib";
}

async function buildHostDylib(outPath: string): Promise<string> {
  const dylib = hostDylibPath(outPath);
  const sources = [...CORE_MODULES.map((m) => `${RT}/${m}.c`), `${RT}/gen_cart.c`];
  // DEVELOPER_DIR: use the CommandLineTools clang when the Xcode.app license
  // hasn't been accepted (the CLT ships its own license-free toolchain).
  const env = { ...process.env };
  if (!env.DEVELOPER_DIR && existsSync("/Library/Developer/CommandLineTools/usr/bin/clang")) {
    env.DEVELOPER_DIR = "/Library/Developer/CommandLineTools";
  }
  await $`clang -O2 -Wall -fno-strict-aliasing -dynamiclib -I${RT} ${sources} -o ${dylib}`.env(env).quiet();
  return dylib;
}

// Generate an SMDH (icon + title metadata) so the Homebrew Launcher shows a
// real name/author instead of a nameless default. Best-effort: a missing
// smdhtool or icon just means a bare .3dsx (still boots fine).
async function buildSmdh(outPath: string, dkp: string, title: string, env: Record<string, string>): Promise<string | null> {
  const smdhtool = existsSync(`${dkp}/tools/bin/smdhtool`) ? `${dkp}/tools/bin/smdhtool` : null;
  const icon = `${dkp}/libctru/default_icon.png`;
  if (!smdhtool || !existsSync(icon)) return null;
  const smdh = outPath.replace(/\.3dsx$/, "") + ".smdh";
  const name = title.replace(/[^\x20-\x7e]/g, "").trim() || "PocketJS";
  await $`${smdhtool} --create ${name} ${"PocketJS-AOT cartridge"} ${"PocketJS"} ${icon} ${smdh}`.env(env).quiet();
  return smdh;
}

async function buildDevice(outPath: string, dkp: string, title: string): Promise<void> {
  const gcc = `${dkp}/devkitARM/bin/arm-none-eabi-gcc`;
  const tool3dsx = existsSync(`${dkp}/tools/bin/3dsxtool`)
    ? `${dkp}/tools/bin/3dsxtool`
    : `${dkp}/devkitARM/bin/3dsxtool`;
  const elf = outPath.replace(/\.3dsx$/, "") + ".elf";

  const ARCH = ["-march=armv6k", "-mtune=mpcore", "-mfloat-abi=hard", "-mtp=soft"];
  const CFLAGS = [...ARCH, "-O2", "-ffunction-sections", "-fdata-sections", "-fno-strict-aliasing", "-Wall", "-D__3DS__"];
  const sources = [...CORE_MODULES.map((m) => `${RT}/${m}.c`), `${RT}/ctru_main.c`, `${RT}/gen_cart.c`];

  const env = { ...process.env, DEVKITPRO: dkp, DEVKITARM: `${dkp}/devkitARM` };
  await $`${gcc} ${CFLAGS} -I${RT} -I${dkp}/libctru/include ${sources} -specs=3dsx.specs -Wl,--gc-sections -L${dkp}/libctru/lib -lctru -lm -o ${elf}`
    .env(env)
    .quiet();

  const smdh = await buildSmdh(outPath, dkp, title, env);
  if (smdh) await $`${tool3dsx} ${elf} ${outPath} --smdh=${smdh}`.env(env).quiet();
  else await $`${tool3dsx} ${elf} ${outPath}`.env(env).quiet();
}

export async function build3ds(out: CompileOutput, outPath: string): Promise<TargetBuildResult> {
  const { blob } = lowerGba(out);
  await Bun.write(`${RT}/gen_cart.c`, emitCartC(blob));

  await buildHostDylib(outPath);

  const dkp = devkitPro();
  if (!dkp) {
    if (!process.env.PJ_3DS_HOST_ONLY) {
      throw new Error(
        "3ds: devkitARM not found (set $DEVKITPRO or install to ~/.pocketjs/toolchains/devkitpro; " +
          "PJ_3DS_HOST_ONLY=1 builds only the host harness dylib)",
      );
    }
    const dylib = hostDylibPath(outPath);
    const size = Bun.file(dylib).size;
    console.warn(`3ds: PJ_3DS_HOST_ONLY — skipped device build, host dylib at ${dylib}`);
    return { rom: dylib, size };
  }

  await buildDevice(outPath, dkp, out.game.title);
  return { rom: outPath, size: Bun.file(outPath).size };
}
