// PocketJS platform capability registry.
//
// Capabilities name framework APIs that a stock host has implemented and
// tested. They are deliberately plain identifiers: hardware specifications,
// permissions, and runtime availability belong to their respective layers.

export type CapabilityRegistry = readonly string[];

export function defineCapabilityRegistry<const T extends CapabilityRegistry>(registry: T): T {
  return registry;
}

export type CapabilityId<T extends CapabilityRegistry> = T[number];

export const PRESENTATION_MODES = ["fill", "fit", "integer-fit", "native", "stretch"] as const;
export type PresentationMode = (typeof PRESENTATION_MODES)[number];
export type Viewport = readonly [width: number, height: number];

export interface DisplayProfile {
  readonly physicalViewport: Viewport;
  readonly logicalViewports: readonly Viewport[];
  readonly presentations: readonly PresentationMode[];
}

export interface TargetProfile<C extends string = string> {
  /** JS/native HostOps wire generation embedded by the selected backend. */
  readonly hostAbi: number;
  readonly display: DisplayProfile;
  /** Framework APIs implemented and tested by this stock host. */
  readonly capabilities: readonly C[];
}

export type TargetRegistry<C extends string = string> = Readonly<Record<string, TargetProfile<C>>>;

export function defineTargetRegistry<
  C extends string,
  const T extends TargetRegistry<C>,
>(registry: T): T {
  return registry;
}

export type TargetId<T extends TargetRegistry> = Extract<keyof T, string>;

export const POCKET_CAPABILITIES = defineCapabilityRegistry([
  "input.analog.left",
  "input.buttons",
  "text.glyphs.baked",
  "ui.drawlist",
] as const);

export type PocketCapabilityId = CapabilityId<typeof POCKET_CAPABILITIES>;

/**
 * The only production target profile registered in the contract layer today.
 *
 * Do not register Vita here merely because native-vita exists on another
 * branch. Its stock host must first satisfy this portable PSP API baseline and
 * pass the same contract tests.
 */
export const POCKET_TARGETS = defineTargetRegistry<PocketCapabilityId, {
  readonly psp: TargetProfile<PocketCapabilityId>;
}>({
  psp: {
    hostAbi: 1,
    display: {
      physicalViewport: [480, 272],
      logicalViewports: [[480, 272]],
      // integer-fit at scale 1 is the portable spelling of the native PSP
      // surface and can be satisfied unchanged by higher-resolution hosts.
      presentations: ["native", "integer-fit"],
    },
    capabilities: [
      "input.analog.left",
      "input.buttons",
      "text.glyphs.baked",
      "ui.drawlist",
    ],
  },
});

export type PocketTargetId = TargetId<typeof POCKET_TARGETS>;

export interface PlatformContractRegistry<
  C extends CapabilityRegistry = CapabilityRegistry,
  T extends TargetRegistry<CapabilityId<C>> = TargetRegistry<CapabilityId<C>>,
> {
  readonly capabilities: C;
  readonly targets: T;
}

export function definePlatformContractRegistry<
  const C extends CapabilityRegistry,
  const T extends TargetRegistry<CapabilityId<C>>,
>(capabilities: C, targets: T): PlatformContractRegistry<C, T> {
  return { capabilities, targets };
}

export const POCKET_PLATFORM_CONTRACTS = definePlatformContractRegistry(
  POCKET_CAPABILITIES,
  POCKET_TARGETS,
);
