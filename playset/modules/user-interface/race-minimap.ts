// playset/modules/user-interface/race-minimap.ts — race overview minimap:
// checkpoints, AI competitors (+ leader ring) and the local vehicle projected
// through MinimapProjector2D onto a fixed rectangle of dot Views.
//
// Ported from GameBlocks (github.com/xt4d/GameBlocks, MIT © 2026 Weihao
// Cheng) — modules/user-interface/RaceMinimap.js. The projection (inherited
// MinimapProjector2D), dot radii (checkpoint 2.1 / next 3.4 / ai 2.8 / leader
// ring 4.8), style keys and next-checkpoint modulo rule are verbatim; the
// Canvas2D class becomes a PocketJS component over an internal projector.
// Deviations forced by the move: the track POLYLINE is skipped in v1 (no line
// primitive — checkpoint dots still trace the circuit), the local-vehicle
// triangle collapses to a stroked dot that still carries projectYaw as a
// rotate (degrees), canvas pixelRatio/syncResolution is moot for native
// views, and rgba() style strings are converted to #rrggbbaa. Rows render via
// <Index> (not For): they are positional projections recomputed per update —
// For would tear every dot down each frame. `basis` is exposed as a prop
// (the original hardcoded the default basis in its super() call).

import type { JSX as SolidJSX } from "solid-js";
import { Index, Show, createMemo } from "solid-js";
import { View, type ViewProps } from "@pocketjs/framework/components";
import { toDeg } from "../math/scalar-utils.ts";
import { DEFAULT_WORLD_BASIS, type VecLike, type WorldBasis } from "../math/world-basis.ts";
import { MinimapProjector2D, type PlanarBounds } from "./minimap-projector-2d.ts";

type StyleObject = NonNullable<ViewProps["style"]>;

// spec/spec.ts ENUMS ordinal (stable wire value; playset avoids a spec dep).
const POS_ABSOLUTE = 1;

export const DEFAULT_STYLES = Object.freeze({
  background: "#060a10e6", // rgba(6,10,16,0.9)
  border: "#8099bf9e", // rgba(128,153,191,0.62)
  track: "#71b9ffb8", // rgba(113,185,255,0.72) — unused until the v1 polyline gap closes
  checkpoint: "#ccdfffb3", // rgba(204,223,255,0.7)
  nextCheckpoint: "#ffe88a",
  localFill: "#f16a45",
  localStroke: "#fff0db",
  leaderRing: "#ffe88a",
});

export type RaceMinimapStyles = { -readonly [K in keyof typeof DEFAULT_STYLES]: string };

function toCssColor(value: string | number | null | undefined, fallback = "#8ab4d8"): string {
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  if (Number.isFinite(value)) {
    return `#${(value as number).toString(16).padStart(6, "0")}`;
  }
  return fallback;
}

export interface LocalVehicle {
  position?: VecLike | null;
  bodyFrame?: { forward?: VecLike | null } | null;
}

export interface RaceProgress {
  nextCheckpointIndex: number;
}

export interface AiCar extends Record<string, unknown> {
  id?: unknown;
  position?: VecLike | null;
  motion?: { position?: VecLike | null } | null;
  color?: string | number;
}

export interface RaceMinimapProps {
  planarBounds: PlanarBounds;
  width?: number;
  height?: number;
  padding?: number;
  invertRight?: boolean;
  invertForward?: boolean;
  basis?: WorldBasis;
  styles?: Partial<RaceMinimapStyles>;
  checkpoints?: () => (VecLike | null | undefined)[];
  localVehicle?: () => LocalVehicle | null | undefined;
  localProgress?: () => RaceProgress | null | undefined;
  aiCars?: () => AiCar[];
  aiLeaderId?: () => unknown;
}

interface ProjectedCheckpoint {
  x: number;
  y: number;
  radius: number;
  color: string;
}

interface ProjectedCar {
  x: number;
  y: number;
  color: string;
  leader: boolean;
}

const LOCAL_MARKER_RADIUS = 4.5; // the original triangle's half-width

