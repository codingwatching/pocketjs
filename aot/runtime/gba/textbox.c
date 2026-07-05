// aot/runtime/gba/textbox.c — BG1 textbox + choice menu (screenblock PJ_TEXT_SBB).
//
// Two text renderers, selected by GameHeader.text_mode:
//   ascii8 — legacy: 8x8 ASCII glyphs baked as static BG tiles (font_base).
//   cjk16  — 16px lines: glyph pixel data lives in the GLYPHS chunk and is
//            streamed on demand into a reserved BG tile region ("slots", 1
//            halfcell = 2 stacked tiles). The compiler pre-wraps/paginates
//            text, so a token stream always fits one page.
#include "runtime.h"

// ascii8 metrics (historic layout, unchanged)
#define BOX_ROW0 12
#define BOX_ROW1 19
#define TEXT_COL0 1
#define TEXT_ROW0 13
#define TEXT_COLMAX 28

// cjk16 metrics: text lines are 2 tile rows tall inside the same box rows.
#define C16_COL0 1
#define C16_ROW0 13
#define C16_CHOICE_ROW0 12
#define C16_CHOICE_COL_CURSOR 1
#define C16_CHOICE_COL_TEXT 3

const char *text_get(int text_id) {
  const u8 *chunk = cart_chunk(CHUNK_TEXT_BANK, 0, 0);
  // u16 count, u16 rsv, u32 offsets[count] (from chunk start), then strings.
  const u32 *offs = (const u32 *)(chunk + 4);
  return (const char *)(chunk + offs[text_id]);
}

static int cjk16(void) { return g.game->text_mode == TEXT_MODE_CJK16; }

static void box_fill(void) {
  u16 *sb = SCREENBLOCK(PJ_TEXT_SBB);
  for (int row = BOX_ROW0; row <= BOX_ROW1; row++)
    for (int col = 0; col < 30; col++)
      sb[row * 32 + col] = SE(g.game->box_tile, 15);
  g.slot_next = 0;
}

static void put_char(u16 *sb, int row, int col, unsigned char c) {
  if (c >= 0x20) sb[row * 32 + col] = SE(g.game->font_base + (c - 0x20), 15);
}

// --- cjk16 glyph streaming ---------------------------------------------------

// Copy one halfcell (2 stacked tiles) from the GLYPHS chunk into the next
// free VRAM slot and place it at (rowTop, col). Silently drops on overflow
// (the compiler sizes pages so this cannot happen).
static void draw_halfcell(u32 glyph_off, int rowTop, int col) {
  u16 *sb;
  if (g.slot_next * 2 + 2 > g.game->glyph_slot_count) return;
  {
    const u8 *store = cart_chunk(CHUNK_GLYPHS, 0, 0);
    int tile = g.game->glyph_slot_base + g.slot_next * 2;
    const u16 *src = (const u16 *)(store + glyph_off);
    u16 *dst = CHARBLOCK(PJ_BG_CBB) + tile * (PJGB_TILE_4BPP_BYTES / 2);
    for (int i = 0; i < PJGB_TILE_4BPP_BYTES; i++) dst[i] = src[i]; // 2 tiles
    sb = SCREENBLOCK(PJ_TEXT_SBB);
    sb[rowTop * 32 + col] = SE(tile, 15);
    sb[(rowTop + 1) * 32 + col] = SE(tile + 1, 15);
    g.slot_next++;
  }
}

static u32 half_glyph_off(int id) {
  return PJGB_GLYPH_STORE_HEADER_SIZE + (u32)id * 2 * PJGB_TILE_4BPP_BYTES;
}
static u32 full_glyph_off(int id, int half) {
  const u8 *store = cart_chunk(CHUNK_GLYPHS, 0, 0);
  u16 half_count = *(const u16 *)store;
  return PJGB_GLYPH_STORE_HEADER_SIZE +
         ((u32)half_count * 2 + (u32)id * 4 + (u32)half * 2) * PJGB_TILE_4BPP_BYTES;
}

