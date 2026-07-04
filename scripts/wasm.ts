// scripts/wasm.ts — build wasm/ for the browser/Bun hosts:
//   cargo build --release --target wasm32-unknown-unknown
// then copy target/wasm32-unknown-unknown/release/pocketjs_framework_wasm.wasm to
// host-web/pocketjs-framework.wasm and print its size.
//
//   bun scripts/wasm.ts
//
// Needs the wasm target: rustup target add wasm32-unknown-unknown

import { existsSync } from "node:fs";

const ROOT = new URL("..", import.meta.url).pathname; // pocketjs-framework/
const WASM_DIR = ROOT + "wasm";
const OUT = ROOT + "host-web/pocketjs-framework.wasm";
const BUILT = WASM_DIR + "/target/wasm32-unknown-unknown/release/pocketjs_framework_wasm.wasm";

// cargo lives in ~/.cargo/bin, which non-login shells may not have on PATH.
const env = {
  ...process.env,
  PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH ?? ""}`,
};

const proc = Bun.spawnSync(
  ["cargo", "build", "--release", "--target", "wasm32-unknown-unknown"],
  { cwd: WASM_DIR, env, stdout: "inherit", stderr: "inherit" },
);
if (proc.exitCode !== 0) {
  console.error(
    "pocketjs-framework wasm: cargo build failed" +
      " (missing target? run: rustup target add wasm32-unknown-unknown)",
  );
  process.exit(proc.exitCode ?? 1);
}
if (!existsSync(BUILT)) {
  console.error(`pocketjs-framework wasm: build succeeded but ${BUILT} is missing`);
  process.exit(1);
}

const bytes = await Bun.file(BUILT).arrayBuffer();
await Bun.write(OUT, bytes);
console.log(
  `pocketjs-framework wasm: host-web/pocketjs-framework.wasm (${bytes.byteLength} bytes, ${(bytes.byteLength / 1024).toFixed(1)} KiB)`,
);
