# @pocketjs/saga

**Author interactive pixel-art life-montage films in TypeScript; ship a real GBA ROM that flexes the 2D hardware.**

`@pocketjs/saga` is an independent sibling of `@pocketjs/aot`. Where aot compiles tile-RPGs for three consoles, saga is a *sagamatic* DSL for one: declarative parallax scenes plus residualized generator timelines, built to push Game Boy Advance Mode-0 as far as it goes — per-scanline gradient skies, alpha and white-out fades, mosaic, affine emblems, WIN0 letterboxing, wave rasters, typewriter CJK captions, PSG micro-sfx, and playable beats (walks, choices, button-mash counters) in every scene.

The first film is **《渐进人生 · A PROGRESSIVE LIFE》** — an Up-style interactive montage of the life of 尤雨溪 (Evan You), creator of Vue.js and Vite. A fan tribute (同人致敬), unaffiliated; all dialogue is original writing based on public interviews and first-party posts. All art is generated through [PixelLab](https://www.pixellab.ai) with franchise-neutral prompts and committed to `film/art/`.

## The film

Boot straight into a chapter menu (真机-friendly), or play the whole montage (~6 minutes):

1. **486与画笔** — 无锡, 1990s. Walk young him to the family 486; art came before code.
2. **水族馆** — Shanghai high school; a Flash aquarium swims across the projector (wave raster).
3. **一个URL** — New York, 2011. A thick JS book on a 45-minute bus; mash A as a demo climbs Hacker News.
4. **取名那晚** — npm says `seed` is taken; you pick the new name. February 2014: the first stars.
5. **那封信** — New Jersey, Feb 2016. An insurance letter, $4k/month on Patreon, a six-month deal. Choose to jump. *(The scene that earns the montage.)*
6. **星星** — June 16, 2018. The counter rolls past; he walks off the stage and stops watching stars.
7. **OnePiece与闪电** — the two-year Vue 3 rewrite storm, a flag hoisted on 2020-09-18, then a bolt named Vite.
8. **启航** — Singapore, 2024: VoidZero sets sail on Rust keels; 2026: an orange harbor, the flag still reads OPEN & NEUTRAL.
9. **山丘与片尾** — a family on a hill at sunset; credits tick through the anime release code names, A→T… and one unclaimed "U — ???".

## Authoring model

Two zones, same discipline as aot: the declaration zone runs at build time; `cue(function* () { ... })` never runs — its AST is lowered to bytecode for a suspendable cue VM.

```ts
const letter = defineScene({
  id: "letter",
  main: image("art/bg_kitchen.png"),
  actors: { evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [66, 104] }) },
  play: cue(function* () {
    yield letterbox(14, 1);
    yield fadeIn(60);
    yield caption("chip", "新泽西 · 2016年2月");
    yield dialog("妻子", "六个月。不行的话,\n我就把你踢回大厂上班。");
    while (yield varEq("jumped", 0)) {
      const j = yield choice(["跳", "再想想"]);
      if (j === 0) yield setVar("jumped", 1);
    }
    yield fadeOut(50, "white"); // white-out carries into the next scene
  }),
});
```

Fixed layer semantics (Mode 0): BG0 = UI (captions/dialog/choice), BG1 = main stage (up to 512px wide pans), BG2 = far parallax, BG3 = sky — or no sky layer at all when the scene uses a pure raster gradient (the HBlank ISR repaints the backdrop per scanline, so a "15-color" GBA sky gets 160 shades).

## Build & run

```bash
# prerequisites: bun, arm-none-eabi-gcc + binutils; mgba for the headless tests
cd saga
bun run build          # dist/progressive-life.gba (~235 KB)
bash play.sh           # build + open in mGBA.app
bun run test           # headless E2E: full playthrough, 27 assertions
bun run smoke          # engine smoke film with procedural art (no PixelLab)
bun run art            # (re)generate film art via PixelLab — cached, only
                       #  missing files are billed; needs PIXELLAB_API_KEY in ../.env
```

Controls: D-pad ←→ for walk beats and menu, A to advance/confirm/mash. In mGBA defaults that's arrows + X.

The ROM header gets the BIOS logo bitmap + complement checksum (`compiler/rom.ts`), so `dist/progressive-life.gba` is flashcart-ready.

## Engine layout

```
spec/saga.ts        the binary contract: cue ops, tween targets, VRAM plan,
                    debug block (mirrored to runtime/saga_gen.h by spec/gen-c.ts)
dsl/index.ts        defineFilm/defineScene/cue + the residual op vocabulary
compiler/           evaluate (Bun temp-module trick) -> residualize (TS AST ->
                    bytecode) -> assets (median-cut 15-color quantize, H/V-flip
                    tile dedup, OBJ sheets, gradient tables, Unifont glyph store)
                    -> emit (gen_data.c) -> rom (link + header pass)
runtime/            fixed C runtime: cue VM, 16-slot tween engine, fx compositor
                    (BLDCNT state machine, mosaic, WIN0 letterbox, shake, affine),
                    IWRAM HBlank raster ISR, typewriter caption/dialog/choice UI,
                    OAM sprites + digit counter HUD, PSG sfx
pixellab/           typed pixflux client + the film's full prompt sheet
test/               e2e.ts (27 asserts), smoke film, ppm->png tooling
```

The E2E contract is a fixed debug block at EWRAM `0x02000000` (scene, waiting state, cue ip, camera, vars, sprite 0 position…), driven through the same headless mGBA runner as `aot/` (`aot/test/harness/mgba_runner`).

## v1 scope

One BG palette bank per layer (15 colors — PixelLab art quantizes well within it), ≤16 sprites/scene, one affine matrix (one starring emblem at a time), no audio beyond PSG blips, no save. The cue VM has vars/flags/if/while, so menus and branch loops are authored in plain TypeScript control flow.
