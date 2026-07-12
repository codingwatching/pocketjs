# @pocketjs/edge

**Author interactive action-dramas in TypeScript; ship a real GBA ROM with run-and-gun stages, walkable worlds, cinematics — and a PCM insert song.**

`@pocketjs/edge` is the action-genre descendant of `@pocketjs/saga` (which descends from `@pocketjs/cine`). It keeps the whole inherited vocabulary — Mode-0 four-layer parallax, per-scanline raster FX, BLDCNT fades, WIN0 letterbox, affine OBJs, typewriter captions (CJK + ASCII), top-down world scenes with NPC/trigger cues — and adds two new engine systems:

- **A side-scrolling action core** (`runtime/action.c`): player physics (run/jump/shoot, rising diagonal shots, point-blank melee), a five-behavior enemy pool (thug / gunner / drone / turret / boss with telegraphed charges, spread volleys and ground shockwaves), a shared bullet pool, wave gates with checkpoints — and the **Sandevistan**: hold R and the world runs at 1/3 rate while you run full speed, the BG palette swaps to a compiler-tinted cyan duotone, and afterimage ghosts trail behind you. Stages never fail the film: death respawns you at the last gate and increments a story-visible `deaths` var.
- **DirectSound PCM music** (`runtime/audio.c`): s8 mono tracks at 13379 Hz (exactly 224 samples/frame — the VBlank counter is the stream clock) streamed straight from cartridge ROM by DMA1 into FIFO A on Timer 0, embedded at link time via `.incbin`. PSG sfx keep running on top. `yield music("stay")` in a cue starts a track; `music("off")` stops it.

Everything is driven by the same partial-evaluation discipline: the declaration zone runs at build time, `cue(function* () { ... })` bodies are lowered from TS AST to bytecode for the suspendable cue VM.

## The game

**SEE YOU ON THE MOON —《月球见》** — an unaffiliated, non-commercial fan tribute to *Cyberpunk: Edgerunners* (Studio Trigger × CD Projekt RED). Original Chinese dialogue; the research dossier and every compression liberty is in `game/dossier.md`. Twelve scenes, action-first:

1. **Title** — the moon over Night City.
2. **公寓** — Gloria, the crash, the ambulance that never landed.
3. **诊所** — "装上它。" The Sandevistan.
4. **ALLEY** *(action)* — tutorial ambush: run, jump, shoot, melee — and R.
5. **列车** *(world)* — a pickpocket in slow time. Lucy.
6. **据点** *(world)* — Maine's crew, one choice, one trial job.
7. **WAREHOUSE** *(action)* — guards, drones, a sentry turret, two gates.
8. **天台** ★ — the moon braindance, the promise — and the insert song,
   an 8-bit cover of *"I Really Want to Stay at Your House"* streaming
   off the cartridge while you talk.
9. **STREET** *(action)* — the Tanaka job falls apart. Maine's end.
10. **LAB** *(action)* — six months later; the cyberskeleton raid.
11. **TOWER** *(boss)* — Adam Smasher. Canon says you cannot win this
    fight: at half HP the script takes over.
12. **月球** — Lucy, the quiet epilogue, credits, reprise.

~15 minutes. Controls: D-pad move · A jump/confirm · B shoot (melee point-blank, ↑+B diagonal) · **R hold = Sandevistan** · in worlds: A talks/examines.

## Build & run

```bash
# prerequisites: bun, arm-none-eabi-gcc + binutils; mgba for the headless tests
cd edge
bun run art            # (re)generate art via PixelLab (cached; needs PIXELLAB_API_KEY)
bun pixellab/walkers.ts      # assemble top-down walker sheets
bun pixellab/actionsheets.ts # assemble side-view action sheets
bun run build          # dist/see-you-on-the-moon.gba
bun run play          # build + open in mGBA.app
bun run test:engine    # engine E2E on the placeholder smoke film (40 asserts)
bun run test           # game E2E: full playthrough via mgba
```

The insert-song `.raw` files under `game/music/` are **git-ignored** — they are
third-party 8-bit fan covers used for a private, non-commercial tribute only.
See `game/music/README.md` for the fetch + `ffmpeg -ac 1 -ar 13379 -f s8` step.
The engine tests use a procedurally-generated square-wave track, so
`bun run test:engine` needs none of them; `bun run test` (the full game build)
does. Do not distribute built ROMs that embed the covers.

## Engine layout

```
spec/edge.ts        binary contract: ops, action/stage tables, music tracks,
                    VRAM plan, debug block (mirrored to runtime/edge_gen.h)
dsl/index.ts        defineFilm/defineScene (+world/+action decls) + cue vocabulary
compiler/           evaluate -> residualize -> assets (quantize, walker/action
                    sheets, sandevistan tinted palettes) -> gen_data.c +
                    gen_music.s (.incbin) -> rom
runtime/            fixed C: cue VM + world.c + action.c (physics, enemy AI,
                    bullets, sandevistan) + audio.c (DirectSound streaming)
                    + fx/raster/caption/obj/sfx + breakout.c (inherited)
pixellab/           pixflux client + game prompt sheet + walkers.ts +
                    actionsheets.ts (animate-with-text run/walk cycles)
game/               see-you-on-the-moon.ts + dossier.md + art/ + music/
test/               engine-e2e.ts (40 asserts incl. action/music), game e2e
```

E2E drives the debug block at EWRAM `0x02000000` through `aot/test/harness/mgba_runner` — action fields: player world x, enemies alive, boss hp, sandevistan engaged, music track playing.

## Fan-work notes

Unaffiliated tribute; no trademarks or trade dress in generated art (prompts describe the cast in plain visual language). Real franchise names appear only in the story text of a private fan build. `game/dossier.md` carries the source list, the echoed-lines ledger, and the do-not-state-as-fact list.
