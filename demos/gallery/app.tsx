// demos/gallery/app.tsx — static bitmap gallery with PSP-native 3D LR flips.

import { createSignal } from "solid-js";
import {
  ActionBar,
  Image,
  SceneTransition3D,
  Screen,
  Text,
  View,
  type NodeMirror,
} from "@pocketjs/framework/components";
import { animate } from "@pocketjs/framework/animation";
import { onButtonPress } from "@pocketjs/framework/lifecycle";
import { BTN } from "@pocketjs/framework/input";
import { TILE_SRCS } from "./tiles.ts";

const TILE_LABEL = [
  "OUTRUN", "NEON", "MIRAGE", "PULSE", "CHROME", "MIDNIGHT",
  "EMBER", "DUSK", "AMBER", "SANDS", "COPPER", "FLARE",
  "FERN", "MOSS", "PINE", "JADE", "TIDE", "GROVE",
  "QUASAR", "COMET", "ORBIT", "VIOLET", "NOVA", "DRIFT",
];

const TILE_GROUP = ["SYNTHWAVE", "GOLDEN HOUR", "EVERGREEN", "NEBULA"];
const TILE_SUB = ["neon coast drive", "warm analog haze", "deep forest floor", "far outer dark"];
const PAGE_SIZE = 6;
const PAGE_COUNT = Math.ceil(TILE_SRCS.length / PAGE_SIZE);
const FLIP_MS = 860;

function wrapIndex(index: number): number {
  return (index + PAGE_COUNT) % PAGE_COUNT;
}

function tileIndex(page: number, cell: number): number {
  return (page * PAGE_SIZE + cell) % TILE_SRCS.length;
}

function tileSrc(page: number, cell: number): string {
  return TILE_SRCS[tileIndex(page, cell)];
}

function cellDelay(cell: number, direction: number): number {
  const col = cell % 3;
  const row = Math.floor(cell / 3);
  const sweep = direction < 0 ? 2 - col : col;
  return sweep * 55 + row * 30;
}

export default function GalleryDemo() {
  const [page, setPage] = createSignal(0);
  const [fromPage, setFromPage] = createSignal(0);
  const [toPage, setToPage] = createSignal(0);
  const [direction, setDirection] = createSignal(1);
  const [progress, setProgress] = createSignal(1);
  let current = 0;
  const stages: (NodeMirror | undefined)[] = [];

  const flip = (delta: number): void => {
    const next = wrapIndex(current + delta);
    if (next === current) return;
    const previous = current;
    current = next;
    setFromPage(previous);
    setToPage(next);
    setDirection(delta < 0 ? -1 : 1);
    setProgress(0);
    setPage(next);
    for (let cell = 0; cell < PAGE_SIZE; cell++) {
      const stage = stages[cell];
      if (stage) {
        animate(stage, "flipProgress", 1, {
          dur: FLIP_MS,
          easing: "in-out",
          delay: cellDelay(cell, delta),
        });
      }
    }
  };

  onButtonPress(BTN.LTRIGGER, () => flip(-1));
  onButtonPress(BTN.RTRIGGER, () => flip(1));

  const count = () => String(page() + 1).padStart(2, "0") + " / " + String(PAGE_COUNT).padStart(2, "0");

  return (
    <Screen class="relative w-full h-full bg-slate-950 overflow-hidden">
      <View class="absolute inset-0 bg-gradient-to-b from-slate-900 to-black" />
      <View class="relative flex-col w-full h-full items-center px-5 pt-3 pb-9">
        <View class="w-full flex-row items-end justify-between">
          <View class="flex-col">
            <Text class="text-xs text-cyan-300 tracking-wide">{TILE_SUB[page()]}</Text>
            <Text class="text-xl text-white font-bold">{TILE_GROUP[page()]}</Text>
          </View>
          <Text class="text-xs text-slate-400">{count()}</Text>
        </View>

        <View class="grow w-full flex-row flex-wrap items-center justify-center gap-2">
          {[0, 1, 2, 3, 4, 5].map((cell) => (
            <View class="w-[136] h-[86] rounded-lg border-slate-700 bg-slate-900 shadow-md p-2 flex-row items-center gap-2">
              <View class="w-[58] h-[58] rounded-md bg-black border-cyan-900 items-center justify-center">
                <SceneTransition3D
                  nodeRef={(node) => {
                    stages[cell] = node;
                  }}
                  class="w-[52] h-[52] rounded-md"
                  from={tileSrc(fromPage(), cell)}
                  to={tileSrc(toPage(), cell)}
                  direction={direction()}
                  progress={progress()}
                />
              </View>
              <View class="flex-col flex-1">
                <Text class="text-xs text-slate-400">{String(tileIndex(page(), cell) + 1).padStart(2, "0")}</Text>
                <Text class="text-sm text-white font-bold">{TILE_LABEL[tileIndex(page(), cell)]}</Text>
              </View>
            </View>
          ))}
        </View>

        <View class="w-full flex-row items-center justify-center gap-2 pb-1">
          {TILE_GROUP.map((name, groupIndex) => (
            <View
              class={
                groupIndex === page()
                  ? "w-[84] h-[12] rounded-md bg-cyan-300"
                  : "w-[84] h-[12] rounded-md bg-slate-700"
              }
            />
          ))}
        </View>
      </View>

      <ActionBar class="absolute left-3 right-3 bottom-2 flex-row items-center justify-between px-3 py-1 rounded-lg shadow-md bg-slate-900 border-slate-700">
        <Text class="text-xs text-slate-400">L / R PAGE FLIP</Text>
        <Text class="text-xs text-slate-400">6 STATIC BITMAPS</Text>
      </ActionBar>
    </Screen>
  );
}
