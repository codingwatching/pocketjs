// Native tree mirror + HostOps mutation helpers shared by the React-compatible and Vue
// renderers.

import { NODE_TYPE, PROP, ROOT_ID, STYLE_ID_NONE, type PropName } from "../spec/spec.ts";
import { encodePropValue, getHost, getOps } from "./host.ts";
import { notifyDetached, registerFocusable, registerPress } from "./input.ts";

export interface NodeMirror {
  /** Native generation-tagged node id. */
  id: number;
  /** spec NODE_TYPE ordinal. */
  type: number;
  parent: NodeMirror | null;
  children: NodeMirror[];
  /** Current text (text nodes only). */
  text?: string;
  /** Focus traversal membership (input.ts). */
  focusable?: boolean;
  /** CIRCLE handler while focused (input.ts). */
  onPress?: (() => void) | undefined;
}

/** Mirror of the pre-created native root (full-screen flex column, id 1). */
export const rootMirror: NodeMirror = {
  id: ROOT_ID,
  type: NODE_TYPE.view,
  parent: null,
  children: [],
};

let styleResolver: ((cls: string) => number | undefined) | null = null;

/** Wire the class->styleId lookup (index.ts injects styles.ts's resolveStyle). */
export function setStyleResolver(fn: (cls: string) => number | undefined): void {
  styleResolver = fn;
}

/** Non-strict-host miss counters (PSP: don't crash, count). */
export const missCounters = { unknownClass: 0, unknownTexture: 0 };

const textures = new Map<string, number>();

/** Bind an image key (the `src` string) to an uploadTexture handle. */
export function registerTexture(key: string, handle: number): void {
  textures.set(key, handle);
}

export function resetTextures(): void {
  textures.clear();
}

const sweepSet = new Set<NodeMirror>();
const retained = new Set<NodeMirror>();

/** Keep a detached subtree alive across frames (skip the sweep). */
export function retain(node: NodeMirror): void {
  retained.add(node);
  sweepSet.delete(node);
}

/** Undo retain(); a still-detached node re-enters the next sweep. */
export function release(node: NodeMirror): void {
  retained.delete(node);
  if (node.parent === null && node !== rootMirror) sweepSet.add(node);
}

function subtreeHasRetained(node: NodeMirror): boolean {
  if (retained.has(node)) return true;
  for (let i = 0; i < node.children.length; i++) {
    if (subtreeHasRetained(node.children[i])) return true;
  }
  return false;
}

/**
 * Destroy every subtree removed during the frame and still detached. Called
 * once per frame by globalThis.frame after app code and input handlers ran.
 */
export function runSweep(): void {
  if (sweepSet.size === 0) return;
  const ops = getOps();
  const keep: NodeMirror[] = [];
  for (const node of sweepSet) {
    if (node.parent !== null) continue;
    if (subtreeHasRetained(node)) {
      keep.push(node);
      continue;
    }
    ops.destroyNode(node.id);
  }
  sweepSet.clear();
  for (let i = 0; i < keep.length; i++) sweepSet.add(keep[i]);
}

/** Tests: drop sweep/retain state without touching the native tree. */
export function resetRendererState(): void {
  sweepSet.clear();
  retained.clear();
  rootMirror.children.length = 0;
}

export function createElement(tag: string): NodeMirror {
  const type = (NODE_TYPE as Record<string, number>)[tag];
  if (type === undefined) {
    throw new Error(`psp-ui: unknown element <${tag}> - only view/text/image exist`);
  }
  return { id: getOps().createNode(type), type, parent: null, children: [] };
}

export function createTextNode(value: string): NodeMirror {
  const ops = getOps();
  const id = ops.createNode(NODE_TYPE.text);
  ops.setText(id, value);
  return { id, type: NODE_TYPE.text, parent: null, children: [], text: value };
}

export function replaceText(node: NodeMirror, value: string): void {
  getOps().replaceText(node.id, value);
  node.text = value;
}

export function isTextNode(node: NodeMirror): boolean {
  return node.type === NODE_TYPE.text;
}

/** Unlink from the current mirror parent (native insertBefore self-unlinks). */
function unlink(node: NodeMirror): void {
  const p = node.parent;
  if (!p) return;
  const i = p.children.indexOf(node);
  if (i >= 0) p.children.splice(i, 1);
  node.parent = null;
}

export function insertNode(parent: NodeMirror, node: NodeMirror, anchor?: NodeMirror | null): void {
  const ops = getOps();
  unlink(node);
  sweepSet.delete(node);
  ops.insertBefore(parent.id, node.id, anchor ? anchor.id : 0);
  if (anchor) {
    const i = parent.children.indexOf(anchor);
    if (i < 0) throw new Error("psp-ui: insert anchor is not a child of parent");
    parent.children.splice(i, 0, node);
  } else {
    parent.children.push(node);
  }
  node.parent = parent;
}

