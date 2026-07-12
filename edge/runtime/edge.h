/* edge/runtime/edge.h — internal contract of the @pocketjs/edge GBA runtime.
 *
 * The compiler residualizes a film into gen_data.c: one EdgeFilm with per-scene
 * palettes, tile sets, tilemaps, OBJ sheets, cue bytecode, gradient tables and
 * a global text bank + glyph store. The runtime is fixed; it never parses
 * containers — everything is typed arrays in ROM.
 *
 * Frame order (main.c): input -> vm_tick -> tween_step -> sprites/captions
 * update -> fx_apply (registers+shadow) -> debug -> vblank (ISR: OAM DMA,
 * scroll regs) -> caption_pump (VRAM glyph streaming).
 */
#ifndef EDGE_H
#define EDGE_H
#include "gba.h"
#include "edge_gen.h"

/* --- residual data (gen_data.c) ---------------------------------------------- */
typedef struct {
  u16 tile_base; /* in 4bpp-tile units within the scene OBJ sheet */
  u8 w, h;       /* pixels: 8,16,32,64 */
  u8 frames;
  u8 palbank;
  u8 fps;      /* default anim speed, frames per cell */
  u8 walk_fpd; /* walker sheet: frames per direction row (0 = plain sprite).
                  rows: DOWN, UP, SIDE (right = SIDE hflipped) */
} EdgeProto;

typedef struct {
  u8 cx, cy, dir;
  u8 slot;  /* sprite slot the NPC lives in */
  u8 proto; /* sprite proto index */
  u8 cue;   /* cue index run on A-interact (0xff none) */
  u8 solid;
} EdgeNpc;

typedef struct {
  u8 cx, cy, w, h; /* cell rect */
  u8 kind;         /* C_TRIG_* */
  s16 value;       /* EXIT: pushed as OP_WORLD result */
  u8 cue;          /* EXAMINE/AUTO: cue index (0xff none) */
} EdgeTrig;

typedef struct {
  u8 cols, rows;  /* cells (16px) */
  const u8 *solid; /* cols*rows, 1 = blocked */
  u8 start_cx, start_cy, start_dir;
  u8 player_slot;  /* sprite slot the player walker lives in */
  u8 player_proto; /* walker proto index */
  const EdgeNpc *npcs;
  u8 n_npcs;
  const EdgeTrig *trigs;
  u8 n_trigs;
} EdgeWorld;

/* --- action stages ------------------------------------------------------------ */
typedef struct {
  s16 x;      /* spawn world x (activates when the player closes within ~260px) */
  u8 proto;   /* enemy sprite proto (4-frame strip, EF_*) */
  u8 behavior;/* C_EB_* */
  u8 hp;
  u8 wave;    /* gate tag */
} EdgeSpawn;

typedef struct {
  s16 x, y; /* top surface */
  s16 w;
} EdgePlat;

typedef struct {
  s16 x;   /* player x is held here... */
  u8 wave; /* ...while any spawned enemy of this wave lives */
} EdgeGate;

typedef struct {
  u8 player_proto; /* 8-frame side-view strip, PF_* */
  u8 hp_max;
  u8 sande_max;    /* gauge units; 0 = no sandevistan in this stage */
  u8 exit_kind;    /* C_AEXIT_* */
  s16 ground_y;    /* ground surface, world px */
  s16 length;      /* virtual stage length px (map wraps at 512) */
  u8 hp_var, sande_var, kills_var, deaths_var; /* bound game vars */
  const EdgeSpawn *spawns;
  u8 n_spawns;
  const EdgePlat *plats;
  u8 n_plats;
  const EdgeGate *gates;
  u8 n_gates;
  u8 boss;          /* spawn index of the boss, 0xff none */
  u8 boss_phase_hp; /* boss hp <= this ends the stage with ACT_BOSS_PHASE */
  const u16 *pal_sande; /* 256: tinted BG palette while time is slowed */
} EdgeStage;

typedef struct {
  const s8 *pcm; /* s8 mono @ C_MUSIC_RATE, u32-padded */
  u32 samples;
  u8 loop;
} EdgeTrack;

typedef struct {
  const u16 *pal_bg;  /* 256 */
  const u16 *pal_obj; /* 256 */
  const u8 *tiles_main;
  u16 n_main; /* 4bpp tiles -> charblock 0.. */
  const u8 *tiles_shared;
  u16 n_shared; /* -> charblock 2 @ C_FARSKY_BASE */
  const u16 *map_main; /* 32x32; 64x32 when map_sz 1; 64x64 (4 SBBs) when 2 */
  const u16 *map_far;  /* 32x32 or 0 (map_sz < 2 only) */
  const u16 *map_sky;  /* 32x32 or 0 (map_sz < 2 only) */
  u8 map_sz;           /* 0 = 32x32, 1 = 64x32 wide pan, 2 = 64x64 world */
  s16 far_fac_q8, sky_fac_q8; /* parallax factors vs camera */
  s16 far_vx_q8, sky_vx_q8;   /* autoscroll */
  const u16 *gradient;        /* [160] backdrop per scanline, or 0 */
  const u8 *obj_tiles;
  u16 obj_bytes;
  const EdgeProto *protos;
  u8 n_protos;
  const u8 *cue;       /* all cues, one blob */
  u16 cue_len;
  const u16 *cue_offs; /* [n_cues] offsets; cue 0 = play */
  u8 n_cues;
  u8 kind;             /* C_SCENE_* */
  const EdgeWorld *world; /* kind == WORLD */
  const EdgeStage *stage; /* kind == ACTION */
  s16 cam0, cam_min, cam_max;
  u8 raster_mode, raster_amp;
  u8 letterbox0;
  u16 backdrop; /* PAL[0] when no gradient */
} EdgeScene;

