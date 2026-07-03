// demos/settings.tsx — "settings menu" showcase: a grouped list of system
// toggles (spring-sliding pill knobs — first demo use of `rounded-full`,
// build-time-fixed w/h per DESIGN.md so the compiler can bake the exact
// corner radius), a brightness control CIRCLE cycles through 5 steps (a
// spring width tween, same mechanism as stats.tsx's bars but driven by
// presses instead of mount), and a row of theme swatches whose SELECTED
// state (a persistent signal) is a separate visual layer from FOCUSED (the
// transient native focus: variant) — pressing one live-recolors the header
// title, the simplest possible demonstration of cross-component reactivity.
//
// No custom frame wiring: every interaction is UP/DOWN navigation + CIRCLE
// press, entirely covered by the engine's default input pass (src/input.ts)
// — unlike stats.tsx/library.tsx this entry needs no beforeFrame.

import { createEffect, createSignal } from "solid-js";
import { spring } from "../src/anim.ts";
import type { NodeMirror } from "../src/renderer.ts";

type ThemeName = "indigo" | "emerald" | "amber" | "rose";

function titleCls(t: ThemeName): string {
  if (t === "emerald") return "text-2xl text-emerald-300 font-bold";
  if (t === "amber") return "text-2xl text-amber-300 font-bold";
  if (t === "rose") return "text-2xl text-rose-300 font-bold";
  return "text-2xl text-indigo-300 font-bold";
}

// ---------------------------------------------------------------------------
// Toggle row
// ---------------------------------------------------------------------------

function Toggle(props: { label: string; value: boolean; onToggle: () => void }) {
  let knob: NodeMirror | undefined;
  createEffect(() => {
    if (knob) spring(knob, "translateX", props.value ? 16 : 0);
  });
  return (
    <view
      class="flex-row items-center justify-between p-1 bg-slate-800 border-slate-700 focus:bg-slate-700 focus:border-indigo-400 transition-colors duration-150"
      focusable
      onPress={props.onToggle}
    >
      <text class="text-sm text-slate-200">{props.label}</text>
      <view
        class={
          props.value
            ? "w-9 h-5 rounded-full bg-indigo-500 flex-row items-center"
            : "w-9 h-5 rounded-full bg-slate-700 flex-row items-center"
        }
      >
        <view
          ref={knob}
          class={props.value ? "w-4 h-4 rounded-full bg-white m-[2] translate-x-4" : "w-4 h-4 rounded-full bg-white m-[2] translate-x-0"}
        />
      </view>
    </view>
  );
}

// ---------------------------------------------------------------------------
// Brightness (CIRCLE cycles 1..5, wraps)
// ---------------------------------------------------------------------------

const BRIGHTNESS_TRACK_W = 120;

function Brightness() {
  const [level, setLevel] = createSignal(3);
  let fill: NodeMirror | undefined;
  createEffect(() => {
    if (fill) spring(fill, "width", (level() / 5) * BRIGHTNESS_TRACK_W);
  });
  return (
    <view
      class="flex-row items-center justify-between p-1 bg-slate-800 border-slate-700 focus:bg-slate-700 focus:border-amber-400 transition-colors duration-150"
      focusable
      onPress={() => setLevel(level() >= 5 ? 1 : level() + 1)}
    >
      <text class="text-sm text-slate-200">BRIGHTNESS</text>
      <view class="flex-row items-center gap-2">
        <view class="w-[120] h-2 bg-slate-900">
          <view ref={fill} class="h-2 w-0 bg-gradient-to-r from-amber-400 to-amber-600" />
        </view>
        <text class="text-xs text-slate-500">{level()}/5</text>
      </view>
    </view>
  );
}

// ---------------------------------------------------------------------------
// Theme swatches
// ---------------------------------------------------------------------------

interface ThemeOption {
  name: ThemeName;
  swatchCls: string;
  selectedCls: string;
}

const THEMES: ThemeOption[] = [
  {
    name: "indigo",
    swatchCls: "w-6 h-6 bg-indigo-500 border-slate-800 focus:border-slate-300 transition-colors duration-150",
    selectedCls: "w-6 h-6 bg-indigo-500 border-white transition-colors duration-150",
  },
  {
    name: "emerald",
    swatchCls: "w-6 h-6 bg-emerald-500 border-slate-800 focus:border-slate-300 transition-colors duration-150",
    selectedCls: "w-6 h-6 bg-emerald-500 border-white transition-colors duration-150",
  },
  {
    name: "amber",
    swatchCls: "w-6 h-6 bg-amber-500 border-slate-800 focus:border-slate-300 transition-colors duration-150",
    selectedCls: "w-6 h-6 bg-amber-500 border-white transition-colors duration-150",
  },
  {
    name: "rose",
    swatchCls: "w-6 h-6 bg-rose-500 border-slate-800 focus:border-slate-300 transition-colors duration-150",
    selectedCls: "w-6 h-6 bg-rose-500 border-white transition-colors duration-150",
  },
];

function ThemeRow(props: { value: ThemeName; onPick: (t: ThemeName) => void }) {
  return (
    <view class="flex-col gap-1 p-1 bg-slate-800 border-slate-700">
      <text class="text-sm text-slate-200">THEME</text>
      <view class="flex-row gap-3">
        {THEMES.map((t) => (
          <view
            class={props.value === t.name ? t.selectedCls : t.swatchCls}
            focusable
            onPress={() => props.onPick(t.name)}
          />
        ))}
      </view>
    </view>
  );
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function Settings() {
  const [sfx, setSfx] = createSignal(true);
  const [vibration, setVibration] = createSignal(false);
  const [theme, setTheme] = createSignal<ThemeName>("indigo");

  return (
    <view class="flex-col w-full h-full p-3 gap-2 bg-gradient-to-b from-slate-900 to-slate-950">
      <view class="flex-row items-end justify-between">
        <view class="flex-col">
          <text class="text-xs text-indigo-300 tracking-wide">PSP-UI SHOWCASE</text>
          <text class={titleCls(theme())}>Settings</text>
        </view>
        <text class="text-xs text-slate-500">4 OPTIONS</text>
      </view>

      <view class="flex-col gap-1">
        <Toggle label="SOUND EFFECTS" value={sfx()} onToggle={() => setSfx(!sfx())} />
        <Toggle label="VIBRATION" value={vibration()} onToggle={() => setVibration(!vibration())} />
        <Brightness />
        <ThemeRow value={theme()} onPick={setTheme} />
      </view>

      <text class="text-xs text-slate-500">UP / DOWN move focus · CIRCLE toggle / cycle / select</text>
    </view>
  );
}
