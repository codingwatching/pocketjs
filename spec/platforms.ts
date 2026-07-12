// PocketJS platform/capability contract registry.
//
// This file describes facts supplied by framework-owned hosts. Applications do
// not get to assert these facts in pocket.json; they only state requirements.
// Keep production profiles honest: a target belongs in POCKET_TARGETS only
// after its host implements and tests every advertised capability.

export type CapabilityParameterKind = "boolean" | "integer" | "number" | "string";
export type CapabilityParameterRelation = "equal" | "at-least";

export interface CapabilityParameterDefinition {
  readonly kind: CapabilityParameterKind;
  readonly required: boolean;
  readonly relation: CapabilityParameterRelation;
  readonly minimum?: number;
}

export interface CapabilityDefinition {
  /** Integer ABI generation. Different generations are not assumed compatible. */
  readonly version: number;
  readonly parameters: Readonly<Record<string, CapabilityParameterDefinition>>;
}

export type CapabilityRegistry = Readonly<Record<string, CapabilityDefinition>>;

export function defineCapabilityRegistry<const T extends CapabilityRegistry>(registry: T): T {
  return registry;
}

export type CapabilityId<T extends CapabilityRegistry> = Extract<keyof T, string>;

export interface ProvidedCapability {
  readonly version: number;
  readonly parameters?: Readonly<Record<string, boolean | number | string>>;
}

export const PRESENTATION_MODES = ["fill", "fit", "integer-fit", "native", "stretch"] as const;
export type PresentationMode = (typeof PRESENTATION_MODES)[number];
export type Viewport = readonly [width: number, height: number];

export interface DisplayProfile {
  readonly physicalViewport: Viewport;
  readonly logicalViewports: readonly Viewport[];
  readonly presentations: readonly PresentationMode[];
}

export type PackageDefaultValue = boolean | number | string | { readonly from: "app.title" };

export interface TargetProfile<C extends string = string> {
  /** Version of the target profile itself, independent from the host ABI. */
  readonly profileVersion: number;
  /** JS/native host contract generation consumed by pocket.json engine.abi. */
  readonly hostAbi: number;
  /** Selects the matching manifest packages entry without target-specific branching. */
  readonly packageFormat: string;
  /** Complete deterministic metadata used by dev builds before app overrides. */
  readonly packageDefaults: Readonly<Record<string, PackageDefaultValue>>;
  readonly display: DisplayProfile;
  readonly capabilities: Readonly<Partial<Record<C, ProvidedCapability>>>;
}

export type TargetRegistry<C extends string = string> = Readonly<Record<string, TargetProfile<C>>>;

export function defineTargetRegistry<
  C extends string,
  const T extends TargetRegistry<C>,
>(registry: T): T {
  return registry;
}

export type TargetId<T extends TargetRegistry> = Extract<keyof T, string>;

export const POCKET_CAPABILITIES = defineCapabilityRegistry({
  "input.analog": {
    version: 1,
    parameters: {
      sticks: { kind: "integer", required: true, relation: "at-least", minimum: 1 },
    },
  },
  "input.buttons": {
    version: 1,
    parameters: {},
  },
  "text.glyphs.baked": {
    version: 1,
    parameters: {},
  },
  "ui.drawlist": {
    version: 1,
    parameters: {},
  },
} as const);

export type PocketCapabilityId = CapabilityId<typeof POCKET_CAPABILITIES>;

/**
 * The only production target profile registered in the contract layer today.
 *
 * Do not register Vita here merely because native-vita exists on another
 * branch. Its stock host must first satisfy the portable PSP contract (notably
 * left-stick delivery) and pass the same contract tests.
 */
export const POCKET_TARGETS = defineTargetRegistry<PocketCapabilityId, {
  readonly psp: TargetProfile<PocketCapabilityId>;
}>({
  psp: {
    profileVersion: 1,
    hostAbi: 1,
    packageFormat: "psp",
    packageDefaults: {
      title: { from: "app.title" },
      icon0: "builtin:pocketjs/psp/icon0",
      pic1: "builtin:pocketjs/psp/pic1",
      memoryBudgetMb: 20,
    },
    display: {
      physicalViewport: [480, 272],
      logicalViewports: [[480, 272]],
      // integer-fit at scale 1 is the portable spelling of the native PSP
      // surface and can be satisfied unchanged by higher-resolution hosts.
      presentations: ["native", "integer-fit"],
    },
    capabilities: {
      "input.analog": { version: 1, parameters: { sticks: 1 } },
      "input.buttons": { version: 1 },
      "text.glyphs.baked": { version: 1 },
      "ui.drawlist": { version: 1 },
    },
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
