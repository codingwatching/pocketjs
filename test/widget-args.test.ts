import { describe, expect, test } from "bun:test";
import {
  STAGE_HOST_ABI,
  STAGE_TARGET_ID,
  parseWidgetArgs,
  resolveStageBuildPlan,
  validateWidgetArgs,
} from "../scripts/widget.ts";

describe("widget wrapper arguments", () => {
  test("defaults to the hero app", () => {
    expect(parseWidgetArgs([])).toEqual({
      app: "hero-main",
      proof: false,
      pass: [],
    });
  });

  test("uses only a leading positional token as the app", () => {
    expect(
      parseWidgetArgs([
        "im",
        "--auto-quit",
        "5",
        "--profile",
        "/tmp/psp profile.json",
        "--orbit",
        "35,-12",
      ]),
    ).toEqual({
      app: "im-main",
      proof: false,
      pass: [
        "--auto-quit",
        "5",
        "--profile",
        "/tmp/psp profile.json",
        "--orbit",
        "35,-12",
      ],
    });
  });

  test("does not mistake flag values for an app", () => {
    expect(parseWidgetArgs(["--auto-quit", "5", "--profile", "im", "--orbit", "0,15"])).toEqual({
      app: "hero-main",
      proof: false,
      pass: ["--auto-quit", "5", "--profile", "im", "--orbit", "0,15"],
    });
  });

  test("consumes only the wrapper proof flag", () => {
    expect(parseWidgetArgs(["hero", "--proof", "--auto-quit", "5"])).toEqual({
      app: "hero-main",
      proof: true,
      pass: ["--auto-quit", "5"],
    });
  });

  test("keeps proof deterministic by rejecting forwarded stage flags", () => {
    expect(() =>
      validateWidgetArgs(parseWidgetArgs(["--proof", "--orbit", "10,20"])),
    ).toThrow("fixed bundled-stage acceptance");
  });

  test("keeps proof on the app whose screen state it asserts", () => {
    expect(() => validateWidgetArgs(parseWidgetArgs(["im", "--proof"]))).toThrow(
      "bundled hero-main",
    );
  });

  test("ignores bun option separators while preserving argument order", () => {
    expect(parseWidgetArgs(["--", "--profile", "/tmp/model.json", "--orbit", "10,20"])).toEqual({
      app: "hero-main",
      proof: false,
      pass: ["--profile", "/tmp/model.json", "--orbit", "10,20"],
    });
  });
});

describe("Pocket Stage manifest admission", () => {
  test("resolves a fixed PSP-shaped app as an embedded target", async () => {
    const manifest = await Bun.file(new URL("../pocket.json", import.meta.url)).json();
    const plan = resolveStageBuildPlan(manifest);
    expect(plan.target).toEqual({ id: STAGE_TARGET_ID, hostAbi: STAGE_HOST_ABI });
    expect(plan.viewport).toEqual({
      logical: [480, 272],
      physical: [480, 272],
      presentation: "integer-fit",
      rasterDensity: 1,
    });
  });

  test("rejects a dynamic-only app before the native host starts", async () => {
    const manifest = await Bun.file(
      new URL("../demos/note/pocket.json", import.meta.url),
    ).json();
    expect(() => resolveStageBuildPlan(manifest)).toThrow("fixed screen");
  });
});
