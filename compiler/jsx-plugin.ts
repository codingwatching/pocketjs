// Engine-aware pass-1 Babel transform + AST collection.

import { transformAsync, type PluginObj } from "@babel/core";
import reactPreset from "@babel/preset-react";
import solidPreset from "babel-preset-solid";
import tsPreset from "@babel/preset-typescript"; // untyped - see compiler/ambient.d.ts
import vueJsxPlugin from "@vue/babel-plugin-jsx";
import type { BunPlugin } from "bun";
import reactPresetPkg from "@babel/preset-react/package.json";
import solidPresetPkg from "babel-preset-solid/package.json";
import vueJsxPkg from "@vue/babel-plugin-jsx/package.json";
import babelCorePkg from "@babel/core/package.json";
import tsPresetPkg from "@babel/preset-typescript/package.json";

export type JsxEngine = "react" | "vue" | "solid";

export const RENDERER_PATH = new URL("../src/renderer.ts", import.meta.url).pathname;
export const RENDERER_VUE_PATH = new URL("../src/renderer-vue.ts", import.meta.url).pathname;
export const RENDERER_SOLID_PATH = new URL("../src/renderer-solid.ts", import.meta.url).pathname;
export const COMPONENTS_PATH = new URL("../src/components.tsx", import.meta.url).pathname;
export const COMPONENTS_VUE_PATH = new URL("../src/components-vue.tsx", import.meta.url).pathname;
export const COMPONENTS_SOLID_PATH = new URL("../src/components-solid.tsx", import.meta.url).pathname;
export const FRAME_PATH = new URL("../src/frame.ts", import.meta.url).pathname;
export const FRAME_SOLID_PATH = new URL("../src/frame-solid.ts", import.meta.url).pathname;
export const HOOKS_PATH = new URL("../src/hooks.ts", import.meta.url).pathname;
export const HOOKS_SOLID_PATH = new URL("../src/hooks-solid.ts", import.meta.url).pathname;
export const REACTIVITY_PATH = new URL("../src/reactivity.ts", import.meta.url).pathname;
export const REACTIVITY_SOLID_PATH = new URL("../src/reactivity-solid.ts", import.meta.url).pathname;
const REACT_COMPAT_PATH = new URL("../src/react-compat.ts", import.meta.url).pathname;
const REACT_JSX_RUNTIME_PATH = new URL("../src/react-jsx-runtime.ts", import.meta.url).pathname;

const CACHE_DIR = new URL("../.cache/transforms/", import.meta.url).pathname;
const CACHE_VERSION = "8";

export interface TransformResult {
  /** ESM JS: JSX compiled for the selected engine. */
  code: string;
  /** Candidate class strings (deduped, in AST order). */
  classStrings: string[];
  /** Every codepoint appearing in any collected literal. */
  textCodepoints: Set<number>;
}

interface Collected {
  classStrings: string[];
  textCodepoints: Set<number>;
}

function makeCollector(out: Collected): PluginObj {
  const seen = new Set<string>();
  const add = (s: string) => {
    if (!s) return;
    for (const ch of s) out.textCodepoints.add(ch.codePointAt(0)!);
    if (!seen.has(s)) {
      seen.add(s);
      out.classStrings.push(s);
    }
  };
  return {
    name: "psp-ui-collect",
    visitor: {
      Program: {
        enter(program) {
          program.traverse({
            StringLiteral(path) {
              add(path.node.value);
            },
            TemplateLiteral(path) {
              for (const q of path.node.quasis) add(q.value.cooked ?? q.value.raw);
            },
            JSXText(path) {
              const raw = path.node.extra?.raw;
              if (typeof raw === "string" && raw !== path.node.value) {
                throw path.buildCodeFrameError(
                  "psp-ui: HTML entities in JSX text are not decoded by the JSX " +
                    'renderer - write the literal character (é, ♥) or a string expression {"\\u00e9"} instead.',
                );
              }
              add(path.node.value);
            },
            JSXAttribute(path) {
              const name = path.node.name;
              if (name.type === "JSXIdentifier" && name.name === "classList") {
                throw path.buildCodeFrameError(
                  "psp-ui: `classList` is not supported (v1). Use ternaries of FULL class literals: " +
                    'class={cond() ? "p-2 bg-red-500" : "p-2 bg-slate-700"}',
                );
              }
              if (name.type === "JSXIdentifier" && name.name === "class") {
                const v = path.node.value;
                if (
                  v?.type === "JSXExpressionContainer" &&
                  v.expression.type === "TemplateLiteral" &&
                  v.expression.expressions.length > 0
                ) {
                  throw path.buildCodeFrameError(
                    "psp-ui: template-interpolated class fragments are not supported (v1). " +
                      "Styles compile at build time - use ternaries of FULL literals.",
                  );
                }
              }
            },
          });
        },
      },
    },
  };
}

