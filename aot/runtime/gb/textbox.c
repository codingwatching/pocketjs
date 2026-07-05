// aot/runtime/gb/textbox.c — WINDOW-layer textbox + choice menu (cjk16 only).
//
// DMG VRAM is only reliably writable during vblank, so all rendering goes
// through a per-frame "pump" (textbox_pump, called right after
// wait_vbl_done): first the box rows are filled, then glyph halfcells are
// streamed 2 per frame — a natural typewriter reveal. Glyph pixel data is
// copied from banked ROM into the reserved BG tile slot region
// (PJ_SLOT_BASE..), and the window map points at the freshly-written tiles.
//
// Choice menus reserve slot 0 for the '>' cursor so moving the cursor is two
// map-byte writes, not a re-render.
#include <string.h>
#include "gbrt.h"

#define WIN_MAP ((u8 *)0x9C00)
#define WIN_COLS 20

// text layout inside the window (rows are window-map rows)
#define TB_TEXT_ROW0 1
#define TB_TEXT_COL0 1
#define TB_CHOICE_CURSOR_COL 1
#define TB_CHOICE_TEXT_COL 3

#define MAX_JOBS 5
#define TOKBUF 160

typedef struct {
  u16 text_id;
  u8 row, col;
} TbJob;

static TbJob jobs[MAX_JOBS];
static u8 n_jobs, cur_job;
static u8 tokbuf[TOKBUF];
static u8 tok_pos, tok_loaded;
static u8 cur_row, cur_col;
static u8 box_rows, fill_row;
static u8 pump_done;
static u8 cursor_slot_ready;
static u8 cursor_row_prev;

static void load_tokens(u16 text_id) {
  u16 off;
  u8 i;
  SWITCH_ROM(BANK(pj_texts));
  off = pj_text_offs[text_id];
  for (i = 0; i < TOKBUF - 1; i++) {
    tokbuf[i] = pj_texts[off + i];
    if (tokbuf[i] == TOK_END) break;
  }
  tokbuf[TOKBUF - 1] = TOK_END;
  tok_pos = 0;
  tok_loaded = 1;
}

// Copy one halfcell (2 tiles = 32 bytes) into the next slot and point the
// window map cell (row, col) at it. set_bkg_data/set_win_tile_xy are the
// GBDK STAT-aware writers, safe with the LCD on (DMG drops plain writes
// during mode 3, which shows up as missing glyph rows).
static void draw_halfcell_from(const u8 *src_banked, u8 bank, u8 row, u8 col, u16 slot) {
  u8 tile = (u8)(PJ_SLOT_BASE + slot * 2);
  SWITCH_ROM(bank);
  set_bkg_data(tile, 2, src_banked);
  set_win_tile_xy(col, row, tile);
  set_win_tile_xy(col, row + 1, tile + 1);
}

static u16 alloc_slot(void) {
  u16 s = g.slot_next;
  if (s * 2 + 2 > (u16)PJGB_TEXT_GLYPH_SLOTS * 2) return 0; // overflow: reuse 0
  g.slot_next++;
  return s;
}

// Draw the next halfcell token; returns 0 when the current stream is done.
static u8 pump_token(void) {
  u8 tok;
  if (!tok_loaded) return 0;
  tok = tokbuf[tok_pos];
  if (tok == TOK_END) return 0;
  tok_pos++;
  if (tok == TOK_NEWLINE) {
    cur_row += 2;
    cur_col = jobs[cur_job].col;
    return 1;
  }
  if (tok & TOK_FULL_FLAG) {
    u16 id = (((u16)(tok & 0x3f)) << 8) | tokbuf[tok_pos++];
    u16 off = id << 6; // 64 bytes per fullwidth glyph
    u16 s0 = alloc_slot(), s1 = alloc_slot();
    draw_halfcell_from(pj_glyphs_full + off, BANK(pj_glyphs_full), cur_row, cur_col, s0);
    draw_halfcell_from(pj_glyphs_full + off + 32, BANK(pj_glyphs_full), cur_row, cur_col + 1, s1);
    cur_col += 2;
  } else {
    u16 s0 = alloc_slot();
    draw_halfcell_from(pj_glyphs_half + (u16)(tok - TOK_ASCII_MIN) * 32, BANK(pj_glyphs_half), cur_row, cur_col, s0);
    cur_col += 1;
  }
  return 1;
}

static void open_box(u8 rows) {
  box_rows = rows;
  fill_row = 0;
  pump_done = 0;
  cur_job = 0;
  tok_loaded = 0;
  g.slot_next = 0;
  cursor_slot_ready = 0;
  WY_REG = (u8)((PJGB_SCREEN_TILES_H - rows) * 8);
  WX_REG = 7;
  LCDC_REG |= 0x20; // window on
}

