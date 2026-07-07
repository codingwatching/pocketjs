// saga/test/e2e.ts — build 《渐进人生》 and play it headlessly in mGBA,
// asserting the debug-block contract scene by scene: menu navigation, control
// walks, mash counters, choice branches (incl. the hesitate loop), white-fade
// scene chaining, credits, and the coda -> title loop.
//
//   bun test/e2e.ts

import { $ } from "bun";
import { compileFilm } from "../compiler/index.ts";
import { emitGenData } from "../compiler/emit.ts";
import { buildRom } from "../compiler/rom.ts";
import { DEBUG_ADDR, DBG, DBG_MAGIC, WAITING } from "../spec/saga.ts";

const HERE = new URL(".", import.meta.url).pathname;
const ROOT = HERE + "../";
const RUNNER = ROOT + "../aot/test/harness/mgba_runner";
const ROM = ROOT + "dist/progressive-life.gba";
const SHOTS = ROOT + "dist/shots";

type Step =
  | { op: "advance"; frames: number }
  | { op: "press"; buttons: string[]; frames: number; release?: number }
  | { op: "read"; name: string; addr: number; size: 1 | 2 | 4 }
  | { op: "screenshot"; path: string };

const A = (n = 8): Step => ({ op: "press", buttons: ["A"], frames: 1, release: n });
const DOWN: Step = { op: "press", buttons: ["DOWN"], frames: 1, release: 6 };
const adv = (frames: number): Step => ({ op: "advance", frames });
const rd = (name: string, off: number, size: 1 | 2 | 4 = 1): Step => ({ op: "read", name, addr: DEBUG_ADDR + off, size });
const shot = (n: string): Step => ({ op: "screenshot", path: `${SHOTS}/${n}.ppm` });

async function run(steps: Step[]): Promise<Record<string, number>> {
  const scenario = ROOT + "dist/e2e-scenario.json";
  await Bun.write(scenario, JSON.stringify({ steps }));
  const out = await $`${RUNNER} ${ROM} ${scenario}`.text();
  const line = out.trim().split("\n").reverse().find((l) => l.trim().startsWith("{"));
  if (!line) throw new Error("runner produced no JSON:\n" + out);
  const parsed = JSON.parse(line);
  if (!parsed.ok) throw new Error("runner error: " + JSON.stringify(parsed));
  return parsed.reads ?? {};
}

let passed = 0;
let failed = 0;
function check(name: string, got: unknown, want: unknown): void {
  const ok = got === want;
  console.log(`  ${ok ? "\x1b[32mPASS\x1b[0m" : "\x1b[31mFAIL\x1b[0m"} ${name}: got ${got}${ok ? "" : `, want ${want}`}`);
  ok ? passed++ : failed++;
}
function checkNear(name: string, got: number, want: number, tol: number): void {
  const ok = Math.abs(got - want) <= tol;
  console.log(`  ${ok ? "\x1b[32mPASS\x1b[0m" : "\x1b[31mFAIL\x1b[0m"} ${name}: got ${got}${ok ? "" : `, want ${want}±${tol}`}`);
  ok ? passed++ : failed++;
}

// menu helpers: boot -> title card -> (从头播放 | chapter n)
const bootToMenu: Step[] = [adv(230)];
const pickChapter = (page2: boolean, index: number): Step[] => {
  const s: Step[] = [...bootToMenu, DOWN, A(), adv(10)];
  const downs = page2 ? 4 : index;
  for (let i = 0; i < downs; i++) s.push(DOWN);
  s.push(A(), adv(10));
  if (page2) {
    for (let i = 0; i < index; i++) s.push(DOWN);
    s.push(A(), adv(10));
  }
  return s;
};

