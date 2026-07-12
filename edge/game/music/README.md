# Insert-song audio (not committed)

The rooftop and moon scenes stream an 8-bit cover of *"I Really Want to Stay at
Your House"* (Rosa Walton feat. Hallie Coggins). Those `.raw` PCM files are
**git-ignored on purpose** — they are third-party fan covers, used here for a
private, non-commercial tribute only. Do not commit them or distribute a built
ROM that embeds them.

To make the full game build locally, place two files here:

- `stay-gxscc.raw`  — rooftop track (looping)
- `stay-raxlen-45s.raw` — moon reprise (looping)

Each is signed 8-bit mono PCM at 13379 Hz. From any source audio:

```
ffmpeg -i cover.m4a -ac 1 -ar 13379 -f s8 stay-gxscc.raw
# a 45s excerpt: -ss 0 -t 45
```

13379 Hz is the classic GBA DirectSound rate (exactly 224 samples per frame).
Tracks are streamed by DMA1 into FIFO A on Timer 0 (`runtime/audio.c`) and
embedded at link time via `.incbin` (`compiler/emit.ts::emitGenMusic`). The
committed engine tests use a procedurally-generated square-wave track instead,
so `bun run test:engine` needs none of this.
