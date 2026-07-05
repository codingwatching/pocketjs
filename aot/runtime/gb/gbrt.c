// aot/runtime/gb/gbrt.c — platform layer: video boot, map load, collision,
// player movement, camera, OAM scene, input, debug block.
#include <string.h>
#include "gbrt.h"

PjGame g;
PjActor pj_actors[BUDGET_MAX_ACTORS_PER_MAP];
PjWarp pj_warps[8];
u8 pj_script_ram[PJ_SCRIPT_BUF];

static const s8 DX[4] = {0, 0, -1, 1}; // down, up, left, right
static const s8 DY[4] = {1, -1, 0, 0};

#define PLAYER_SPEED 2
#define ANIM_RATE 6

// --- VRAM addressing (signed BG tile mode: LCDC.4 = 0) ----------------------
u8 *bg_tile_addr(u8 id) {
  if (id < 128) return (u8 *)(0x9000 + (u16)id * 16);
  return (u8 *)(0x8800 + (u16)(id - 128) * 16);
}

// --- boot --------------------------------------------------------------------
void video_boot(void) {
  DISPLAY_OFF;
  LCDC_REG = 0x00; // off; BG data signed, BG map 0x9800, WIN map 0x9C00, 8x16 OBJ
  BGP_REG = 0xE4;
  OBP0_REG = 0xE4;

  SWITCH_ROM(BANK(pj_bg_tiles));
  {
    u8 i;
    for (i = 0; i < PJ_BG_TILE_COUNT; i++) {
      memcpy(bg_tile_addr(i), pj_bg_tiles + (u16)i * 16, 16);
    }
  }
  SWITCH_ROM(BANK(pj_obj_tiles));
  set_sprite_data(0, PJ_OBJ_TILE_COUNT, pj_obj_tiles);

  SPRITES_8x16;
  WX_REG = 7;
  WY_REG = 144; // window offscreen until a textbox opens
}

// --- map ----------------------------------------------------------------------
void map_enter(u8 map_id, u8 tx, u8 ty, u8 dir) {
  const PjMapInfo *mi = &pj_maps[map_id];
  u8 lcd_was_on = (LCDC_REG & 0x80) ? 1 : 0;
  u8 i;

  if (lcd_was_on) DISPLAY_OFF;

  g.map_id = map_id;
  g.map_w = mi->w;
  g.map_h = mi->h;
  g.map_bank = mi->bank;
  g.map_tiles = mi->tiles;
  g.map_coll = mi->coll;
  g.n_actors = mi->n_actors;
  g.n_warps = mi->n_warps > 8 ? 8 : mi->n_warps;

  SWITCH_ROM(mi->bank);
  memcpy(pj_actors, mi->actors, sizeof(PjActor) * mi->n_actors);
  memcpy(pj_warps, mi->warps, sizeof(PjWarp) * g.n_warps);
  for (i = 0; i < g.n_actors; i++) {
    g.actor_dir[i] = pj_actors[i].facing;
    g.actor_frame[i] = 0;
  }

  // Whole BG map: map tiles inside, blank (0) outside.
  {
    u8 cx, cy;
    u8 *dst = (u8 *)0x9800;
    const u8 *row = g.map_tiles;
    for (cy = 0; cy < 32; cy++) {
      if (cy < g.map_h) {
        for (cx = 0; cx < 32; cx++) dst[cx] = (cx < g.map_w) ? row[cx] : 0;
        row += g.map_w;
      } else {
        memset(dst, 0, 32);
      }
      dst += 32;
    }
  }

  g.px = (s16)tx * 8;
  g.py = (s16)ty * 8;
  g.dir = dir;
  g.moving = 0;
  g.anim_frame = 0;
  g.anim_timer = 0;

  g.pending_enter = (mi->on_enter == 0xff) ? -1 : (s8)mi->on_enter;

  camera_update();
  SCX_REG = (u8)g.cam_x;
  SCY_REG = (u8)g.cam_y;

  // LCD on | WIN map 0x9C00 (bit6) | WIN off (bit5) | BG data signed (bit4=0)
  // | BG map 0x9800 (bit3=0) | OBJ 8x16 (bit2) | OBJ on | BG on
  LCDC_REG = 0xC7;
}