export function RaceMinimap(props: RaceMinimapProps): SolidJSX.Element {
  const styles: RaceMinimapStyles = { ...DEFAULT_STYLES, ...(props.styles ?? {}) };
  const basis = props.basis ?? DEFAULT_WORLD_BASIS;
  const projector = new MinimapProjector2D({
    planarBounds: { ...props.planarBounds },
    width: props.width ?? 200,
    height: props.height ?? 200,
    padding: props.padding ?? 0,
    invertRight: props.invertRight ?? false,
    invertForward: props.invertForward ?? false,
    basis,
  });

  const checkpointDots = createMemo<ProjectedCheckpoint[]>(() => {
    const checkpoints = props.checkpoints?.() ?? [];
    const progress = props.localProgress?.() ?? null;
    const nextCheckpointIndex = progress
      ? progress.nextCheckpointIndex % checkpoints.length
      : -1;
    const out: ProjectedCheckpoint[] = [];
    for (let i = 0; i < checkpoints.length; i += 1) {
      const point = projector.project(checkpoints[i], { x: 0, y: 0 });
      const isNext = i === nextCheckpointIndex;
      out.push({
        x: point.x,
        y: point.y,
        radius: isNext ? 3.4 : 2.1,
        color: isNext ? styles.nextCheckpoint : styles.checkpoint,
      });
    }
    return out;
  });

  const carDots = createMemo<ProjectedCar[]>(() => {
    const leaderId = props.aiLeaderId?.();
    const out: ProjectedCar[] = [];
    for (const aiCar of props.aiCars?.() ?? []) {
      const position = aiCar?.position ?? aiCar?.motion?.position ?? null;
      if (!position) continue;

      const point = projector.project(position, { x: 0, y: 0 });
      out.push({
        x: point.x,
        y: point.y,
        color: toCssColor(aiCar?.color),
        leader: aiCar?.id === leaderId && leaderId != null,
      });
    }
    return out;
  });

  const local = createMemo<{ x: number; y: number; rotate: number } | null>(() => {
    const vehicle = props.localVehicle?.() ?? null;
    const localPosition = vehicle?.position;
    if (!localPosition) return null;

    const point = projector.project(localPosition, { x: 0, y: 0 });
    const yaw = projector.projectYaw(vehicle?.bodyFrame?.forward ?? basis.forwardVector());
    return { x: point.x, y: point.y, rotate: toDeg(yaw) };
  });

  const dotStyle = (x: number, y: number, radius: number, rest: StyleObject): StyleObject => ({
    posType: POS_ABSOLUTE,
    insetL: 0,
    insetT: 0,
    width: radius * 2,
    height: radius * 2,
    radius,
    translateX: x - radius,
    translateY: y - radius,
    ...rest,
  });

  return View({
    debugName: "RaceMinimap",
    style: {
      width: projector.width,
      height: projector.height,
      bgColor: styles.background,
      borderColor: styles.border,
      borderWidth: 1,
    },
    children: [
      // checkpoints (the v1 track line is these dots' circuit)
      Index({
        get each() {
          return checkpointDots();
        },
        children: (item: () => ProjectedCheckpoint) =>
          View({
            get style(): StyleObject {
              const d = item();
              return dotStyle(d.x, d.y, d.radius, { bgColor: d.color });
            },
          }),
      }) as unknown as SolidJSX.Element,
      // AI competitors + leader ring
      Index({
        get each() {
          return carDots();
        },
        children: (item: () => ProjectedCar) =>
          [
            View({
              get style(): StyleObject {
                const d = item();
                return dotStyle(d.x, d.y, 2.8, { bgColor: d.color });
              },
            }),
            Show({
              get when() {
                return item().leader;
              },
              get children() {
                return View({
                  get style(): StyleObject {
                    const d = item();
                    return dotStyle(d.x, d.y, 4.8, {
                      borderColor: styles.leaderRing,
                      borderWidth: 1,
                    });
                  },
                });
              },
            }),
          ] as unknown as SolidJSX.Element,
      }) as unknown as SolidJSX.Element,
      // local vehicle — triangle collapsed to a stroked dot, yaw kept as rotate
      Show({
        get when() {
          return local() !== null;
        },
        get children() {
          return View({
            get style(): StyleObject {
              const d = local();
              if (!d) return {};
              return dotStyle(d.x, d.y, LOCAL_MARKER_RADIUS, {
                bgColor: styles.localFill,
                borderColor: styles.localStroke,
                borderWidth: 1,
                rotate: d.rotate,
              });
            },
          });
        },
      }) as unknown as SolidJSX.Element,
    ],
  });
}