export function removeNode(parent: NodeMirror, node: NodeMirror): void {
  notifyDetached(node);
  getOps().removeChild(parent.id, node.id);
  unlink(node);
  sweepSet.add(node);
}

export function detachNode(parent: NodeMirror, node: NodeMirror): void {
  removeNode(parent, node);
}

export function getParentNode(node: NodeMirror): NodeMirror | undefined {
  return node.parent ?? undefined;
}

export function getFirstChild(node: NodeMirror): NodeMirror | undefined {
  return node.children[0];
}

export function getNextSibling(node: NodeMirror): NodeMirror | undefined {
  const p = node.parent;
  if (!p) return undefined;
  const i = p.children.indexOf(node);
  return i >= 0 ? p.children[i + 1] : undefined;
}

function setClass(node: NodeMirror, value: unknown): void {
  const ops = getOps();
  if (value == null || value === "") {
    ops.setStyle(node.id, STYLE_ID_NONE);
    return;
  }
  if (typeof value !== "string") {
    throw new Error("psp-ui: class must be a string literal of utilities");
  }
  const styleId = styleResolver ? styleResolver(value) : undefined;
  if (styleId === undefined) {
    if (getHost().strict) {
      throw new Error(
        `psp-ui: unknown class "${value}" - not in the compiled style table ` +
          "(dynamic classes must be ternaries of full literals)",
      );
    }
    missCounters.unknownClass++;
    return;
  }
  ops.setStyle(node.id, styleId);
}

function setSrc(node: NodeMirror, value: unknown): void {
  const ops = getOps();
  if (value == null || value === "") {
    ops.setImage(node.id, -1);
    return;
  }
  if (typeof value !== "string") {
    throw new Error("psp-ui: src must be a string key");
  }
  const handle = textures.get(value);
  if (handle === undefined) {
    if (getHost().strict) {
      throw new Error(
        `psp-ui: unknown image src "${value}" - no texture registered under that key`,
      );
    }
    missCounters.unknownTexture++;
    return;
  }
  ops.setImage(node.id, handle);
}

type StyleObject = Record<string, number | string>;

function setStyleObject(node: NodeMirror, value: unknown, prev: unknown): void {
  const ops = getOps();
  const next = (value ?? {}) as StyleObject;
  const before = (prev ?? {}) as StyleObject;
  for (const key in next) {
    const v = next[key];
    if (before[key] === v) continue;
    const propId = (PROP as Record<string, number>)[key];
    if (propId === undefined) {
      throw new Error(`psp-ui: unknown style prop '${key}' (see spec PROP)`);
    }
    ops.setProp(node.id, propId, encodePropValue(key as PropName, v));
  }
}

export function setProp<T>(node: NodeMirror, name: string, value: T, prev?: T): void {
  if (value === prev && name !== "style") return;
  if (name === "className") name = "class";
  switch (name) {
    case "class":
      setClass(node, value);
      return;
    case "onPress":
    case "on:press":
      registerPress(node, value as (() => void) | undefined);
      return;
    case "src":
      setSrc(node, value);
      return;
    case "style":
      setStyleObject(node, value, prev);
      return;
    case "focusable":
      registerFocusable(node, !!value);
      return;
    case "ref":
    case "nodeRef":
    case "key":
    case "children":
      return;
    default:
      break;
  }
  if (name === "classList") {
    throw new Error(
      "psp-ui: classList is not supported - use ternaries of full class literals",
    );
  }
  if (name.startsWith("on:") || name.startsWith("bool:") || name.startsWith("prop:")) {
    throw new Error(`psp-ui: unsupported namespaced attribute '${name}'`);
  }
  throw new Error(`psp-ui: unknown property '${name}' on <${tagName(node)}>`);
}

export type HostProps = Record<string, unknown>;

export function applyProps(node: NodeMirror, next: HostProps, prev: HostProps = {}): void {
  const seen = new Set<string>();
  for (const key of Object.keys(next)) {
    seen.add(key);
    setProp(node, key, next[key], prev[key]);
  }
  for (const key of Object.keys(prev)) {
    if (seen.has(key)) continue;
    if (key === "children" || key === "key" || key === "ref" || key === "nodeRef") continue;
    setProp(node, key, undefined, prev[key]);
  }
}

export function clearContainer(container: NodeMirror): void {
  for (const child of [...container.children]) removeNode(container, child);
}

function tagName(node: NodeMirror): string {
  for (const key of Object.keys(NODE_TYPE)) {
    if ((NODE_TYPE as Record<string, number>)[key] === node.type) return key;
  }
  return String(node.type);
}
