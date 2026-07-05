// aot/compiler/text.ts — compile-time text layout + tokenization (cjk16).
//
// The runtimes never measure text: the compiler wraps each string to the
// target's textbox metrics, splits it into pages (one OP_TEXT per page), and
// encodes each page as the token stream from spec/pjgb.ts. Newlines inside a
// page are explicit TOK_NEWLINE bytes.

import {
  TOK_ASCII_MAX,
  TOK_ASCII_MIN,
  TOK_END,
  TOK_FULL_FLAG,
  TOK_NEWLINE,
  type TargetSpec,
} from "../spec/pjgb.ts";
import { isFullwidth } from "./cjk.ts";

/** Halfcell width of one char (1 = halfwidth, 2 = fullwidth). */
export function charCells(ch: string): number {
  return isFullwidth(ch) ? 2 : 1;
}

/** Halfcell width of a whole string. */
export function textCells(s: string): number {
  let n = 0;
  for (const ch of s) n += charCells(ch);
  return n;
}

/**
 * Wrap `text` to `cols` halfcells per line and split into pages of at most
 * `lines` lines. Explicit "\n" in the source is honored. Lines break at the
 * last position that fits (CJK breaks anywhere; no hyphenation for ASCII —
 * an over-long ASCII word breaks mid-word, which is fine for game dialogue).
 */
export function wrapPages(text: string, spec: Pick<TargetSpec, "textCols" | "textLines">): string[] {
  const lines: string[] = [];
  for (const src of text.split("\n")) {
    let line = "";
    let cells = 0;
    for (const ch of src) {
      const w = charCells(ch);
      if (cells + w > spec.textCols) {
        lines.push(line);
        line = "";
        cells = 0;
        if (ch === " ") continue; // do not start a wrapped line with a space
      }
      line += ch;
      cells += w;
    }
    lines.push(line);
  }
  const pages: string[] = [];
  for (let i = 0; i < lines.length; i += spec.textLines) {
    pages.push(lines.slice(i, i + spec.textLines).join("\n"));
  }
  return pages.length ? pages : [""];
}

/**
 * Encode one page into the cjk16 token stream. Non-ASCII chars intern a
 * fullwidth glyph id through `fullGlyphId` (char -> dense id).
 */
export function tokenize(page: string, fullGlyphId: (ch: string) => number): number[] {
  const out: number[] = [];
  for (const ch of page) {
    const cp = ch.codePointAt(0)!;
    if (ch === "\n") {
      out.push(TOK_NEWLINE);
    } else if (cp >= TOK_ASCII_MIN && cp <= TOK_ASCII_MAX) {
      out.push(cp);
    } else {
      const id = fullGlyphId(ch);
      if (id > 0x3fff) throw new Error(`glyph id ${id} out of range for "${ch}"`);
      out.push(TOK_FULL_FLAG | (id >> 8), id & 0xff);
    }
  }
  out.push(TOK_END);
  return out;
}
