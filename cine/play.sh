#!/usr/bin/env bash
# cine/play.sh — build 《渐进人生》 and open it in mGBA.
#   bash cine/play.sh [film.ts]
set -euo pipefail
cd "$(dirname "$0")"
FILM="${1:-film/progressive-life.ts}"
OUT="dist/$(basename "${FILM%.*}").gba"
bun compiler/cli.ts build "$FILM" --out "$OUT" --title PROGLIFE
MGBA_APP="$(brew --prefix mgba 2>/dev/null)/mGBA.app"
if [ -d "$MGBA_APP" ]; then
  open -n "$MGBA_APP" --args "$PWD/$OUT"
else
  mgba "$OUT"
fi
