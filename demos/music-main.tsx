// @title psp-ui: Now Playing
import Music, { musicFrame } from "./music.tsx";
import { mount } from "../src/index.ts";

mount(() => <Music />, { beforeFrame: musicFrame });
