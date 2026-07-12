// edge/test/e2e.ts — full playthrough of SEE YOU ON THE MOON, headless.
//
//   bun test/e2e.ts
//
// One continuous mgba run drives all 12 scenes: title → apartment → clinic →
// ALLEY → train → hideout → WAREHOUSE → rooftop(song) → STREET → LAB →
// TOWER(boss) → moon(credits) → FILM_DONE, asserting the debug block at every
// stage boundary: scene ids, scene kinds, music stream state, the boss phase
// result, kills/deaths and the final film_done flag.
//
// World-scene navigation exploits the grid stepper's determinism: holds are
// either wall-terminated (overshoot-safe) or exact multiples of the 8-frame
// cell step. Action stages use a resilient advance-and-strafe pattern; death
// respawns at the last gate, so progress is monotonic.

import { $ } from "bun";
import { compileFilm } from "../compiler/index.ts";
import { emitGenData, emitGenMusic } from "../compiler/emit.ts";
import { buildRom } from "../compiler/rom.ts";
import {
  DEBUG_ADDR, DBG, DBG_MAGIC, WAITING,
  SCENE_CINE, SCENE_WORLD, SCENE_ACTION, ACT_BOSS_PHASE,
} from "../spec/edge.ts";

const HERE = new URL(".", import.meta.url).pathname;
const ROOT = HERE + "../";
const RUNNER = ROOT + "../aot/test/harness/mgba_runner";
const ROM = ROOT + "dist/see-you-on-the-moon.gba";
const SHOTS = ROOT + "dist/shots";

type Step =
  | { op: "advance"; frames: number }
  | { op: "press"; buttons: string[]; frames: number; release?: number }
  | { op: "read"; name: string; addr: number; size: 1 | 2 | 4 }
  | { op: "screenshot"; path: string };

const A = (n = 8): Step => ({ op: "press", buttons: ["A"], frames: 2, release: n });
const hold = (b: string, frames: number, release = 4): Step => ({ op: "press", buttons: [b], frames, release });
/** exactly one grid cell step (4 press + 8 release = one 8-frame step, no 2nd) */
const step1 = (b: string): Step => ({ op: "press", buttons: [b], frames: 4, release: 8 });
const adv = (frames: number): Step => ({ op: "advance", frames });
const rd = (name: string, off: number, size: 1 | 2 | 4 = 1): Step => ({ op: "read", name, addr: DEBUG_ADDR + off, size });
const shot = (n: string): Step => ({ op: "screenshot", path: `${SHOTS}/${n}.ppm` });

/** one dialog beat: finish typing (or fast-forward) then dismiss */
const dlg = (n = 1): Step[] =>
  Array.from({ length: n }, (): Step[] => [adv(110), A(), adv(8), A(12)]).flat();
/** one blocking-caption + waitA beat */
const wA = (): Step[] => [adv(90), A(12)];
/** run-and-gun advance: run right, snap shot, anti-air shot */
const fight = (rounds: number): Step[] =>
  Array.from({ length: rounds }, (): Step[] => [
    hold("RIGHT", 50),
    hold("B", 4, 6),
    adv(4),
    { op: "press", buttons: ["B", "UP"], frames: 4, release: 8 },
  ]).flat();
/** boss loop: hold ground, shoot, back off, shoot up */
const bossRounds = (rounds: number): Step[] =>
  Array.from({ length: rounds }, (): Step[] => [
    hold("B", 4, 8),
    { op: "press", buttons: ["B", "UP"], frames: 4, release: 8 },
    hold("LEFT", 14),
    hold("B", 4, 8),
    adv(6),
  ]).flat();

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
function checkTrue(name: string, got: boolean, detail = ""): void {
  console.log(`  ${got ? "\x1b[32mPASS\x1b[0m" : "\x1b[31mFAIL\x1b[0m"} ${name}${detail ? `: ${detail}` : ""}`);
  got ? passed++ : failed++;
}