u8 map_solid(s16 tx, s16 ty) {
  u8 i;
  if (tx < 0 || ty < 0 || tx >= (s16)g.map_w || ty >= (s16)g.map_h) return 1;
  SWITCH_ROM(g.map_bank);
  if (g.map_coll[(u16)ty * g.map_w + (u16)tx]) return 1;
  for (i = 0; i < g.n_actors; i++) {
    if ((pj_actors[i].flags & ACTOR_FLAG_SOLID) && (s16)pj_actors[i].x == tx && (s16)pj_actors[i].y == ty) return 1;
  }
  return 0;
}

s8 map_actor_at(s16 tx, s16 ty) {
  u8 i;
  for (i = 0; i < g.n_actors; i++) {
    if ((s16)pj_actors[i].x == tx && (s16)pj_actors[i].y == ty) return (s8)i;
  }
  return -1;
}

// --- player ---------------------------------------------------------------------
void player_update(void) {
  if (!g.moving && !g.locked) {
    if (key_pressed(PJK_A)) {
      s16 tx = g.px >> 3, ty = g.py >> 3;
      s16 fx = tx + DX[g.dir], fy = ty + DY[g.dir];
      s8 slot = map_actor_at(fx, fy);
      if (slot >= 0 && pj_actors[(u8)slot].on_talk != PJGB_SCRIPT_NONE) {
        vm_start((u8)pj_actors[(u8)slot].on_talk, slot);
        return;
      }
    }
    {
      s8 dir = -1;
      if (key_held(PJK_DOWN)) dir = DIR_DOWN;
      else if (key_held(PJK_UP)) dir = DIR_UP;
      else if (key_held(PJK_LEFT)) dir = DIR_LEFT;
      else if (key_held(PJK_RIGHT)) dir = DIR_RIGHT;
      if (dir >= 0) {
        s16 nx, ny;
        g.dir = (u8)dir;
        nx = (g.px >> 3) + DX[g.dir];
        ny = (g.py >> 3) + DY[g.dir];
        if (!map_solid(nx, ny)) g.moving = 1;
      }
    }
  }

  if (g.moving) {
    g.px += DX[g.dir] * PLAYER_SPEED;
    g.py += DY[g.dir] * PLAYER_SPEED;
    if (++g.anim_timer >= ANIM_RATE) {
      g.anim_timer = 0;
      g.anim_frame++;
    }
    if (((g.px & 7) == 0) && ((g.py & 7) == 0)) {
      u8 i;
      s16 tx = g.px >> 3, ty = g.py >> 3;
      g.moving = 0;
      for (i = 0; i < g.n_warps; i++) {
        if ((s16)pj_warps[i].x == tx && (s16)pj_warps[i].y == ty) {
          map_enter(pj_warps[i].dest_map, (u8)pj_warps[i].dest_x, (u8)pj_warps[i].dest_y, pj_warps[i].dest_dir);
          return;
        }
      }
    }
  }
}

// --- camera ----------------------------------------------------------------------
static s16 clamp16(s16 v, s16 lo, s16 hi) {
  if (v < lo) return lo;
  if (v > hi) return hi;
  return v;
}

void camera_update(void) {
  s16 max_x = (s16)g.map_w * 8 - PJGB_SCREEN_W;
  s16 max_y = (s16)g.map_h * 8 - PJGB_SCREEN_H;
  if (max_x < 0) max_x = 0;
  if (max_y < 0) max_y = 0;
  g.cam_x = clamp16(g.px + 4 - PJGB_SCREEN_W / 2, 0, max_x);
  g.cam_y = clamp16(g.py + 4 - PJGB_SCREEN_H / 2, 0, max_y);
}

// --- OAM scene ---------------------------------------------------------------------
// 8x16 mode: each 16x16 character = 2 hardware sprites (left, right halfcell).
static void put_char_sprite(u8 hw, u16 tile, s16 sx, s16 sy) {
  if (sx <= -16 || sx >= PJGB_SCREEN_W || sy <= -16 || sy >= PJGB_SCREEN_H) {
    move_sprite(hw, 0, 0);
    move_sprite(hw + 1, 0, 0);
    return;
  }
  set_sprite_tile(hw, (u8)tile);
  set_sprite_tile(hw + 1, (u8)(tile + 2));
  move_sprite(hw, (u8)(sx + 8), (u8)(sy + 16));
  move_sprite(hw + 1, (u8)(sx + 16), (u8)(sy + 16));
}

