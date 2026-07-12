import { describe, expect, test } from "bun:test";

async function bundlePlatform(defines: Record<string, string> = {}) {
  const result = await Bun.build({
    entrypoints: [new URL("../src/platform.ts", import.meta.url).pathname],
    format: "esm",
    target: "bun",
    define: defines,
  });
  expect(result.success).toBe(true);
  const source = await result.outputs[0]!.text();
  return import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`) as Promise<{
    platform: { target: string; features: Record<string, boolean> };
    hasFeature: (feature: "input.buttons") => boolean;
  }>;
}

describe("platform feature availability", () => {
  test("embeds the resolved feature map in manifest builds", async () => {
    const runtime = await bundlePlatform({
      __POCKET_TARGET__: JSON.stringify("psp"),
      __POCKET_FEATURES__: JSON.stringify({ "input.buttons": true }),
    });
    expect(runtime.platform).toEqual({
      target: "psp",
      features: { "input.buttons": true },
    });
    expect(runtime.hasFeature("input.buttons")).toBe(true);
  });

  test("legacy builds have an unknown target and no feature claims", async () => {
    const runtime = await bundlePlatform();
    expect(runtime.platform).toEqual({ target: "unknown", features: {} });
    expect(runtime.hasFeature("input.buttons")).toBe(false);
  });
});
