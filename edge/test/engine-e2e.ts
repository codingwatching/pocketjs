// edge/test/engine-e2e.ts — headless engine test for the NEW edge paths:
// world scenes (grid walk, collision, NPC talk, examine spot, exit trigger)
// and the breakout minigame + meters. Uses the smoke film (placeholder art).
//
//   bun test/engine-e2e.ts

import { $ } from "bun";
import { compileFilm } from "../compiler/index.ts";
import { emitGenData, emitGenMusic } from "../compiler/emit.ts";
import { buildRom } from "../compiler/rom.ts";
import {
  DEBUG_ADDR, DBG, DBG_MAGIC, WAITING, SCENE_WORLD, SCENE_ACTION, ACT_CLEARED, DIR_UP, DIR_DOWN,
} from "../spec/edge.ts";

const HERE = new URL(".", import.meta.url).pathname;
const ROOT = HERE + "../";
const RUNNER = ROOT + "../aot/test/harness/mgba_runner";
const ROM = ROOT + "dist/smoke.gba";
const SHOTS = ROOT + "dist/shots";

type Step =
  | { op: "advance"; frames: number }
  | { op: "press"; buttons: string[]; frames: number; release?: number }
  | { op: "read"; name: string; addr: number; size: 1 | 2 | 4 }
  | { op: "screenshot"; path: string };

const A = (n = 8): Step => ({ op: "press", buttons: ["A"], frames: 1, release: n });
const hold = (b: string, frames: number): Step => ({ op: "press", buttons: [b], frames, release: 4 });
const adv = (frames: number): Step => ({ op: "advance", frames });
const rd = (name: string, off: number, size: 1 | 2 | 4 = 1): Step => ({ op: "read", name, addr: DEBUG_ADDR + off, size });
const shot = (n: string): Step => ({ op: "screenshot", path: `${SHOTS}/${n}.ppm` });

