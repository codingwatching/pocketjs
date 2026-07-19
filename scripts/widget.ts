// bun run widget [app] [flags…] — build + launch a Pocket Stage desktop widget
// (the first bundled stage is the PSP asset; WIDGET.md).
//
//   bun run widget                # the hero demo inside the widget
//   bun run widget im             # any demo (name resolves to <name>-main)
//   bun run widget -- --focus     # extra flags pass through to the binary
//   bun run widget im --auto-quit 5
//   bun run widget --proof        # headless acceptance: a scripted D-pad
//                                 # tap + a real ray-picked CIRCLE click
//                                 # drive hero to "Count: 1"
//
// The windowed run stays attached to your terminal — quit with Esc (or
// Ctrl-C). On exit the shell prints its governor receipt:
// "pocket-widget: N ticks, M frames rendered" — a settled app should show
// M ≪ N.
import { $ } from "bun";
import { mkdirSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { demoManifestFor } from "./demo-identity.ts";
import {
  POCKET_CAPABILITIES,
  definePlatformContractRegistry,
  defineTargetRegistry,
} from "../spec/platforms.ts";
import { validateAndResolveBuildPlan } from "../src/manifest/resolve.ts";
import type { ResolvedBuildPlan } from "../src/manifest/plan.ts";

const root = new URL("..", import.meta.url).pathname;

/**
 * Transitional profile for the bundled PSP stage. The outer process is a
 * desktop widget, but the guest occupies a fixed screen mesh and therefore
 * resolves as an embedded target. Once pocket-stage.json lands, these display
 * facts come from its selected surface instead of this constant.
 */
export const STAGE_TARGET_ID = "macos-embedded";
// Same current desktop HostOps wire generation as macos-widget; form and
// capabilities differ even though the native UI surface implementation is shared.
export const STAGE_HOST_ABI = 3;
export const STAGE_PLATFORM_CONTRACTS = definePlatformContractRegistry(
  POCKET_CAPABILITIES,
  defineTargetRegistry({
    [STAGE_TARGET_ID]: {
      hostAbi: STAGE_HOST_ABI,
      platform: "macos",
      form: "embedded",
      display: {
        physicalViewport: [480, 272],
        logicalViewports: [[480, 272]],
        presentations: ["native", "integer-fit"],
        rasterDensity: 1,
      },
      capabilities: [
        "input.analog.left",
        "input.buttons",
        "input.cursor",
        "text.glyphs.baked",
      ],
    },
  }),
);

export function resolveStageBuildPlan(input: unknown): ResolvedBuildPlan {
  const resolution = validateAndResolveBuildPlan(
    input,
    { target: STAGE_TARGET_ID },
    STAGE_PLATFORM_CONTRACTS,
  );
  if (!resolution.ok) {
    throw new Error(
      `pocket-stage: manifest did not resolve: ${resolution.diagnostics
        .map((diagnostic) => `${diagnostic.path || "/"}: ${diagnostic.message}`)
        .join("; ")}`,
    );
  }
  return resolution.plan;
}

export interface WidgetArgs {
  app: string;
  proof: boolean;
  pass: string[];
}

export function validateWidgetArgs(args: WidgetArgs): void {
  if (args.proof && args.app !== "hero-main") {
    throw new Error("--proof uses the bundled hero-main acceptance app");
  }
  if (args.proof && args.pass.length > 0) {
    throw new Error(
      "--proof is a fixed bundled-stage acceptance and cannot be combined with stage flags",
    );
  }
}

/**
 * Parse wrapper arguments without guessing which tokens are flag values.
 * Only argv[0], when positional, names the app. Everything after it keeps
 * its original order for the Rust binary, except the wrapper-only --proof.
 */
export function parseWidgetArgs(rawArgs: readonly string[]): WidgetArgs {
  // `bun run widget -- ...` may leave the option separator in argv. It is a
  // wrapper delimiter, not an argument understood by the pocket-stage binary.
  const args = rawArgs.filter((arg) => arg !== "--");
  const first = args[0];
  const hasApp = first !== undefined && !first.startsWith("--");
  const name = hasApp ? first : "hero";
  const rest = hasApp ? args.slice(1) : args;

  // Demo names resolve to their mounted -main entry (demos/<name>/main.tsx);
  // the bare name would build the side-effect-free component module.
  const app = name.includes("/") || name.endsWith("-main") ? name : `${name}-main`;
  return {
    app,
    proof: rest.includes("--proof"),
    pass: rest.filter((arg) => arg !== "--proof"),
  };
}

async function main(): Promise<void> {
  const parsed = parseWidgetArgs(process.argv.slice(2));
  validateWidgetArgs(parsed);
  const { app, proof, pass } = parsed;

  // Stock demos own their committed pocket.json. Legacy demos without one
  // inherit the root template through demoManifestFor; either way the build
  // is admitted once against the embedded Stage profile and every later
  // compiler input comes from the serialized plan.
  const demo = app.replace(/-main$/, "");
  const manifest = demoManifestFor(root, demo);
  const plan = resolveStageBuildPlan(manifest);
  const planPath = `${root}.pocket/${STAGE_TARGET_ID}/plan.json`;
  mkdirSync(resolvePath(planPath, ".."), { recursive: true });
  await Bun.write(planPath, JSON.stringify(plan, null, 2) + "\n");

  await $`bun scripts/build.ts --plan=${planPath} --project-root=${root}`.cwd(root);
  await $`cargo build --release -p pocket-stage`.cwd(`${root}pocket3d`);

  const bin = `${root}pocket3d/target/release/pocket-stage`;
  const env = { ...process.env, RUST_LOG: process.env.RUST_LOG ?? "info" };

  if (proof) {
    const shot = `${root}dist/pocket-stage-proof.png`;
    await $`${bin} --app ${plan.app.output} --screenshot ${shot} --frames 90 --tap down@10 --click 869,255 --expect-hit btn_circle --expect-ui-hash 0xc34a21cff1f13b06`.env(env);
    console.log(
      "\nproof: the binary asserted both the 3D CIRCLE ray hit and" +
        '\nthe final PocketJS DrawList — the screen reads "Count: 1".' +
        `\n${shot}`,
    );
    await $`open ${shot}`.nothrow();
  } else {
    await $`${bin} --app ${plan.app.output} ${pass}`.env(env);
  }
}

if (import.meta.main) {
  await main();
}
