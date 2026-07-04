// Vue Vapor renderer over the native `ui.*` tree.

import { createVaporApp, insert as vaporInsert, remove as vaporRemove } from "vue";
import {
  createElement as createNativeElement,
  createTextNode,
  detachNode,
  getFirstChild,
  getNextSibling,
  getParentNode,
  insertNode,
  isTextNode,
  missCounters,
  registerTexture,
  release,
  removeNode,
  replaceText,
  resetRendererState,
  resetTextures,
  retain,
  rootMirror,
  runSweep,
  setProp,
  setStyleResolver,
  type NodeMirror,
} from "./native-tree.ts";

const insertVaporBlock = vaporInsert as unknown as (
  block: unknown,
  parent: NodeMirror,
  anchor?: NodeMirror | null,
) => void;
const removeVaporBlock = vaporRemove as unknown as (block: unknown, parent: NodeMirror) => void;
const createPocketVaporApp = createVaporApp as unknown as (component: { setup: () => unknown }) => {
  mount(root: NodeMirror): void;
  unmount(): void;
};

export {
  createTextNode,
  detachNode,
  getFirstChild,
  getNextSibling,
  getParentNode,
  insertNode,
  isTextNode,
  missCounters,
  registerTexture,
  release,
  replaceText,
  resetRendererState,
  resetTextures,
  retain,
  rootMirror,
  runSweep,
  setProp,
  setStyleResolver,
  type NodeMirror,
};

export interface RenderRoot {
  update(node: unknown): void;
  dispose(): void;
}

export function createElement(type: string): NodeMirror {
  return createNativeElement(type);
}

export function createRenderRoot(root: NodeMirror): RenderRoot {
  let current: unknown;
  return {
    update(node: unknown) {
      if (current) removeVaporBlock(current, root);
      current = node;
      if (current) insertVaporBlock(current, root);
    },
    dispose() {
      if (current) removeVaporBlock(current, root);
      current = undefined;
    },
  };
}

export function render(code: () => unknown, root: NodeMirror): () => void {
  const app = createPocketVaporApp({ setup: code });
  app.mount(root as never);
  return () => app.unmount();
}