async function main(): Promise<void> {
  console.log("Building SEE YOU ON THE MOON...");
  const film = await compileFilm(ROOT + "game/see-you-on-the-moon.ts");
  const rom = await buildRom(emitGenData(film), ROM, "MOONSMILE", emitGenMusic(film));
  await $`mkdir -p ${SHOTS}`.quiet();
  console.log(`ROM: ${rom.size} bytes\n`);

  const sc = film.debug.sceneIds;
  const varAddr = (name: string): number => {
    const idx = film.debug.vars[name];
    if (idx === undefined) throw new Error("unknown var " + name);
    return DBG.VARS + idx * 2;
  };

  const steps: Step[] = [
    // ---- S0 title --------------------------------------------------------------
    adv(60),
    rd("magic", DBG.MAGIC, 4),
    rd("kind_title", DBG.KIND),
    adv(260), // fadeIn + card + chip + sub typing
    shot("s0_title"),
    A(10),
    adv(80), // fadeOut + load
    rd("scene_apartment", DBG.SCENE),

    // ---- S1 apartment ----------------------------------------------------------
    adv(90), // fadeIn + chip
    shot("s1_apartment"),
    ...dlg(5),
    adv(240), // fadeOut, crash card (wait 80), fadeIn
    ...wA(), // trauma-team sub
    adv(90), // walkTo + dialog
    ...dlg(1),
    ...wA(), // clinic sub
    adv(90), // fadeOut + load
    rd("scene_clinic", DBG.SCENE),

    // ---- S2 clinic --------------------------------------------------------------
    adv(80),
    ...wA(), // relic sub
    ...dlg(4),
    shot("s2_clinic"),
    adv(260), // mosaic + card(70) + fadeOut
    rd("scene_alley", DBG.SCENE),

    // ---- S3 ALLEY (tutorial action) ---------------------------------------------
    adv(60),
    ...dlg(2),
    adv(90), // tutorial sub types, meters, OP_ACTION
    rd("kind_alley", DBG.KIND),
    rd("alley_waiting", DBG.WAITING),
    shot("s3_alley"),
    hold("R", 30, 0), // engage the sandevistan right away (stage guaranteed live)
    rd("alley_sande", DBG.ACT_SANDE),
    shot("s3_alley_sande"),
    hold("RIGHT", 90), // to gate 1 (also releases R)
    ...fight(4), // thug 1
    ...fight(14), // gate 2 pair + push to clear
    ...fight(8),
    adv(80),
    rd("alley_kills", varAddr("kills"), 2),
    ...wA(), // post-stage caption
    adv(80),
    rd("scene_train", DBG.SCENE),

    // ---- S4 train (world) --------------------------------------------------------
    adv(90),
    rd("kind_train", DBG.KIND),
    shot("s4_train"),
    hold("RIGHT", 16), // (1,5) -> (2,5)
    hold("UP", 16), // -> (2,4)
    hold("RIGHT", 220), // wall-terminated: blocked by Lucy at (16,4) -> stand (15,4)
    A(10), // talk
    adv(130), // slow-time caption types
    A(12),
    ...dlg(4),
    ...wA(), // "去车厢尽头的门"
    step1("DOWN"), // exactly one step -> row 5
    hold("RIGHT", 140), // onto the door (18,5) -> exit
    adv(90),
    rd("scene_hideout", DBG.SCENE),

    // ---- S5 hideout (world) --------------------------------------------------------
    // start (12,13); the grid is a maze, only col 1 is a clean corridor. L-path
    // to Maine (3,4) via the left wall + top row, then back down col 1 and along
    // row 13 to the door (9,14). Every move is wall-terminated or an exact step.
    adv(90),
    shot("s5_hideout"),
    hold("LEFT", 200), // -> (1,13)
    hold("UP", 200), // -> (1,3)
    step1("RIGHT"), step1("RIGHT"), // -> (3,3), above Maine
    hold("DOWN", 8), // face Maine (solid) -> stays (3,3)
    A(10),
    ...dlg(3), // Maine x2 + Maine "why"
    adv(100), // choice renders
    A(12), // pick option 0
    ...dlg(3), // branch reply + trial-job x2
    ...wA(), // "走出后门"
    hold("LEFT", 40), // back to (1,3)
    hold("DOWN", 200), // col 1 -> (1,13)
    step1("RIGHT"), step1("RIGHT"), step1("RIGHT"), step1("RIGHT"),
    step1("RIGHT"), step1("RIGHT"), step1("RIGHT"), step1("RIGHT"), // -> (9,13)
    step1("DOWN"), // onto the door (9,14) -> exit
    adv(90),
    rd("scene_warehouse", DBG.SCENE),

    // ---- S6 WAREHOUSE (action) -----------------------------------------------------
    adv(70),
    ...dlg(1), // rebecca
    adv(50),
    rd("kind_wh", DBG.KIND),
    shot("s6_warehouse"),
    ...fight(20),
    ...fight(12),
    adv(80),
    rd("wh_kills", varAddr("kills"), 2),
    ...dlg(2), // rebecca post + david
    adv(90),
    rd("scene_rooftop", DBG.SCENE),

    // ---- S7 rooftop (the song) ------------------------------------------------------
    adv(110), // music starts on the first op
    rd("music_rooftop", DBG.MUSIC),
    rd("kind_rooftop", DBG.KIND),
    adv(130), // wait 90 + chip
    shot("s7_rooftop"),
    ...dlg(5),
    adv(60), // wait 40
    ...dlg(2),
    adv(100),
    A(12), // choice: 我保证
    ...dlg(2),
    adv(240), // song title card + wait(180)
    shot("s7_rooftop_song"),
    adv(280), // wait(120) + fadeOut(80) + music off
    rd("scene_street", DBG.SCENE),
    rd("music_after_rooftop", DBG.MUSIC),

    // ---- S8 STREET (action + Maine) ---------------------------------------------------
    adv(70),
    ...wA(), // trap sub
    adv(40),
    shot("s8_street"),
    ...fight(22),
    ...fight(12),
    adv(80),
    ...dlg(5), // Maine's last stand
    adv(280), // white flash + card(100) + fades
    rd("scene_lab", DBG.SCENE),
    rd("street_deaths", varAddr("deaths"), 2),

    // ---- S9 LAB (cyberskeleton action) ------------------------------------------------
    adv(80),
    ...wA(),
    ...wA(),
    ...dlg(2), // fake lucy + david
    adv(130), // mosaic shake + sync caption
    ...wA(),
    adv(40),
    shot("s9_lab"),
    ...fight(22),
    ...fight(12),
    adv(80),
    ...wA(), // glitch caption
    adv(90),
    rd("scene_tower", DBG.SCENE),
    rd("lab_kills", varAddr("kills"), 2),

    // ---- S10 TOWER (boss) ---------------------------------------------------------------
    adv(80),
    ...dlg(3), // rebecca x2 + david
    adv(50),
    rd("kind_tower", DBG.KIND),
    shot("s10_tower"),
    ...fight(8), // the two cops at the gate
    hold("RIGHT", 60),
    adv(40),
    rd("tower_boss_hp", DBG.ACT_BOSS_HP),
    shot("s10_boss"),
    ...bossRounds(18),
    ...fight(4), // walk back in if we got knocked away
    ...bossRounds(20),
    adv(100),
    ...dlg(1), // SMASHER
    ...wA(), // burnout caption
    ...wA(), // rebecca caption
    adv(130), // lucy walks in
    shot("s10_farewell"),
    ...dlg(6), // farewell
    adv(240), // white fadeOut
    rd("scene_moon", DBG.SCENE),
    rd("endfight", varAddr("endfight"), 2),

    // ---- S11 moon ---------------------------------------------------------------------
    adv(130),
    rd("music_moon", DBG.MUSIC),
    shot("s11_moon"),
    adv(240), // lucy walks + sub
    ...wA(),
    ...dlg(1),
    ...wA(),
    adv(900), // credits cards
    shot("s11_credits"),
    adv(700),
    adv(400),
    rd("film_done", DBG.FILM_DONE),
    rd("final_waiting", DBG.WAITING),
    rd("final_kills", varAddr("kills"), 2),
    rd("final_deaths", varAddr("deaths"), 2),
  ];

  const r = await run(steps);

  console.log("Scene flow");
  check("debug magic 'EDGE'", r.magic >>> 0, DBG_MAGIC);
  check("title is CINE", r.kind_title, SCENE_CINE);
  check("title -> apartment", r.scene_apartment, sc.apartment);
  check("apartment -> clinic", r.scene_clinic, sc.clinic);
  check("clinic -> alley", r.scene_alley, sc.alley);
  check("alley is ACTION", r.kind_alley, SCENE_ACTION);
  check("alley stage running", r.alley_waiting, WAITING.ACTION);
  check("sandevistan engages", r.alley_sande, 1);
  checkTrue("alley kills counted", r.alley_kills >= 3, `kills=${r.alley_kills}`);
  check("alley -> train", r.scene_train, sc.train);
  check("train is WORLD", r.kind_train, SCENE_WORLD);
  check("train -> hideout", r.scene_hideout, sc.hideout);
  check("hideout -> warehouse", r.scene_warehouse, sc.warehouse);
  check("warehouse is ACTION", r.kind_wh, SCENE_ACTION);
  checkTrue("warehouse kills grew", r.wh_kills >= 8, `kills=${r.wh_kills}`);
  check("warehouse -> rooftop", r.scene_rooftop, sc.rooftop);
  console.log("The song");
  check("rooftop is CINE", r.kind_rooftop, SCENE_CINE);
  check("insert song streaming (track 1)", r.music_rooftop, 1);
  check("song stopped after the scene", r.music_after_rooftop, 0);
  check("rooftop -> street", r.scene_street, sc.street);
  console.log("The fall");
  check("street -> lab", r.scene_lab, sc.lab);
  checkTrue("deaths within mercy rules", r.street_deaths >= 0 && r.street_deaths <= 12, `deaths=${r.street_deaths}`);
  check("lab -> tower", r.scene_tower, sc.tower);
  checkTrue("lab kills grew", r.lab_kills >= 15, `kills=${r.lab_kills}`);
  console.log("The boss");
  check("tower is ACTION", r.kind_tower, SCENE_ACTION);
  checkTrue("smasher took the field", r.tower_boss_hp > 0 && r.tower_boss_hp <= 40, `boss_hp=${r.tower_boss_hp}`);
  check("boss fight ended scripted", r.endfight, ACT_BOSS_PHASE);
  check("tower -> moon", r.scene_moon, sc.moon);
  console.log("The moon");
  check("reprise streaming (track 2)", r.music_moon, 2);
  check("film completes", r.film_done, 1);
  check("final waiting is FILM_DONE", r.final_waiting, WAITING.FILM_DONE);
  checkTrue("final kills sane", r.final_kills >= 18, `kills=${r.final_kills}`);
  checkTrue("final deaths sane", r.final_deaths <= 20, `deaths=${r.final_deaths}`);

  console.log(`\n${failed === 0 ? "\x1b[32m" : "\x1b[31m"}${passed} passed, ${failed} failed\x1b[0m`);
  process.exit(failed === 0 ? 0 : 1);
}

await main();
