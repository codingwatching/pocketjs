import { describe, expect, test } from "bun:test";
import {
  generatePocketManifestV2Schema,
  type PocketManifestV2,
} from "../spec/pocket-manifest.ts";
import {
  POCKET_CAPABILITIES,
  POCKET_PLATFORM_CONTRACTS,
  POCKET_TARGETS,
  defineCapabilityRegistry,
  definePlatformContractRegistry,
  defineTargetRegistry,
  type CapabilityId,
  type TargetId,
  type TargetProfile,
} from "../spec/platforms.ts";
import { canonicalJson, verifyBuildPlanHash } from "../src/manifest/plan.ts";
import {
  resolveBuildPlan,
  validateAndResolveBuildPlan,
  validatePlatformContractRegistry,
} from "../src/manifest/resolve.ts";
import { validatePocketManifest } from "../src/manifest/validate.ts";

const fixtureUrl = (name: string) => new URL(`./fixtures/manifests/${name}.json`, import.meta.url);
const portableInput: unknown = await Bun.file(fixtureUrl("portable-psp")).json();
const invalidExtraInput: unknown = await Bun.file(fixtureUrl("invalid-extra-field")).json();
const touchInput: unknown = await Bun.file(fixtureUrl("requires-touch")).json();

function manifest(input: unknown): PocketManifestV2 {
  const result = validatePocketManifest(input);
  if (!result.ok) throw new Error(JSON.stringify(result.diagnostics));
  return result.value;
}

const SYNTHETIC_CAPABILITIES = defineCapabilityRegistry({
  ...POCKET_CAPABILITIES,
  "input.touch": {
    version: 1,
    parameters: {
      contacts: { kind: "integer", required: true, relation: "at-least", minimum: 1 },
    },
  },
} as const);

type SyntheticCapabilityId = CapabilityId<typeof SYNTHETIC_CAPABILITIES>;

const syntheticTargetDefinitions = {
  psp: POCKET_TARGETS.psp,
  "vita-test": {
    profileVersion: 1,
    hostAbi: 1,
    packageFormat: "vita-test",
    packageDefaults: {
      title: { from: "app.title" },
      titleId: "DEV000001",
      icon0: "builtin:pocketjs/vita/icon0",
      memoryBudgetMb: 64,
    },
    display: {
      physicalViewport: [960, 544],
      logicalViewports: [[480, 272]],
      presentations: ["integer-fit"],
    },
    capabilities: {
      "input.analog": { version: 1, parameters: { sticks: 2 } },
      "input.buttons": { version: 1 },
      "input.touch": { version: 1, parameters: { contacts: 6 } },
      "text.glyphs.baked": { version: 1 },
      "ui.drawlist": { version: 1 },
    },
  },
} as const satisfies Readonly<Record<string, TargetProfile<SyntheticCapabilityId>>>;

const SYNTHETIC_TARGETS = defineTargetRegistry<
  SyntheticCapabilityId,
  typeof syntheticTargetDefinitions
>(syntheticTargetDefinitions);

type SyntheticTargetId = TargetId<typeof SYNTHETIC_TARGETS>;
const SYNTHETIC_CONTRACTS = definePlatformContractRegistry(
  SYNTHETIC_CAPABILITIES,
  SYNTHETIC_TARGETS,
);

describe("pocket.json v2 schema", () => {
  test("committed JSON Schema is byte-exact with the TypeScript source", async () => {
    const committed = await Bun.file(new URL("../schema/pocket-2.json", import.meta.url)).text();
    expect(committed).toBe(generatePocketManifestV2Schema());
  });

  test("accepts the portable PSP fixture", () => {
    expect(validatePocketManifest(portableInput).ok).toBe(true);
  });

  test("rejects unknown fields at their exact JSON Pointer", () => {
    const result = validatePocketManifest(invalidExtraInput);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.diagnostics).toContainEqual({
      code: "schema.additionalProperty",
      path: "/app/target",
      message: "unknown property",
    });
  });

  test("rejects path traversal and non-primitive capability parameters", () => {
    const bad = structuredClone(portableInput) as Record<string, any>;
    bad.app.entry = "../outside.tsx";
    bad.engine.capabilities.requires[0].parameters = { unsafe: {} };
    const result = validatePocketManifest(bad);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.diagnostics.map((item) => [item.code, item.path])).toEqual(expect.arrayContaining([
      ["schema.pattern", "/app/entry"],
      ["schema.anyOf", "/engine/capabilities/requires/0/parameters/unsafe"],
    ]));
  });
});