typedef struct {
  const EdgeScene *scenes;
  u8 n_scenes;
  const u32 *text_offs; /* [n_texts] into text_blob */
  const u8 *text_blob;
  u16 n_texts;
  const u8 *glyphs; /* halfcells: 64 bytes each (two stacked 4bpp tiles) */
  u16 n_halfcells;
  const u8 *ui_bg_tiles;  /* 4 tiles: blank, box, accent, cursor */
  const u8 *ui_obj_tiles; /* A-prompt 16x16 + digits 0-9 8x16 */
  u16 ui_obj_bytes;
  const EdgeTrack *tracks; /* PCM insert songs (0 when none) */
  u8 n_tracks;
} EdgeFilm;

extern const EdgeFilm film;

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
  const EdgeScene *sc;
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

  /* world mode (top-down grid) */
  u8 pl_cx, pl_cy, pl_dir;
  u8 pl_step;    /* frames left in current 16px step (0 = idle) */
  s8 pl_dx, pl_dy;
  u8 anim_phase; /* walk animation phase counter */
  u8 in_sub;     /* running an NPC/trigger cue from free roam */
  u16 ret_ip;    /* not used to jump — roam resumes at the in-flight WORLD op */
  u16 trig_seen; /* TRIG_AUTO one-shot mask */
  s16 cam_max_x, cam_max_y;

  /* scripted walk (OP_WALK) */
  u8 wk_active, wk_slot;
  s16 wk_tx, wk_ty; /* target px (sprite top-left) */

  /* meters */
  u8 meter_on[2], meter_var[2], meter_max[2];
  s16 meter_x[2], meter_y[2];

  /* fx state */
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
} Edge;

/* --- action-mode state (action.c; exposed for debug_flush) --------------------- */
typedef struct {
  u8 active, behavior, proto, hp, wave, spawn_idx;
  u8 face;   /* 1 = facing left */
  u8 timer;  /* behavior clock */
  u8 tele;   /* telegraph frames (boss charge) */
  u8 anim;
  s16 x, y;      /* feet center, world px */
  s16 vx_q4, vy_q4;
  s16 home_y;    /* drone hover base */
} ActEnemy;

typedef struct {
  u8 active, from_enemy, tile;
  s16 x, y;
  s16 vx_q4, vy_q4;
} ActBullet;

typedef struct {
  u8 active;
  const EdgeStage *st;
  u8 pl_slot;          /* Spr slot the player actor occupies */
  s16 x, y;            /* player feet center, world px */
  s16 vx_q4, vy_q4;
  s32 x_q4, y_q4;
  u8 grounded, face_left, shoot_cd, iframes, hurt_timer;
  u8 sande_on;         /* engaged this frame */
  u16 sande_frames;    /* drain/regen clocks */
  u8 world_tick;       /* increments only on world-update frames */
  u16 frame;
  s16 checkpoint_x;
  u8 next_spawn;       /* spawns are x-sorted; index of first not yet activated */
  ActEnemy en[C_MAX_ENEMIES];
  ActBullet bl[C_MAX_BULLETS];
  u8 alive;            /* enemies alive */
  u8 spawned_dead;     /* count of spawns fully resolved (for AEXIT_CLEAR) */
  u8 boss_hp;
  u8 done;             /* result pushed */
  /* sandevistan afterimages: ring of recent player poses */
  s16 gx[C_ACT_GHOSTS], gy[C_ACT_GHOSTS];
  u8 gframe[C_ACT_GHOSTS], gface[C_ACT_GHOSTS], ghead;
} Act;

extern Act act;

extern Edge g;
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
void vm_push(s16 v);
void vm_run_sub(u8 cue); /* jump into an NPC/trigger cue from free roam */

/* world.c */
void world_enter(void);  /* OP_WORLD executed: place player, camera, roam on */
void world_service(void); /* one frame of free roam (WAITING_WORLD) */
void world_warp(u8 cx, u8 cy, u8 dir);
void world_face(u8 slot, u8 dir);
void walk_start(u8 slot, u8 cx, u8 cy);
u8 walk_service(void); /* 1 while still walking */

/* breakout.c */
void breakout_start(u8 rows, u8 lives, u16 budget);
u8 breakout_service(void); /* 1 while running; pushes result when done */
void breakout_draw(void);  /* fills its OAM slots (bricks/ball/paddle) */
u8 breakout_left(void);    /* bricks remaining (debug) */

/* action.c */
void action_start(void);   /* OP_ACTION executed on the scene's stage */
void action_service(void); /* one frame (WAITING_ACTION) */
void action_draw(void);    /* enemies/bullets/ghosts into oam_shadow */

/* audio.c */
void music_boot(void);
void music_play(u8 id);
void music_stop(void);
void music_service(void); /* per-frame stream clock: loop / end */
u8 music_playing(void);   /* track + 1, 0 = silent (debug) */

/* obj.c (meters live with the other HUD OBJs) */
void meters_draw(void);

/* sfx.c */
void sfx_boot(void);
void sfx_play(u8 id);

/* debug.c */
void debug_flush(void);

#define FLAG_GET(i) ((g.flags >> (i)) & 1)
#define FLAG_SET(i) (g.flags |= (u16)(1 << (i)))
#define FLAG_CLR(i) (g.flags &= (u16)~(1 << (i)))

#endif
