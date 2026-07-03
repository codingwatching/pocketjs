// @title psp-ui: Game Library
import Library, { libraryFrame } from "./library.tsx";
import { mount } from "../src/index.ts";

mount(() => <Library />, { beforeFrame: libraryFrame });