async function hashKey(path: string, src: string, engine: JsxEngine): Promise<string> {
  const h = new Bun.CryptoHasher("sha256");
  h.update(
    CACHE_VERSION +
      "\0" +
      engine +
      "\0" +
      reactPresetPkg.version +
      "\0" +
      vueJsxPkg.version +
      "\0" +
      solidPresetPkg.version +
      "\0" +
      babelCorePkg.version +
      "\0" +
      tsPresetPkg.version +
      "\0" +
      path +
      "\0",
  );
  h.update(src);
  return h.digest("hex");
}

interface CacheEntry {
  code: string;
  classStrings: string[];
  textCodepoints: number[];
}

function transformOptions(engine: JsxEngine) {
  if (engine === "react") {
    return {
      presets: [
        [reactPreset, { runtime: "automatic", importSource: "react" }],
        [tsPreset, { isTSX: true, allExtensions: true }],
      ],
      plugins: [] as unknown[],
    };
  }
  if (engine === "solid") {
    return {
      presets: [
        [solidPreset, { generate: "universal", moduleName: RENDERER_SOLID_PATH }],
        [tsPreset, { isTSX: true, allExtensions: true }],
      ],
      plugins: [] as unknown[],
    };
  }
  return {
    presets: [[tsPreset, { isTSX: true, allExtensions: true }]],
    plugins: [[vueJsxPlugin, { enableObjectSlots: false }]],
  };
}

/**
 * Transform one source file (content-hash cached). Throws on lint violations
 * and syntax errors; the error message carries file + code frame.
 */
export async function transformFile(
  path: string,
  src: string,
  engine: JsxEngine,
): Promise<TransformResult> {
  const key = await hashKey(path, src, engine);
  const cacheFile = CACHE_DIR + key + ".json";
  const cached = (await Bun.file(cacheFile).json().catch(() => null)) as CacheEntry | null;
  if (cached && typeof cached.code === "string") {
    return {
      code: cached.code,
      classStrings: cached.classStrings,
      textCodepoints: new Set(cached.textCodepoints),
    };
  }

  const collected: Collected = { classStrings: [], textCodepoints: new Set() };
  const opts = transformOptions(engine);
  const res = await transformAsync(src, {
    filename: path,
    presets: opts.presets,
    plugins: [makeCollector(collected), ...opts.plugins] as never,
    babelrc: false,
    configFile: false,
    sourceMaps: false,
  });
  if (!res?.code && res?.code !== "") {
    throw new Error(`psp-ui transform produced no output for ${path}`);
  }

  const entry: CacheEntry = {
    code: res.code!,
    classStrings: collected.classStrings,
    textCodepoints: [...collected.textCodepoints],
  };
  await Bun.write(cacheFile, JSON.stringify(entry));
  return { code: entry.code, classStrings: entry.classStrings, textCodepoints: collected.textCodepoints };
}

function sourcePathForEngine(path: string, engine: JsxEngine): string {
  if (engine === "vue") {
    if (path === RENDERER_PATH) return RENDERER_VUE_PATH;
    if (path === COMPONENTS_PATH) return COMPONENTS_VUE_PATH;
  }
  if (engine === "solid") {
    if (path === RENDERER_PATH) return RENDERER_SOLID_PATH;
    if (path === COMPONENTS_PATH) return COMPONENTS_SOLID_PATH;
    if (path === FRAME_PATH) return FRAME_SOLID_PATH;
    if (path === HOOKS_PATH) return HOOKS_SOLID_PATH;
    if (path === REACTIVITY_PATH) return REACTIVITY_SOLID_PATH;
  }
  return path;
}

export function jsxPlugin(engine: JsxEngine, opts: { entry?: string } = {}): BunPlugin {
  return {
    name: `psp-ui-${engine}-jsx`,
    setup(build) {
      if (engine === "react") {
        build.onResolve({ filter: /^react$/ }, () => ({ path: REACT_COMPAT_PATH }));
        build.onResolve({ filter: /^react\/jsx-(?:dev-)?runtime$/ }, () => ({
          path: REACT_JSX_RUNTIME_PATH,
        }));
      }
      build.onLoad({ filter: /\.tsx?$/ }, async (args) => {
        if (args.path.includes("/node_modules/") || args.path.endsWith(".d.ts")) return undefined;
        const sourcePath = sourcePathForEngine(args.path, engine);
        let src = await Bun.file(sourcePath).text();
        if (args.path === opts.entry) {
          src = `import "psp-ui/prelude";\n${src}`;
        }
        const { code } = await transformFile(sourcePath, src, engine);
        return { contents: code, loader: "js" };
      });
    },
  };
}