describe("platform registry", () => {
  test("production advertises only the truthful PSP profile", () => {
    expect(Object.keys(POCKET_TARGETS)).toEqual(["psp"]);
    expect(validatePlatformContractRegistry(POCKET_PLATFORM_CONTRACTS)).toEqual([]);
  });

  test("TargetId and capability registries extend without changing the resolver", () => {
    const ids: SyntheticTargetId[] = ["psp", "vita-test"];
    expect(ids).toEqual(["psp", "vita-test"]);
    expect(validatePlatformContractRegistry(SYNTHETIC_CONTRACTS)).toEqual([]);
  });
});

describe("semantic resolution", () => {
  test("resolves the PSP contract and carries package metadata into a hashed plan", () => {
    const result = validateAndResolveBuildPlan(portableInput, { target: "psp" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.plan.target).toEqual({ id: "psp", profileVersion: 1, hostAbi: 1 });
    expect(result.plan.package).toEqual({
      format: "psp",
      metadata: {
        title: "Pocket Telemetry",
        icon0: "art/ICON0.png",
        pic1: "art/PIC1.png",
        memoryBudgetMb: 20,
      },
    });
    expect(result.plan.viewport.scale).toEqual({
      x: { numerator: 1, denominator: 1 },
      y: { numerator: 1, denominator: 1 },
    });
    expect(result.plan.capabilities.requires.map((item) => item.requirement.id)).toEqual([
      "input.analog",
      "input.buttons",
      "text.glyphs.baked",
      "ui.drawlist",
    ]);
    expect(result.plan.contractHash).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(verifyBuildPlanHash(result.plan)).toBe(true);
  });

  test("resolved PSP plan is byte-exact with its committed contract fixture", async () => {
    const result = validateAndResolveBuildPlan(portableInput, { target: "psp" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const committed = await Bun.file(
      new URL("./fixtures/plans/portable-psp.plan.json", import.meta.url),
    ).text();
    expect(JSON.stringify(result.plan, null, 2) + "\n").toBe(committed);
  });

  test("canonical hash is independent of manifest capability order", () => {
    const reordered = structuredClone(portableInput) as Record<string, any>;
    reordered.engine.capabilities.requires.reverse();
    const left = validateAndResolveBuildPlan(portableInput, { target: "psp" });
    const right = validateAndResolveBuildPlan(reordered, { target: "psp" });
    expect(left.ok && right.ok).toBe(true);
    if (!left.ok || !right.ok) return;
    expect(canonicalJson(left.plan)).toBe(canonicalJson(right.plan));
    expect(left.plan.contractHash).toBe(right.plan.contractHash);
  });

  test("package entries are overrides, not a target compatibility allowlist", () => {
    const noPackages = structuredClone(portableInput) as Record<string, any>;
    delete noPackages.packages;
    const psp = validateAndResolveBuildPlan(noPackages, { target: "psp" });
    expect(psp.ok).toBe(true);
    if (!psp.ok) return;
    expect(psp.plan.package).toEqual({
      format: "psp",
      metadata: {
        title: "Pocket Telemetry",
        icon0: "builtin:pocketjs/psp/icon0",
        pic1: "builtin:pocketjs/psp/pic1",
        memoryBudgetMb: 20,
      },
    });

    const vita = validateAndResolveBuildPlan(noPackages, { target: "vita-test" }, SYNTHETIC_CONTRACTS);
    expect(vita.ok).toBe(true);
    if (!vita.ok) return;
    expect(vita.plan.package).toEqual({
      format: "vita-test",
      metadata: {
        title: "Pocket Telemetry",
        titleId: "DEV000001",
        icon0: "builtin:pocketjs/vita/icon0",
        memoryBudgetMb: 64,
      },
    });
  });

  test("reports unknown target, ABI mismatch and missing hard requirements", () => {
    const unknownTarget = validateAndResolveBuildPlan(portableInput, { target: "vita" });
    expect(unknownTarget.ok).toBe(false);
    if (!unknownTarget.ok) expect(unknownTarget.diagnostics[0]?.code).toBe("target.unknown");

    const abi = structuredClone(portableInput) as Record<string, any>;
    abi.engine.abi = 2;
    const abiResult = validateAndResolveBuildPlan(abi, { target: "psp" });
    expect(abiResult.ok).toBe(false);
    if (!abiResult.ok) expect(abiResult.diagnostics.some((item) => item.code === "engine.abiMismatch")).toBe(true);

    const twoSticks = structuredClone(portableInput) as Record<string, any>;
    twoSticks.engine.capabilities.requires.find((item: any) => item.id === "input.analog").parameters.sticks = 2;
    const sticksResult = validateAndResolveBuildPlan(twoSticks, { target: "psp" });
    expect(sticksResult.ok).toBe(false);
    if (!sticksResult.ok) expect(sticksResult.diagnostics.some((item) => item.code === "capability.unavailable")).toBe(true);
  });

  test("rejects unknown parameters and duplicate requires/enhances declarations", () => {
    const bad = structuredClone(portableInput) as Record<string, any>;
    bad.engine.capabilities.requires[0].parameters = { surprise: true };
    bad.engine.capabilities.enhances = [{ id: "ui.drawlist", version: 1 }];
    const result = validateAndResolveBuildPlan(bad, { target: "psp" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.diagnostics.map((item) => item.code)).toEqual(expect.arrayContaining([
      "capability.unknownParameter",
      "capability.duplicate",
    ]));
  });

  test("synthetic higher-resolution target proves PSP compatibility without production Vita claims", () => {
    const result = resolveBuildPlan(manifest(portableInput), { target: "vita-test" }, SYNTHETIC_CONTRACTS);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.plan.viewport).toMatchObject({
      logical: [480, 272],
      physical: [960, 544],
      presentation: "integer-fit",
      scale: {
        x: { numerator: 2, denominator: 1 },
        y: { numerator: 2, denominator: 1 },
      },
    });
  });

  test("enhances does not gate PSP and resolves available on the synthetic touch host", () => {
    const adaptive = structuredClone(touchInput) as Record<string, any>;
    adaptive.engine.capabilities.requires = [
      { id: "ui.drawlist", version: 1 },
      { id: "input.buttons", version: 1 },
    ];
    adaptive.engine.capabilities.enhances = [
      { id: "input.touch", version: 1, parameters: { contacts: 2 } },
    ];
    const parsed = manifest(adaptive);

    const psp = resolveBuildPlan(parsed, { target: "psp" }, SYNTHETIC_CONTRACTS);
    const vita = resolveBuildPlan(parsed, { target: "vita-test" }, SYNTHETIC_CONTRACTS);
    expect(psp.ok && vita.ok).toBe(true);
    if (!psp.ok || !vita.ok) return;
    expect(psp.plan.capabilities.enhances[0]?.status).toBe("unavailable");
    expect(vita.plan.capabilities.enhances[0]?.status).toBe("available");
  });

  test("production registry rejects future capabilities instead of pretending Vita exists", () => {
    const result = validateAndResolveBuildPlan(touchInput, { target: "psp" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.diagnostics.some((item) => item.code === "capability.unknown")).toBe(true);
  });
});
