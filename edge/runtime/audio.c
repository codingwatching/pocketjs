/* edge/runtime/audio.c — DirectSound A PCM streaming for insert songs.
 *
 * Tracks are s8 mono in ROM at C_MUSIC_RATE (13379 Hz — exactly C_MUSIC_SPF
 * samples per 59.73 Hz frame, so the frame loop is the stream clock). Timer 0
 * paces the FIFO; DMA1 in special mode refills it straight from the cartridge
 * (DMA1/2 may read gamepak — only DMA0 can't). music_service() counts frames
 * and restarts/stops the stream at the sample count, so a track end never
 * plays garbage ROM as noise.
 *
 * PSG sfx (sfx.c) keep running on top: SOUNDCNT_H keeps the PSG mix at 100%
 * alongside the DirectSound channel. */
#include "edge.h"

static struct {
  u8 playing; /* track + 1 */
  u32 frames; /* frames since (re)start */
} mus;

void music_boot(void) {
  mus.playing = 0;
}

static void dma_arm(const s8 *pcm) {
  REG_DMA1CNT = 0;
  REG_SOUNDCNT_H = SND_DSA_ON | SND_DSA_RESET; /* reset FIFO */
  REG_SOUNDCNT_H = SND_DSA_ON;
  REG_DMA1SAD = (u32)pcm;
  REG_DMA1DAD = (u32)&REG_FIFO_A;
  REG_DMA1CNT = DMA_ENABLE | DMA_REPEAT | DMA_32 | DMA_DST_FIXED | DMA_START_SPECIAL;
}

void music_play(u8 id) {
  const EdgeTrack *t;
  if (id >= film.n_tracks) return;
  t = &film.tracks[id];
  REG_TM0CNT_H = 0;
  REG_TM0CNT_L = C_MUSIC_TIMER;
  dma_arm(t->pcm);
  REG_TM0CNT_H = TM_ENABLE;
  mus.playing = (u8)(id + 1);
  mus.frames = 0;
}

void music_stop(void) {
  if (!mus.playing) return;
  mus.playing = 0;
  REG_DMA1CNT = 0;
  REG_TM0CNT_H = 0;
  REG_SOUNDCNT_H = 0x0002; /* PSG-only mix again */
}

void music_service(void) {
  const EdgeTrack *t;
  if (!mus.playing) return;
  t = &film.tracks[mus.playing - 1];
  mus.frames++;
  if (mus.frames * C_MUSIC_SPF >= t->samples) {
    if (t->loop) {
      dma_arm(t->pcm); /* seamless-enough restart at the loop point */
      mus.frames = 0;
    } else {
      music_stop();
    }
  }
}

u8 music_playing(void) {
  return mus.playing;
}
