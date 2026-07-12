#!/usr/bin/env bun
// edge/play.ts — build SEE YOU ON THE MOON and open it in mGBA.
//   bun play.ts [game.ts]
import { $ } from "bun";
import { basename } from "node:path";

const here = new URL(".", import.meta.url).pathname;
const game = process.argv[2] ?? "game/see-you-on-the-moon.ts";
const out = `dist/${basename(game).replace(/\.[^.]+$/, "")}.gba`;

await $`bun ${here}compiler/cli.ts build ${here}${game} --out ${here}${out} --title MOONSMILE`;

const prefix = (await $`brew --prefix mgba`.nothrow().quiet().text()).trim();
const app = prefix ? `${prefix}/mGBA.app` : "";
if (app && (await Bun.file(`${app}/Contents/Info.plist`).exists())) {
  await $`open -n ${app} --args ${here}${out}`;
} else {
  await $`mgba ${here}${out}`;
}