// Render a whole token stream starting at (rowTop, col0). Newline advances
// one 16px line. The compiler guarantees the page fits.
static void render_tokens(const u8 *t, int rowTop, int col0) {
  int row = rowTop, col = col0;
  while (*t) {
    u8 tok = *t++;
    if (tok == TOK_NEWLINE) {
      row += 2;
      col = col0;
      continue;
    }
    if (tok & TOK_FULL_FLAG) {
      int id = ((tok & 0x3f) << 8) | *t++;
      draw_halfcell(full_glyph_off(id, 0), row, col);
      draw_halfcell(full_glyph_off(id, 1), row, col + 1);
      col += 2;
    } else {
      draw_halfcell(half_glyph_off(tok - TOK_ASCII_MIN), row, col);
      col += 1;
    }
  }
}

// --- textbox -----------------------------------------------------------------

void textbox_init(void) {
  g.text_active = 0;
  g.choice_active = 0;
  g.choice_result = -1;
}

void textbox_show(int text_id) {
  g.cur_text = (u16)text_id;
  g.text_active = 1;

  box_fill();

  if (cjk16()) {
    render_tokens((const u8 *)text_get(text_id), C16_ROW0, C16_COL0);
  } else {
    u16 *sb = SCREENBLOCK(PJ_TEXT_SBB);
    const char *t = text_get(text_id);
    int col = TEXT_COL0, row = TEXT_ROW0;
    for (const char *c = t; *c; c++) {
      if (*c == '\n') {
        row++;
        col = TEXT_COL0;
        if (row > BOX_ROW1) break;
        continue;
      }
      if (col > TEXT_COLMAX) {
        row++;
        col = TEXT_COL0;
      }
      if (row > BOX_ROW1) break;
      put_char(sb, row, col, (unsigned char)*c);
      col++;
    }
  }

  REG_DISPCNT |= DCNT_BG1;
}

void textbox_hide(void) {
  g.text_active = 0;
  REG_DISPCNT &= ~DCNT_BG1;
}

int textbox_active(void) { return g.text_active; }

void textbox_tick(void) {
  if (g.text_active && !g.choice_active && key_pressed(KEY_A)) textbox_hide();
}

// --- choice menu -----------------------------------------------------------
static void choice_render(void) {
  box_fill();
  if (cjk16()) {
    for (int i = 0; i < g.choice_n; i++) {
      int row = C16_CHOICE_ROW0 + i * 2;
      if (i == g.choice_cursor) draw_halfcell(half_glyph_off('>' - 0x20), row, C16_CHOICE_COL_CURSOR);
      render_tokens((const u8 *)text_get(g.choice_ids[i]), row, C16_CHOICE_COL_TEXT);
    }
    return;
  }
  {
    u16 *sb = SCREENBLOCK(PJ_TEXT_SBB);
    for (int i = 0; i < g.choice_n; i++) {
      int row = TEXT_ROW0 + i;
      if (row > BOX_ROW1) break;
      if (i == g.choice_cursor) put_char(sb, row, TEXT_COL0, '>');
      const char *t = text_get(g.choice_ids[i]);
      int col = TEXT_COL0 + 2;
      for (const char *c = t; *c && col <= TEXT_COLMAX; c++, col++)
        put_char(sb, row, col, (unsigned char)*c);
    }
  }
}

void choice_show(int n, const u16 *text_ids) {
  g.choice_active = 1;
  g.choice_n = (u8)n;
  g.choice_cursor = 0;
  g.choice_result = -1;
  for (int i = 0; i < n && i < 8; i++) g.choice_ids[i] = text_ids[i];
  choice_render();
  REG_DISPCNT |= DCNT_BG1;
}

int choice_active(void) { return g.choice_active; }

int choice_result(void) { return g.choice_result; }

void choice_tick(void) {
  if (!g.choice_active) return;
  if (key_pressed(KEY_UP) && g.choice_cursor > 0) {
    g.choice_cursor--;
    choice_render();
  } else if (key_pressed(KEY_DOWN) && g.choice_cursor < g.choice_n - 1) {
    g.choice_cursor++;
    choice_render();
  }
  if (key_pressed(KEY_A)) {
    g.choice_result = g.choice_cursor;
    g.choice_active = 0;
    textbox_hide();
  }
}