async function run(steps: Step[]): Promise<Record<string, number>> {
  const scenario = ROOT + "dist/engine-e2e-scenario.json";
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
  console.log("Building smoke film...");
  const film = await compileFilm(ROOT + "test/smoke-film.ts");
  const rom = await buildRom(emitGenData(film), ROM, "EDGESMOKE", emitGenMusic(film));
  await $`mkdir -p ${SHOTS}`.quiet();
  console.log(`ROM: ${rom.size} bytes\n`);

  const sc = film.debug.sceneIds;
  const varAddr = (name: string): number => {
    const idx = film.debug.vars[name];
    if (idx === undefined) throw new Error("unknown var " + name);
    return DBG.VARS + idx * 2;
  };

  console.log("Scenario 1 — world scene: roam, collision, NPC, spot, exit");
  {
    const r = await run([
      adv(90), // fade in + chip caption + OP_WORLD
      rd("magic", DBG.MAGIC, 4),
      rd("kind", DBG.KIND),
      rd("waiting_world", DBG.WAITING),
      rd("cx0", DBG.PLAYER_CX),
      rd("cy0", DBG.PLAYER_CY),
      shot("engine_world"),
      // walk up 4 cells; the NPC on (10,2) blocks the 5th so we stop at (10,3)
      hold("UP", 70),
      rd("cy_up", DBG.PLAYER_CY),
      rd("dir_up", DBG.PLAYER_DIR),
      // talk to the NPC (facing up)
      A(),
      adv(90), // dialog typing
      rd("waiting_dialog", DBG.WAITING),
      shot("engine_npc"),
      A(), // dismiss
      adv(12),
      rd("talked", varAddr("talked"), 2),
      rd("back_to_world", DBG.WAITING),
      // to the bench row, then walk left until the solid bench stops us at (5,4)
      hold("DOWN", 8), // exactly one step -> (10,4)
      hold("LEFT", 70), // 5 steps then blocked by the bench
      rd("cx_left", DBG.PLAYER_CX),
      rd("cy_row", DBG.PLAYER_CY),
      rd("dir_left", DBG.PLAYER_DIR),
      A(), // examine the bench (facing (4,4))
      adv(70),
      rd("waiting_spot", DBG.WAITING),
      A(),
      adv(12),
      rd("benched", varAddr("benched"), 2),
      // to the door: DOWN to the bottom wall (5,13), then RIGHT across the exit
      hold("DOWN", 200),
      hold("RIGHT", 100), // steps onto (10,13) mid-hold -> exit fires
      adv(40), // captionClear + setVar + fadeOut
      rd("exit_code", varAddr("exit_code"), 2),
      adv(40),
      rd("scene_next", DBG.SCENE),
    ]);
    check("debug magic 'EDGE'", r.magic >>> 0, DBG_MAGIC);
    check("scene kind is WORLD", r.kind, SCENE_WORLD);
    check("roaming (WAITING_WORLD)", r.waiting_world, WAITING.WORLD);
    check("player starts at cx=10", r.cx0, 10);
    check("player starts at cy=7", r.cy0, 7);
    check("NPC blocks at cy=3", r.cy_up, 3);
    check("facing up", r.dir_up, DIR_UP);
    check("NPC talk opens dialog", r.waiting_dialog, WAITING.DIALOG);
    check("talk cue ran (talked=1)", r.talked, 1);
    check("dialog returns to roam", r.back_to_world, WAITING.WORLD);
    check("bench blocks at cx=5", r.cx_left, 5);
    check("on the bench row (cy=4)", r.cy_row, 4);
    check("facing left", r.dir_left, 2);
    check("bench spot waits for A", r.waiting_spot, WAITING.A);
    check("spot cue ran (benched=1)", r.benched, 1);
    check("door exit pushes its value", r.exit_code, 7);
    check("exit chains into arcade", r.scene_next, sc.arcade);
  }

  console.log("Scenario 3 — action: stage, gate, kills, sandevistan, music, exit");
  {
    const preRoll: Step[] = [
      adv(90), // room boots
      hold("DOWN", 130), // straight out the door
      adv(60), // arcade fade + caption + OP_BREAKOUT
      adv(440), // let the breakout budget (420) expire untouched
      A(), // dismiss the arcade dialog
      adv(80), // fadeOut + scene switch + alley fadeIn + music op
    ];
    const r = await run([
      ...preRoll,
      rd("scene_alley", DBG.SCENE),
      rd("kind", DBG.KIND),
      rd("waiting_action", DBG.WAITING),
      rd("music_on", DBG.MUSIC),
      rd("hp0", varAddr("hp"), 2),
      rd("x0", DBG.ACT_X, 2),
      shot("engine_action0"),
      // run right toward the gate; the thug (wave 1) activates and closes in
      hold("RIGHT", 120),
      rd("x_gate", DBG.ACT_X, 2),
      rd("enemies1", DBG.ACT_ENEMIES),
      // sandevistan: hold R and check the flag + the world slowdown survives
      { op: "press", buttons: ["R"], frames: 30, release: 0 },
      rd("sande_on", DBG.ACT_SANDE),
      rd("sande_left", varAddr("sande"), 2),
      shot("engine_action_sande"),
      adv(10),
      // kill the thug: face it and fire until it drops (melee if adjacent)
      hold("B", 4), adv(20), hold("B", 4), adv(20), hold("B", 4), adv(20),
      hold("B", 4), adv(20), hold("B", 4), adv(30),
      rd("kills1", varAddr("kills"), 2),
      // push on run-and-gun style: run bursts with covering fire, over and over.
      // Deaths respawn at the gate checkpoint, so progress is monotonic-ish.
      ...Array.from({ length: 16 }, (): Step[] => [
        hold("RIGHT", 55),
        hold("B", 4),
        adv(6),
        { op: "press", buttons: ["B", "UP"], frames: 4, release: 6 }, // anti-drone diagonal
      ]).flat(),
      rd("x_late", DBG.ACT_X, 2),
      ...Array.from({ length: 6 }, (): Step[] => [hold("RIGHT", 55), hold("B", 4), adv(6)]).flat(),
      adv(40), // fadeOut after action returns
      rd("act_result", varAddr("act_result"), 2),
      rd("music_off", DBG.MUSIC),
      rd("deaths", varAddr("deaths"), 2),
      adv(60),
      rd("scene_after", DBG.SCENE),
    ]);
    check("arcade jumps into alley", r.scene_alley, sc.alley);
    check("scene kind is ACTION", r.kind, SCENE_ACTION);
    check("stage running (WAITING_ACTION)", r.waiting_action, WAITING.ACTION);
    check("music track 0 streaming", r.music_on, 1);
    check("hp meter primed", r.hp0, 6);
    checkTrue("player starts near the left edge", r.x0 < 60, `x0=${r.x0}`);
    checkTrue("gate holds the player at 320", r.x_gate <= 322, `x_gate=${r.x_gate}`);
    checkTrue("wave-1 thug activated", r.enemies1 >= 1, `enemies=${r.enemies1}`);
    check("sandevistan engages on R", r.sande_on, 1);
    checkTrue("sandevistan gauge drains", r.sande_left < 24, `sande=${r.sande_left}`);
    checkTrue("thug went down (kills>=1)", r.kills1 >= 1, `kills=${r.kills1}`);
    checkTrue("gate opened after the kill", r.x_late > 400, `x_late=${r.x_late}`);
    check("stage cleared (ACT_CLEARED)", r.act_result, ACT_CLEARED);
    check("music('off') silences the stream", r.music_off, 0);
    checkTrue("deaths counted sanely", r.deaths >= 0 && r.deaths <= 3, `deaths=${r.deaths}`);
    check("alley falls through to street", r.scene_after, sc.street);
  }

  console.log("Scenario 2 — breakout: launch, bricks fall, budget end, meter");
  {
    const r = await run([
      adv(90), // room fadein+caption grace (scene 0 boots first)
      // replay room quickly: straight to the door
      hold("DOWN", 130),
      adv(60),
      rd("scene_arcade", DBG.SCENE),
      adv(60), // arcade fade + caption + meter + OP_BREAKOUT
      rd("waiting_minigame", DBG.WAITING),
      rd("bricks0", DBG.BRICKS),
      shot("engine_breakout0"),
      A(), // launch
      adv(240),
      rd("bricks_mid", DBG.BRICKS),
      shot("engine_breakout1"),
      adv(260), // budget (420) expires
      rd("after_game", DBG.WAITING),
      rd("cleared", varAddr("cleared"), 2),
      adv(40),
      A(), // dismiss dialog
      adv(60),
      rd("scene_after", DBG.SCENE),
    ]);
    check("door column exits the room", r.scene_arcade, sc.arcade);
    check("breakout running (MINIGAME)", r.waiting_minigame, WAITING.MINIGAME);
    check("3 rows x 12 bricks", r.bricks0, 36);
    checkTrue("ball cleared some bricks", r.bricks_mid < 36, `bricks_mid=${r.bricks_mid}`);
    checkTrue("budget ended the game", r.after_game !== WAITING.MINIGAME, `waiting=${r.after_game}`);
    checkTrue("cleared count recorded", r.cleared >= 1 && r.cleared <= 36, `cleared=${r.cleared}`);
    check("arcade chains into alley", r.scene_after, sc.alley);
  }

  console.log(`\n${failed === 0 ? "\x1b[32m" : "\x1b[31m"}${passed} passed, ${failed} failed\x1b[0m`);
  process.exit(failed === 0 ? 0 : 1);
}

await main();
