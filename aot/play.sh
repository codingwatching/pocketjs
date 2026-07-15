#!/usr/bin/env bash
# aot/play.sh — build the demo ROM from TSX and open it in a target emulator so
# you can play it yourself with the keyboard.
#
#   bash aot/play.sh                         # GBA 神雕旧事 demo (legacy form)
#   bash aot/play.sh gba|gb|nes              # select a target
#   bash aot/play.sh gba|gb|nes path/to/game.tsx
#   bash aot/play.sh path/to/game.tsx        # custom GBA game (legacy form)
#
# Controls (mGBA defaults):
#   Arrow keys .... walk
#   X ............. A  (talk / confirm / advance text / pick menu item)
#   Z ............. B  (cancel)
#   Enter ......... Start        Backspace ... Select
set -euo pipefail

CALLER_PWD="$PWD"
cd "$(cd "$(dirname "$0")/.." && pwd)" # repo root

TARGET="gba"
ENTRY="aot/demo-shendiao/game.tsx"
ROM_NAME="shendiao"
CUSTOM_ENTRY=0

if [[ $# -gt 0 ]]; then
  case "$1" in
    gba|gb|nes)
      TARGET="$1"
      shift
      ;;
    *)
      ENTRY="$1"
      CUSTOM_ENTRY=1
      shift
      ;;
  esac
fi

if [[ $# -gt 0 ]]; then
  ENTRY="$1"
  CUSTOM_ENTRY=1
  shift
fi

if [[ $# -gt 0 ]]; then
  echo "usage: bash aot/play.sh [gba|gb|nes] [path/to/game.tsx]" >&2
  exit 2
fi

if [[ "$ENTRY" != /* && -f "$CALLER_PWD/$ENTRY" ]]; then
  ENTRY="$CALLER_PWD/$ENTRY"
fi

if [[ "$CUSTOM_ENTRY" -eq 0 ]]; then
  echo "▸ Generating 神雕旧事 assets"
  bun aot/demo-shendiao/imagegen/build-assets.ts
else
  ENTRY_DIR="$(basename "$(dirname "$ENTRY")")"
  case "$ENTRY_DIR" in
    demo-*) ROM_NAME="${ENTRY_DIR#demo-}" ;;
    demo) ROM_NAME="pocket-town" ;;
    *) ROM_NAME="$ENTRY_DIR" ;;
  esac
fi

ROM="$PWD/aot/dist/$ROM_NAME.$TARGET"

echo "▸ Compiling $ENTRY → $ROM"
bun aot/compiler/cli.ts build "$ENTRY" --target "$TARGET" --out "$ROM"

if [[ "$TARGET" == "nes" ]]; then
  if ! command -v fceux >/dev/null 2>&1; then
    echo "error: FCEUX is required to play NES ROMs (macOS: brew install fceux)" >&2
    exit 1
  fi
  echo "▸ Launching FCEUX"
  fceux "$ROM"
  exit 0
fi

APP=""
if command -v brew >/dev/null 2>&1; then
  MGBA_PREFIX="$(brew --prefix mgba 2>/dev/null || true)"
  if [[ -n "$MGBA_PREFIX" ]]; then
    APP="$MGBA_PREFIX/mGBA.app"
  fi
fi

echo "▸ Launching mGBA — arrows to walk, X = A (talk/confirm), Z = B, Enter = Start"
if [[ -d "$APP" ]] && command -v open >/dev/null 2>&1; then
  open -n "$APP" --args "$ROM"
elif command -v mgba >/dev/null 2>&1; then
  mgba "$ROM"
else
  echo "error: mGBA is required to play GBA/GB ROMs (macOS: brew install mgba)" >&2
  exit 1
fi
