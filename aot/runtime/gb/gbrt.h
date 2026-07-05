// aot/runtime/gb/gbrt.h — internal contract for the PocketJS-AOT Game Boy
// runtime (GBDK-2020 / SDCC, DMG-compatible).
//
// MEMORY / VRAM PLAN:
//   LCDC: BG on @ 0x9800, WINDOW (textbox) @ 0x9C00, OBJ 8x16.
//   BG tile data uses SIGNED addressing (LCDC.4 = 0): ids 0..127 -> 0x9000,
//   ids 128..255 -> 0x8800. OBJ tiles own 0x8000..0x87FF (128 tiles).
//   BG ids: 0 = blank, 1.. tileset, PJ_BOX_TILE, then PJ_SLOT_BASE.. glyph
//   slots (2 tiles per halfcell), all sized by the compiler.
//
// DATA ACCESS: all game data lives in autobanked ROM (gen/*.c). Every read
// switches with SWITCH_ROM(BANK(sym)). Actors/warps of the current map are
// copied to WRAM on map_enter; script bytecode is copied to a WRAM buffer on
// vm_start, so the VM never touches banks mid-run.
#ifndef PJ_GBRT_H
#define PJ_GBRT_H

#include <gbdk/platform.h>
#include <stdint.h>
#include "pjgb_gen.h"
#include "gen_data.h"

typedef uint8_t u8;
typedef uint16_t u16;
typedef int8_t s8;
typedef int16_t s16;
typedef uint32_t u32;

// --- keys (PJ layout, matches the GBA runtime + harness expectations) -------
#define PJK_A 0x01
#define PJK_B 0x02
#define PJK_SELECT 0x04
#define PJK_START 0x08
#define PJK_RIGHT 0x10
#define PJK_LEFT 0x20
#define PJK_UP 0x40
#define PJK_DOWN 0x80

// --- VM ----------------------------------------------------------------------
enum { VM_SUSP_NONE = 0, VM_SUSP_TEXT, VM_SUSP_CHOICE, VM_SUSP_WAIT };

typedef struct {
  u8 active;
  u8 suspend;
  u16 ip;
  s16 stack[PJGB_VM_MAX_STACK];
  u8 sp;
  u16 wait_frames;
  s8 actor_slot;
} PjVm;

// --- global game state --------------------------------------------------------
typedef struct {
  u8 map_id;
  u8 map_w, map_h;
  u8 map_bank; // bank of the current map's tiles/coll arrays
  const u8 *map_tiles;
  const u8 *map_coll;
  u8 n_actors, n_warps;
  u8 on_enter;

  s16 px, py; // player pixel position (world)
  u8 dir;
  u8 moving;
  u8 anim_frame, anim_timer;
  u8 locked;

  s16 cam_x, cam_y;

  u8 actor_dir[BUDGET_MAX_ACTORS_PER_MAP];
  u8 actor_frame[BUDGET_MAX_ACTORS_PER_MAP];

  PjVm vm;
  s8 pending_enter;

  u8 text_active;
  u16 cur_text;
  u8 choice_active;
  u8 choice_n;
  u16 choice_ids[8];
  u8 choice_cursor;
  s8 choice_result;

  u8 flags[16];
  s16 vars[16];

  u16 slot_next;
  u16 rng;

  u8 keys, keys_prev;
  u16 frame;
} PjGame;

extern PjGame g;
// WRAM copies of the current map's entities + the running script.
extern PjActor pj_actors[BUDGET_MAX_ACTORS_PER_MAP];
extern PjWarp pj_warps[8];
extern u8 pj_script_ram[PJ_SCRIPT_BUF];

// --- gbrt.c -------------------------------------------------------------------
void video_boot(void);
void map_enter(u8 map_id, u8 tx, u8 ty, u8 dir);
u8 map_solid(s16 tx, s16 ty);
s8 map_actor_at(s16 tx, s16 ty);
void player_update(void);
void camera_update(void);
void scene_draw(void); // OAM shadow for player + actors
void input_poll(void);
u8 key_held(u8 mask);
u8 key_pressed(u8 mask);
void debug_flush(void);

// VRAM helpers (safe contexts only: vblank pump or LCD off)
u8 *bg_tile_addr(u8 id);

// --- vm.c ---------------------------------------------------------------------
void vm_start(u8 script_id, s8 actor_slot);
void vm_tick(void);
u8 vm_active(void);

// --- textbox.c ------------------------------------------------------------------
void textbox_init(void);
void textbox_show(u16 text_id);
void textbox_hide(void);
u8 textbox_active(void);
void textbox_tick(void);
void textbox_pump(void); // call right after wait_vbl_done(): streams VRAM work
void choice_show(u8 n, const u16 *text_ids);
u8 choice_active(void);
void choice_tick(void);
s8 choice_result(void);

// --- flags ---------------------------------------------------------------------
#define flag_get(id) ((g.flags[(id) >> 3] >> ((id) & 7)) & 1)
#define flag_set1(id) (g.flags[(id) >> 3] |= (u8)(1 << ((id) & 7)))
#define flag_set0(id) (g.flags[(id) >> 3] &= (u8)~(1 << ((id) & 7)))

#endif
