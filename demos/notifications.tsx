// demos/notifications.tsx — "notification center" showcase: a real <For>
// list (the other three demos only ever .map() a fixed-length array — this
// is the one demo whose array actually shrinks, so it's the one that
// exercises <For>'s per-item mount/unmount identity instead of just reorder).
// Each item staggers in with a delayed opacity+translateX tween on mount;
// CIRCLE dismisses the focused card — an imperative fade+slide (native
// `opacity`/`translateX` tweens fired straight from onPress, not a reactive
// class, so they can't be clobbered by an unrelated re-render) — a frame-
// driven timer (beforeFrame, same shape as stats.tsx/library.tsx) removes it
// from the list once the tween has had time to finish.
//
// v1-aware design notes: p-1 rows / p-3 root — 4 cards is already a tight fit
// in 480x272 (DESIGN.md punts kinetic scroll, so the list can't overflow the
// screen); every class a FULL literal.

import { createSignal, For, onMount, Show } from "solid-js";
import { animate } from "../src/anim.ts";
import type { NodeMirror } from "../src/renderer.ts";

interface Notice {
  id: string;
  title: string;
  message: string;
  time: string;
  /** dot: FULL literal (fixed size + accent color, rounded-full is safe —
   *  build-time known w/h). */
  dotCls: string;
}

const INITIAL: Notice[] = [
  {
    id: "update",
    title: "UPDATE AVAILABLE",
    message: "Firmware 6.61 is ready to install.",
    time: "2m ago",
    dotCls: "w-2 h-2 rounded-full bg-sky-400",
  },
  {
    id: "friend",
    title: "FRIEND REQUEST",
    message: "RIDGE_FOX wants to join your session.",
    time: "14m ago",
    dotCls: "w-2 h-2 rounded-full bg-emerald-400",
  },
  {
    id: "battery",
    title: "LOW BATTERY",
    message: "12% remaining — plug in soon.",
    time: "35m ago",
    dotCls: "w-2 h-2 rounded-full bg-amber-400",
  },
  {
    id: "trophy",
    title: "TROPHY UNLOCKED",
    message: '"First Contact" — Iron Vanguard.',
    time: "1h ago",
    dotCls: "w-2 h-2 rounded-full bg-indigo-400",
  },
];

const DISMISS_FRAMES = 16; // >= the 200ms fade tween (~12 frames), plus margin

// ---------------------------------------------------------------------------
// Frame driver (wired by notifications-main.tsx): once a dismiss is in
// flight, counts down and splices the item out when its tween has settled.
// ---------------------------------------------------------------------------

const [items, setItems] = createSignal<Notice[]>(INITIAL);
const [dismissingId, setDismissingId] = createSignal<string | null>(null);
const [dismissFrame, setDismissFrame] = createSignal(0);

export function notificationsFrame(): void {
  const id = dismissingId();
  if (id === null) return;
  const n = dismissFrame() + 1;
  setDismissFrame(n);
  if (n >= DISMISS_FRAMES) {
    setItems(items().filter((it) => it.id !== id));
    setDismissingId(null);
    setDismissFrame(0);
  }
}

function dismiss(id: string, el: NodeMirror | undefined): void {
  if (dismissingId() !== null || !el) return;
  setDismissingId(id);
  setDismissFrame(0);
  animate(el, "opacity", 0, { dur: 200, easing: "out" });
  animate(el, "translateX", 24, { dur: 200, easing: "out" });
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function Notifications() {
  return (
    <view class="flex-col w-full h-full p-3 gap-2 bg-gradient-to-b from-slate-900 to-slate-950">
      <view class="flex-row items-end justify-between">
        <view class="flex-col">
          <text class="text-xs text-indigo-300 tracking-wide">PSP-UI SHOWCASE</text>
          <text class="text-2xl text-white font-bold">Notifications</text>
        </view>
        <text class="text-xs text-slate-500">{items().length} UNREAD</text>
      </view>

      <view class="flex-col gap-1">
        <For each={items()}>
          {(item, i) => {
            let el: NodeMirror | undefined;
            onMount(() => {
              if (el) {
                animate(el, "opacity", 1, { dur: 250, delay: i() * 70, easing: "out" });
                animate(el, "translateX", 0, { dur: 250, delay: i() * 70, easing: "out" });
              }
            });
            return (
              <view
                ref={el}
                style={{ opacity: 0, translateX: 16 }}
                class="flex-row items-center gap-3 p-1 bg-slate-800 border-slate-700 focus:bg-slate-700 focus:border-indigo-400 transition-colors duration-150"
                focusable
                onPress={() => dismiss(item.id, el)}
              >
                <view class={item.dotCls} />
                <view class="flex-col grow">
                  <text class="text-xs text-white font-bold">{item.title}</text>
                  <text class="text-xs text-slate-400">{item.message}</text>
                </view>
                <text class="text-xs text-slate-500">{item.time}</text>
              </view>
            );
          }}
        </For>
      </view>

      <Show when={items().length === 0}>
        <view class="grow flex-col items-center justify-center">
          <text class="text-sm text-slate-500">ALL CLEAR</text>
        </view>
      </Show>

      <text class="text-xs text-slate-500">UP / DOWN move focus · CIRCLE dismiss</text>
    </view>
  );
}