void scene_draw(void) {
  u8 i;
  // player (sprite id 0) -> hw sprites 0,1
  {
    const PjSprite *sp = &pj_sprites[0];
    u8 frames = sp->frames ? sp->frames : 1;
    u16 tile = sp->tile_base + (u16)g.dir * frames * 4 + (u16)(g.anim_frame % frames) * 4;
    put_char_sprite(0, tile, g.px - g.cam_x - 4, g.py - g.cam_y - 8);
  }
  for (i = 0; i < g.n_actors; i++) {
    u8 hw = 2 + i * 2;
    if (pj_actors[i].sprite == 0xff) {
      move_sprite(hw, 0, 0);
      move_sprite(hw + 1, 0, 0);
      continue;
    }
    {
      const PjSprite *sp = &pj_sprites[pj_actors[i].sprite];
      u8 frames = sp->frames ? sp->frames : 1;
      u16 tile = sp->tile_base + (u16)g.actor_dir[i] * frames * 4 + (u16)(g.actor_frame[i] % frames) * 4;
      put_char_sprite(hw, tile, (s16)pj_actors[i].x * 8 - g.cam_x - 4, (s16)pj_actors[i].y * 8 - g.cam_y - 8);
    }
  }
  // hide the rest
  for (i = 2 + g.n_actors * 2; i < 40; i++) move_sprite(i, 0, 0);
}

// --- input ------------------------------------------------------------------------
void input_poll(void) {
  u8 j = joypad();
  u8 k = 0;
  if (j & J_A) k |= PJK_A;
  if (j & J_B) k |= PJK_B;
  if (j & J_SELECT) k |= PJK_SELECT;
  if (j & J_START) k |= PJK_START;
  if (j & J_RIGHT) k |= PJK_RIGHT;
  if (j & J_LEFT) k |= PJK_LEFT;
  if (j & J_UP) k |= PJK_UP;
  if (j & J_DOWN) k |= PJK_DOWN;
  g.keys_prev = g.keys;
  g.keys = k;
}

u8 key_held(u8 mask) { return (g.keys & mask) != 0; }
u8 key_pressed(u8 mask) { return (g.keys & mask) != 0 && (g.keys_prev & mask) == 0; }

// --- debug block ---------------------------------------------------------------------
#define DBG8(off) (*(volatile u8 *)(PJGB_DEBUG_ADDR + (off)))
#define DBG16(off) (*(volatile u16 *)(PJGB_DEBUG_ADDR + (off)))

void debug_flush(void) {
  u8 i;
  DBG16(DBG_MAGIC) = (u16)(DEBUG_MAGIC & 0xffff);
  DBG16(DBG_MAGIC + 2) = (u16)((u32)DEBUG_MAGIC >> 16);
  DBG16(DBG_PLAYER_X) = (u16)(g.px >> 3);
  DBG16(DBG_PLAYER_Y) = (u16)(g.py >> 3);
  DBG8(DBG_PLAYER_DIR) = g.dir;
  DBG8(DBG_CUR_MAP) = g.map_id;
  DBG8(DBG_TEXT_ACTIVE) = g.text_active;
  DBG8(DBG_SCRIPT_ACTIVE) = vm_active();
  DBG16(DBG_FRAME) = g.frame;
  DBG16(DBG_FRAME + 2) = 0;
  DBG16(DBG_CUR_TEXT) = g.text_active ? g.cur_text : 0xFFFF;
  DBG8(DBG_CHOICE_CURSOR) = g.choice_cursor;
  DBG8(DBG_BOOTED) = 1;
  for (i = 0; i < 16; i++) DBG8(DBG_FLAGS + i) = g.flags[i];
  for (i = 0; i < 16; i++) DBG16(DBG_VARS + i * 2) = (u16)g.vars[i];
}