void textbox_init(void) {
  g.text_active = 0;
  g.choice_active = 0;
  g.choice_result = -1;
  pump_done = 1;
  n_jobs = 0;
}

void textbox_show(u16 text_id) {
  g.cur_text = text_id;
  g.text_active = 1;
  g.choice_n = 0;
  n_jobs = 1;
  jobs[0].text_id = text_id;
  jobs[0].row = TB_TEXT_ROW0;
  jobs[0].col = TB_TEXT_COL0;
  open_box((u8)(PJGB_TEXT_LINES * 2 + 2));
}

void textbox_hide(void) {
  g.text_active = 0;
  n_jobs = 0;
  pump_done = 1;
  LCDC_REG &= ~0x20; // window off
  WY_REG = 144;
}

u8 textbox_active(void) { return g.text_active; }

void textbox_tick(void) {
  if (g.text_active && !g.choice_active && key_pressed(PJK_A)) textbox_hide();
}

// --- choice ------------------------------------------------------------------
void choice_show(u8 n, const u16 *text_ids) {
  u8 i;
  g.choice_active = 1;
  g.choice_n = n;
  g.choice_cursor = 0;
  g.choice_result = -1;
  for (i = 0; i < n && i < 8; i++) g.choice_ids[i] = text_ids[i];
  g.text_active = 1; // box is on screen (parity with GBA semantics)
  n_jobs = n;
  for (i = 0; i < n; i++) {
    jobs[i].text_id = g.choice_ids[i];
    jobs[i].row = (u8)(TB_TEXT_ROW0 + i * 2);
    jobs[i].col = TB_CHOICE_TEXT_COL;
  }
  open_box((u8)(n * 2 + 2));
  g.slot_next = 1; // slot 0 is reserved for the cursor glyph
  cursor_row_prev = TB_TEXT_ROW0;
}

u8 choice_active(void) { return g.choice_active; }
s8 choice_result(void) { return g.choice_result; }

static void cursor_cells(u8 row, u8 top, u8 bottom) {
  set_win_tile_xy(TB_CHOICE_CURSOR_COL, row, top);
  set_win_tile_xy(TB_CHOICE_CURSOR_COL, row + 1, bottom);
}

void choice_tick(void) {
  if (!g.choice_active) return;
  if (key_pressed(PJK_UP) && g.choice_cursor > 0) g.choice_cursor--;
  else if (key_pressed(PJK_DOWN) && g.choice_cursor < g.choice_n - 1) g.choice_cursor++;
  if (key_pressed(PJK_A)) {
    g.choice_result = (s8)g.choice_cursor;
    g.choice_active = 0;
    textbox_hide();
  }
}

// --- the vblank pump -----------------------------------------------------------
void textbox_pump(void) {
  u8 budget;
  if (!g.text_active && !g.choice_active) return;

  // 1) box fill: two rows per frame
  for (budget = 0; budget < 2 && fill_row < box_rows; budget++) {
    fill_win_rect(0, fill_row, WIN_COLS, 1, PJ_BOX_TILE);
    fill_row++;
  }
  if (fill_row < box_rows) return;

  // 2) choice cursor: draw the '>' glyph once into slot 0, then track moves
  if (g.choice_active || g.choice_n) {
    if (g.choice_active && !cursor_slot_ready) {
      draw_halfcell_from(pj_glyphs_half + (u16)('>' - TOK_ASCII_MIN) * 32, BANK(pj_glyphs_half),
                         (u8)(TB_TEXT_ROW0 + g.choice_cursor * 2), TB_CHOICE_CURSOR_COL, 0);
      cursor_slot_ready = 1;
      cursor_row_prev = (u8)(TB_TEXT_ROW0 + g.choice_cursor * 2);
    } else if (g.choice_active && cursor_slot_ready) {
      u8 row = (u8)(TB_TEXT_ROW0 + g.choice_cursor * 2);
      if (row != cursor_row_prev) {
        cursor_cells(cursor_row_prev, PJ_BOX_TILE, PJ_BOX_TILE); // erase old
        cursor_cells(row, PJ_SLOT_BASE, PJ_SLOT_BASE + 1); // slot 0 tiles
        cursor_row_prev = row;
      }
    }
  }

  // 3) glyph streaming: two halfcells per frame
  if (pump_done) return;
  for (budget = 0; budget < 2;) {
    if (!tok_loaded) {
      if (cur_job >= n_jobs) {
        pump_done = 1;
        return;
      }
      load_tokens(jobs[cur_job].text_id);
      cur_row = jobs[cur_job].row;
      cur_col = jobs[cur_job].col;
    }
    if (!pump_token()) {
      tok_loaded = 0;
      cur_job++;
      continue;
    }
    budget++;
  }
}
