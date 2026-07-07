// cine/test/smoke-film.ts — 2-scene pipeline smoke test (placeholder art).
// Exercises: gradient raster, parallax pan, sprite walk, captions (CJK+ASCII),
// dialog, choice branch, control walk, mash+counter, letterbox, mosaic, fades,
// affine zoom/spin, wave raster, scene transition, gotoScene loop guard.

import {
  defineFilm, defineScene, cue, image, gradient, sprite,
  fadeIn, fadeOut, wait, waitA, waitTweens, caption, captionClear, dialog, choice,
  pan, letterbox, mosaicTo, shake, alpha, zoom, spinTo, show, hide, animate,
  moveTo, walkTo, control, mash, counter, affineOn, sfx, gotoScene, setFlag, hasFlag,
  rasterWave, rasterOff,
} from "@pocketjs/cine";

const street = defineScene({
  id: "street",
  sky: gradient("#0a1430", "#2a4a7a", "#e8965a"),
  far: image("art/hills.png", { scroll: 0.35 }),
  main: image("art/street.png", { wide: true }),
  actors: {
    walker: sprite("art/walker.png", { w: 32, h: 32, frames: 2, fps: 8, at: [60, 100] }),
  },
  play: cue(function* () {
    yield letterbox(16, 1);
    yield fadeIn(30);
    yield caption("chip", "1990年代 · 某条街");
    yield wait(20);
    yield show("walker");
    yield letterbox(0, 30);
    yield pan(144, 120, "inout");
    yield walkTo("walker", 240, 120);
    yield caption("sub", "他沿着街走。Hello GBA!");
    yield waitA();
    yield captionClear("all");
    yield dialog("路人", "要不要自己走一段?");
    const c = yield choice(["好啊", "算了"]);
    if (c === 0) {
      yield setFlag("walked");
      yield control("walker", 330, 1.5);
    } else {
      yield walkTo("walker", 330, 90);
    }
    yield caption("sub", "按 A 收集星星!");
    yield counter("stars", 200, 24);
    yield mash("stars", 5);
    yield captionClear("all");
    yield shake(3, 40);
    yield mosaicTo(12, 40);
    yield fadeOut(30);
  }),
});

const dream = defineScene({
  id: "dream",
  sky: gradient("#050510", "#1a1035"),
  backdrop: "#050510",
  wave: { layer: "main", amp: 3 },
  main: image("art/hills.png"),
  actors: {
    emblem: sprite("art/emblem.png", { w: 32, h: 32, at: [120, 70], screen: true }),
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("card", "梦境 DREAM");
    yield wait(30);
    yield show("emblem", 112, 60);
    yield affineOn("emblem");
    yield zoom(0.3, 1);
    yield zoom(1.5, 60, "out");
    yield spinTo(360, 90, "inout");
    yield waitTweens();
    yield rasterOff();
    yield alpha(8, 8, 40);
    yield waitA();
    if (yield hasFlag("walked")) {
      yield caption("sub", "你走过了那条街。");
    } else {
      yield caption("sub", "旁观也是一种走法。");
    }
    yield waitA();
    yield captionClear("all");
    yield fadeOut(30);
    yield gotoScene("street");
  }),
});

export default defineFilm({ title: "CINE SMOKE", scenes: [street, dream] });