async function main(): Promise<void> {
  console.log("Building 渐进人生...");
  const film = await compileFilm(ROOT + "film/progressive-life.ts");
  const rom = await buildRom(emitGenData(film), ROM, "PROGLIFE");
  await $`mkdir -p ${SHOTS}`.quiet();
  console.log(`ROM: ${rom.size} bytes, ${film.scenes.length} scenes, ${film.debug.texts.length} texts\n`);

  const sc = film.debug.sceneIds;
  const varAddr = (name: string): number => {
    const idx = film.debug.vars[name];
    if (idx === undefined) throw new Error("unknown var " + name);
    return DBG.VARS + idx * 2;
  };
  const tid = (s: string): number => {
    const i = film.debug.texts.indexOf(s);
    if (i < 0) throw new Error("text not found: " + s);
    return i;
  };

  console.log("Scenario 1 — boot, title, 从头播放, chapter 1 beat");
  {
    const r = await run([
      adv(60),
      rd("magic", DBG.MAGIC, 4),
      rd("booted", DBG.BOOTED),
      rd("scene0", DBG.SCENE),
      adv(170),
      rd("waiting_menu", DBG.WAITING),
      shot("e2e_title"),
      A(), // 从头播放
      adv(80),
      rd("scene1", DBG.SCENE),
      adv(160), // dad dialog typing
      A(), // dismiss dad dialog
      adv(40),
      rd("walk_wait", DBG.WAITING),
      { op: "press", buttons: ["RIGHT"], frames: 140, release: 4 },
      rd("kidx", DBG.SPR0_X, 2),
      adv(20),
      A(), // (按下电源。)
      adv(60),
      A(), // 画画 caption
      adv(30),
      A(),
      adv(200), // mosaic + fade + scene advance
      rd("scene2", DBG.SCENE),
    ]);
    check("debug magic", r.magic >>> 0, DBG_MAGIC);
    check("booted", r.booted, 1);
    check("boots into title", r.scene0, sc.title);
    check("title menu is a choice", r.waiting_menu, WAITING.CHOICE);
    check("从头播放 -> paint486", r.scene1, sc.paint486);
    check("control walk active", r.walk_wait, WAITING.CONTROL);
    checkNear("kid walked to the 486", r.kidx, 168, 4);
    check("chapter 1 auto-advances to aquarium", r.scene2, sc.aquarium);
  }

  console.log("Scenario 2 — chapter select -> 一个URL: control + HN mash");
  {
    const r = await run([
      ...pickChapter(false, 2),
      adv(80),
      rd("scene", DBG.SCENE),
      adv(60),
      A(), // book caption
      adv(60),
      { op: "press", buttons: ["RIGHT"], frames: 250, release: 4 },
      rd("evanx", DBG.SPR0_X, 2),
      adv(60),
      A(), // 小实验 caption
      adv(80),
      rd("mash_wait", DBG.WAITING),
      rd("hn_before", varAddr("hn"), 2),
      A(6), A(6), A(6), A(6), A(6), A(6), A(6), A(6), A(6), A(6), A(6), A(6),
      rd("hn_after", varAddr("hn"), 2),
      rd("after_mash_wait", DBG.WAITING),
      shot("e2e_hn"),
    ]);
    check("chapter menu -> nyc", r.scene, sc.nyc);
    checkNear("evan walked to the dorm", r.evanx, 330, 4);
    check("mash waiting", r.mash_wait, WAITING.MASH);
    check("hn starts at 88", r.hn_before, 88);
    check("12 presses reach 100", r.hn_after, 100);
    check("mash completes", r.after_mash_wait === WAITING.MASH ? 1 : 0, 0);
  }

  console.log("Scenario 3 — 那封信: hesitate loop, then the jump, white into 星星");
  {
    const r = await run([
      ...pickChapter(true, 0),
      adv(100),
      rd("scene", DBG.SCENE),
      adv(120),
      A(), // patreon caption
      adv(120),
      A(), // 信箱 dialog
      adv(50),
      A(), // 妻子 1
      adv(70),
      A(), // 妻子 2
      adv(40),
      rd("choice_wait", DBG.WAITING),
      DOWN,
      A(), // 再想想
      adv(60),
      A(), // (雪停了)
      adv(70),
      A(), // caption
      adv(30),
      rd("loop_wait", DBG.WAITING), // choice again
      A(), // 跳
      adv(30),
      rd("jumped", varAddr("jumped"), 2),
      rd("choice_val", DBG.LAST_CHOICE),
      adv(260), // 他跳了 + white fade
      rd("scene_after", DBG.SCENE),
      shot("e2e_stars"),
    ]);
    check("chapter menu page 2 -> letter", r.scene, sc.letter);
    check("the choice appears", r.choice_wait, WAITING.CHOICE);
    check("再想想 loops back to the choice", r.loop_wait, WAITING.CHOICE);
    check("跳 sets the flag", r.jumped, 1);
    check("last choice recorded", r.choice_val, 0);
    check("white fade chains into 星星", r.scene_after, sc.stars);
  }

  console.log("Scenario 4 — OnePiece -> 闪电 -> 启航 chain");
  {
    const r = await run([
      ...pickChapter(true, 2),
      adv(80),
      rd("scene", DBG.SCENE),
      adv(140),
      A(), // RFC caption
      adv(60),
      A(), // 风暴 dialog
      adv(80),
      A(), // 他 dialog
      adv(160), // flag hoist
      rd("flag_text", DBG.CUR_TEXT, 2),
      A(), // one piece caption
      adv(80),
      rd("scene_vite", DBG.SCENE),
      adv(200),
      A(), // vite caption
      adv(80),
      A(), // npm create vite card -> 0.3s
      adv(300),
      A(), // 插上闪电
      adv(60),
      A(), // 联合国
      adv(120),
      rd("scene_fleet", DBG.SCENE),
    ]);
    check("chapter menu -> onepiece", r.scene, sc.onepiece);
    check("One Piece caption shows", r.flag_text, tid("Vue 3 起航,代号:\nOne Piece。") + 1);
    check("white flash chains into 闪电", r.scene_vite, sc.vite);
    check("闪电 chains into 启航", r.scene_fleet, sc.fleet);
  }

  console.log("Scenario 5 — 山丘与片尾 -> credits -> loop to title");
  {
    const r = await run([
      ...pickChapter(true, 4),
      adv(120),
      rd("scene", DBG.SCENE),
      adv(80),
      A(), // 渐进式 caption
      adv(500), // credit cards
      rd("cred_text", DBG.CUR_TEXT, 2),
      adv(220),
      A(), // 下一集
      adv(60),
      A(), // final card
      adv(220),
      rd("scene_back", DBG.SCENE),
      rd("menu_wait_frames", DBG.FRAME, 2),
    ]);
    check("chapter menu -> coda", r.scene, sc.coda);
    check(
      "credits ticker running",
      typeof r.cred_text === "number" && r.cred_text > 0 ? 1 : 0,
      1,
    );
    check("coda loops back to title", r.scene_back, sc.title);
  }

  console.log(`\n${failed === 0 ? "\x1b[32m" : "\x1b[31m"}${passed} passed, ${failed} failed\x1b[0m`);
  process.exit(failed === 0 ? 0 : 1);
}

await main();
