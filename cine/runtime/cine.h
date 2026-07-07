/* cine/runtime/cine.h — internal contract of the @pocketjs/cine GBA runtime.
 *
 * The compiler residualizes a film into gen_data.c: one CineFilm with per-scene
 * palettes, tile sets, tilemaps, OBJ sheets, cue bytecode, gradient tables and
 * a global text bank + glyph store. The runtime is fixed; it never parses
 * containers — everything is typed arrays in ROM.
 *
 * Frame order (main.c): input -> vm_tick -> tween_step -> sprites/captions
 * update -> fx_apply (registers+shadow) -> debug -> vblank (ISR: OAM DMA,
 * scroll regs) -> caption_pump (VRAM glyph streaming).
 */
#ifndef CINE_H
#define CINE_H
#include "gba.h"
#include "cine_gen.h"

/* --- residual data (gen_data.c) ---------------------------------------------- */
typedef struct {
  u16 tile_base; /* in 4bpp-tile units within the scene OBJ sheet */
  u8 w, h;       /* pixels: 8,16,32,64 */
  u8 frames;
  u8 palbank;
  u8 fps; /* default anim speed, frames per cell */
} CineProto;

typedef struct {
  const u16 *pal_bg;  /* 256 */
  const u16 *pal_obj; /* 256 */
  const u8 *tiles_main;
  u16 n_main; /* 4bpp tiles -> charblock 0.. */
  const u8 *tiles_shared;
  u16 n_shared; /* -> charblock 2 @ C_FARSKY_BASE */
  const u16 *map_main; /* 32x32, or 64x32 when wide */
  const u16 *map_far;  /* 32x32 or 0 */
  const u16 *map_sky;  /* 32x32 or 0 */
  u8 wide;
  s16 far_fac_q8, sky_fac_q8; /* parallax factors vs camera */
  s16 far_vx_q8, sky_vx_q8;   /* autoscroll */
  const u16 *gradient;        /* [160] backdrop per scanline, or 0 */
  const u8 *obj_tiles;
  u16 obj_bytes;
  const CineProto *protos;
  u8 n_protos;
  const u8 *cue;
  u16 cue_len;
  s16 cam0, cam_min, cam_max;
  u8 raster_mode, raster_amp;
  u8 letterbox0;
  u16 backdrop; /* PAL[0] when no gradient */
} CineScene;

typedef struct {
  const CineScene *scenes;
  u8 n_scenes;
  const u32 *text_offs; /* [n_texts] into text_blob */
  const u8 *text_blob;
  u16 n_texts;
  const u8 *glyphs; /* halfcells: 64 bytes each (two stacked 4bpp tiles) */
  u16 n_halfcells;
  const u8 *ui_bg_tiles;  /* 4 tiles: blank, box, accent, cursor */
  const u8 *ui_obj_tiles; /* A-prompt 16x16 + digits 0-9 8x16 */
  u16 ui_obj_bytes;
} CineFilm;

extern const CineFilm film;

/* --- runtime state ------------------------------------------------------------ */
typedef struct {
  u8 active, proto, flags, mode; /* mode: 0 static, 1 loop */
  s16 x, y;                      /* world px (or screen px when SPR_SCREEN) */
  u8 frame, timer, fps;
  u8 affine; /* uses matrix 0, double-size */
} Spr;

typedef struct {
  u8 active, target, ease;
  s16 from, to;
  u16 t, T;
} Tween;

typedef struct {
  const CineScene *sc;
  u8 scene;
  u8 pending_scene; /* 0xff none */
  u16 frame;
  u8 film_done;
  u8 raster_mode; /* current RASTER_* (scene default, RASTER op overrides) */

  /* cue vm */
  u16 ip;
  s16 stack[8];
  u8 sp;
  u8 waiting; /* WAITING_* */
  u16 wait_frames;
  u8 fade_mode; /* FADE op in flight */
  u8 ctl_slot;
  s16 ctl_exit;
  u8 ctl_speed_q4;
  u8 mash_var;
  u16 mash_target;
  s8 last_choice;

  /* choice ui */
  u8 choice_n, choice_cursor;
  u16 choice_ids[C_MAX_CHOICES];

  /* world */
  s16 fx[16];       /* tween-target values, TW_* indexed */
  s32 far_off_q8, sky_off_q8;
  Spr spr[C_MAX_SPRITES];
  u16 tween_mask;
  Tween tw[C_MAX_TWEENS];

  /* counter hud */
  u8 counter_show, counter_var;
  s16 counter_x, counter_y;
  s16 counter_prev;
  u8 counter_bounce;

  /* text */
  u16 slot_next;
  u16 cur_text;
  u8 caption_busy;
  u8 prompt_on; /* A-prompt blink master switch */

  s16 vars[C_N_VARS];
  u16 flags;

  u16 keys, keys_prev;
  u16 rng;
} Cine;

extern Cine g;
extern ObjAttr oam_shadow[128];

/* values the ISR consumes (computed in fx_apply) */
extern volatile u32 vbl_count;
extern u16 isr_hofs[4], isr_vofs[4]; /* final per-BG scroll incl. shake */
extern u16 isr_lb;                   /* letterbox px */
extern const u16 *isr_grad;          /* 0 = none */
extern u16 isr_backdrop;
extern u16 isr_wave_amp;             /* px, 0 = off */
extern u8 isr_wave_bg;               /* which BG the wave applies to */
extern u16 isr_wave_phase;

/* input.c */
void input_poll(void);
u16 key_held(u16 m);
u16 key_pressed(u16 m);

/* irq.c */
void irq_init(void);
void frame_wait(void);

/* video.c */
void video_boot(void);
void scene_load(u8 id);

/* tween.c */
void tween_start(u8 target, s16 to, u16 T, u8 ease);
void tween_step(void);
s16 *tween_slot_value(u8 target); /* where a target's value lives */

/* fx.c */
void fx_reset(void);
void fx_apply(void);

/* obj.c */
void sprites_update(void);
void sprites_draw(void); /* fills oam_shadow */
s16 spr_screen_x(const Spr *s);

/* caption.c */
void caption_boot(void);
void caption_show(u8 style, u16 text_id);
void caption_clear(u8 style);
void caption_dialog(u16 speaker, u16 body);
void caption_update(void); /* typewriter progress, 1 halfcell/frame */
u8 caption_typing(void);
void choice_show(u8 n, const u16 *ids);
void choice_update(void);
u8 choice_done(s8 *out);

/* cue_vm.c */
void vm_start(void);
void vm_tick(void);

/* sfx.c */
void sfx_boot(void);
void sfx_play(u8 id);

/* debug.c */
void debug_flush(void);

#define FLAG_GET(i) ((g.flags >> (i)) & 1)
#define FLAG_SET(i) (g.flags |= (u16)(1 << (i)))
#define FLAG_CLR(i) (g.flags &= (u16)~(1 << (i)))

#endif
