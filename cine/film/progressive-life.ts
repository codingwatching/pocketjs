// 《渐进人生 · A PROGRESSIVE LIFE》
//
// An interactive pixel-art life montage of 尤雨溪 (Evan You) — creator of
// Vue.js and Vite — in the spirit of the opening montage of "Up".
// A fan tribute (同人致敬). All dialogue is original writing; events follow
// public interviews and first-party posts. Not affiliated with anyone.
//
//   bun compiler/cli.ts build film/progressive-life.ts --out dist/progressive-life.gba
//
// GBA feature bingo, on purpose: per-scanline gradient skies (HBlank raster),
// 64x32 wide pans, parallax + autoscroll, WIN0 letterbox, mosaic, white/black
// BLDY fades, ghost alpha, affine zoom/spin emblems, wave raster, typewriter
// CJK captions, PSG blips, and playable beats in every scene.

import {
  defineFilm, defineScene, cue, image, gradient, sprite,
  fadeIn, fadeOut, wait, waitA, waitTweens, caption, captionClear, dialog, choice,
  pan, letterbox, mosaicTo, shake, alpha, zoom, spinTo, show, hide, animate,
  moveTo, walkTo, control, mash, counter, counterHide, affineOn, affineOff, sfx,
  gotoScene, setVar, varEq, rasterWave, rasterOff,
} from "@pocketjs/cine";

// ---------------------------------------------------------------------------
// 0 · 标题 — chapter select boots straight in (真机验收要求)
// ---------------------------------------------------------------------------
const title = defineScene({
  id: "title",
  sky: gradient("#0b1f24", "#123832", "#2a6b4f", "#0b1f24"),
  far: image("art/far_skyline.png", { scroll: 0.3, y: 80 }),
  backdrop: "#0b1f24",
  actors: {
    vlogo: sprite("art/spr_vuelogo.png", { w: 64, h: 64, screen: true }),
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield show("vlogo", 120, 32);
    yield affineOn("vlogo");
    yield zoom(0.2, 1);
    yield spinTo(360, 80, "out");
    yield zoom(1.0, 80, "out");
    yield caption("card", "渐进人生");
    yield wait(30);
    yield caption("sub", "A PROGRESSIVE LIFE\n同人致敬 · 非官方作品");
    yield waitTweens();
    yield wait(40);
    yield captionClear("sub");
    while (yield varEq("nav", 0)) {
      const m = yield choice(["从头播放", "选择章节"]);
      if (m === 0) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("paint486");
      }
      const p = yield choice(["486与画笔", "水族馆", "一个URL", "取名那晚", "后面的章节…"]);
      if (p === 0) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("paint486");
      }
      if (p === 1) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("aquarium");
      }
      if (p === 2) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("nyc");
      }
      if (p === 3) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("seed");
      }
      const q = yield choice(["那封信", "星星", "OnePiece与闪电", "启航", "山丘与片尾"]);
      if (q === 0) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("letter");
      }
      if (q === 1) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("stars");
      }
      if (q === 2) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("onepiece");
      }
      if (q === 3) {
        yield setVar("nav", 1);
        yield captionClear("all");
        yield fadeOut(30);
        yield gotoScene("fleet");
      }
      yield setVar("nav", 1);
      yield captionClear("all");
      yield fadeOut(30);
      yield gotoScene("coda");
    }
  }),
});

// ---------------------------------------------------------------------------
// 1 · 486 与画笔 — 无锡,1990年代
// ---------------------------------------------------------------------------
const paint486 = defineScene({
  id: "paint486",
  main: image("art/bg_room90s.png"),
  backdrop: "#000000",
  actors: {
    kid: sprite("art/spr_kid.png", { w: 32, h: 32, at: [28, 100] }),
    dad: sprite("art/spr_dad.png", { w: 32, h: 32, at: [120, 98] }),
    drawing: sprite("art/spr_drawing.png", { w: 32, h: 32, screen: true }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield letterbox(12, 1);
    yield fadeIn(45);
    yield caption("chip", "无锡 · 1990年代");
    yield wait(20);
    yield show("dad");
    yield show("kid");
    yield letterbox(0, 30);
    yield dialog("父亲", "搬回来的时候他说:\n以后,计算机就是未来。");
    yield hide("dad");
    yield caption("sub", "←→ 走到那台 486 前");
    yield control("kid", 168, 1.2);
    yield captionClear("all");
    yield dialog("少年", "(按下电源。)");
    yield show("drawing", 192, 60);
    yield sfx("star");
    yield zoom(1.0, 1);
    yield wait(20);
    yield caption("sub", "他没有先学会写程序,\n他先学会了画画。");
    yield waitA();
    yield captionClear("all");
    yield hide("drawing");
    yield mosaicTo(10, 40);
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 2 · 水族馆 — 上海,高中
// ---------------------------------------------------------------------------
const aquarium = defineScene({
  id: "aquarium",
  main: image("art/bg_classroom.png"),
  backdrop: "#000000",
  actors: {
    fish1: sprite("art/spr_fish.png", { w: 32, h: 32 }),
    fish2: sprite("art/spr_fish.png", { w: 32, h: 32 }),
    fish3: sprite("art/spr_fish.png", { w: 32, h: 32 }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield fadeIn(40);
    yield caption("chip", "上海 · 高中机房");
    yield wait(10);
    yield caption("sub", "语文课代表没想到,\n他交上来的是一座水族馆。");
    yield waitA();
    yield captionClear("all");
    yield rasterWave("main", 2);
    yield sfx("whoosh");
    yield show("fish1", -40, 34);
    yield show("fish2", -70, 52);
    yield show("fish3", -100, 68);
    yield moveTo("fish1", 260, 30, 240, "linear");
    yield moveTo("fish2", 260, 56, 300, "linear");
    yield moveTo("fish3", 260, 66, 360, "linear");
    yield wait(90);
    yield dialog("全班", "哇——");
    yield shake(2, 30);
    yield wait(60);
    yield rasterOff();
    yield caption("sub", "他一直记得的,\n是大家抬起头时的表情。");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 3 · 一个 URL — 纽约,2011
// ---------------------------------------------------------------------------
const nyc = defineScene({
  id: "nyc",
  main: image("art/bg_nyc.png"),
  backdrop: "#000000",
  actors: {
    evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [30, 106] }),
    book: sprite("art/spr_book.png", { w: 32, h: 32 }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield letterbox(12, 1);
    yield fadeIn(45);
    yield caption("chip", "纽约 · 2011");
    yield wait(20);
    yield show("evan");
    yield show("book", 44, 94);
    yield caption("sub", "45 分钟的公交车,\n一本厚厚的 JavaScript。");
    yield waitA();
    yield captionClear("sub");
    yield hide("book");
    yield letterbox(0, 30);
    yield caption("sub", "←→ 穿过雨夜,回到宿舍");
    yield control("evan", 330, 1.4);
    yield captionClear("all");
    yield caption("sub", "半夜,他把一个小实验\n放上了网。");
    yield waitA();
    yield captionClear("sub");
    yield setVar("hn", 88);
    yield counter("hn", 210, 26);
    yield caption("sub", "连按 A —— 它正在上首页!");
    yield mash("hn", 100);
    yield sfx("confirm");
    yield captionClear("all");
    yield wait(20);
    yield caption("sub", "一个 URL,可以寄给\n世界上任何一个人。");
    yield waitA();
    yield captionClear("all");
    yield counterHide();
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 4 · 取名那晚 — 2013 → 2014.2
// ---------------------------------------------------------------------------
const seed = defineScene({
  id: "seed",
  main: image("art/bg_office.png"),
  backdrop: "#000000",
  actors: {
    evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [96, 104] }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield fadeIn(40);
    yield caption("chip", "谷歌 · 2013");
    yield wait(10);
    yield show("evan");
    yield caption("sub", "白天做原型;晚上只想留下\n自己最喜欢的那一小部分。");
    yield waitA();
    yield captionClear("all");
    yield dialog("npm", "错误:seed 这个名字\n已经被占用了。");
    const c = yield choice(["View", "Vue", "Vista"]);
    if (c === 1) {
      yield dialog("他", "法语里的 View。\n就它了。");
    } else {
      yield dialog("他", "嗯……不如换成法语?\nVue。就它了。");
    }
    yield captionClear("all");
    yield caption("chip", "2014年2月 · Hacker News");
    yield setVar("stars", 0);
    yield counter("stars", 210, 26);
    yield caption("sub", "连按 A —— 第一批星星!");
    yield mash("stars", 16);
    yield sfx("confirm");
    yield captionClear("all");
    yield caption("sub", "第一周,几百颗星。\n真的有人在用它。");
    yield waitA();
    yield captionClear("all");
    yield counterHide();
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 5 · 那封信 — 新泽西,2016年2月(全片的核心)
// ---------------------------------------------------------------------------
const letter = defineScene({
  id: "letter",
  main: image("art/bg_kitchen.png"),
  backdrop: "#000000",
  actors: {
    evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [66, 104] }),
    wife: sprite("art/spr_wife.png", { w: 32, h: 32, at: [156, 104] }),
    mail: sprite("art/spr_letter.png", { w: 32, h: 32, screen: true }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield setVar("jumped", 0);
    yield letterbox(14, 1);
    yield fadeIn(60);
    yield caption("chip", "新泽西 · 2016年2月");
    yield wait(20);
    yield show("evan");
    yield show("wife");
    yield caption("sub", "他想辞掉工作,全职做开源。\nPatreon:每月四千美元。");
    yield waitA();
    yield captionClear("sub");
    yield show("mail", 104, 64);
    yield affineOn("mail");
    yield zoom(0.4, 1);
    yield zoom(1.6, 70, "out");
    yield sfx("blip");
    yield dialog("信箱", "一封保险续保信。\n她这才知道了这个计划。");
    yield affineOff("mail");
    yield hide("mail");
    yield dialog("妻子", "……去试吧。");
    yield dialog("妻子", "六个月。不行的话,\n我就把你踢回大厂上班。");
    while (yield varEq("jumped", 0)) {
      const j = yield choice(["跳", "再想想"]);
      if (j === 0) {
        yield setVar("jumped", 1);
      } else {
        yield dialog("他", "(窗外,雪停了。)");
        yield caption("sub", "有些事现在不试,\n会想一辈子。");
        yield waitA();
        yield captionClear("sub");
      }
    }
    yield sfx("whoosh");
    yield shake(2, 30);
    yield caption("sub", "他跳了。");
    yield wait(50);
    yield captionClear("all");
    yield fadeOut(50, "white");
  }),
});

// ---------------------------------------------------------------------------
// 6 · 星星 — 2016 → 2018.6.16
// ---------------------------------------------------------------------------
const stars = defineScene({
  id: "stars",
  main: image("art/bg_stage.png"),
  backdrop: "#000000",
  actors: {
    evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [150, 78] }),
    star1: sprite("art/spr_star.png", { w: 32, h: 32, ghost: true }),
    star2: sprite("art/spr_star.png", { w: 32, h: 32, ghost: true }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield fadeIn(45, "white");
    yield caption("chip", "2016 → 2018");
    yield wait(10);
    yield show("evan");
    yield caption("sub", "Vue 2.0。讲台越来越大,\n台下的人来自世界各地。");
    yield waitA();
    yield captionClear("all");
    yield show("star1", 60, 120);
    yield show("star2", 180, 130);
    yield moveTo("star1", 70, 20, 180, "out");
    yield moveTo("star2", 190, 10, 220, "out");
    yield caption("sub", "2018年6月16日。");
    yield wait(40);
    yield dialog("社区", "Vue 的星星数,\n超过了 React!");
    yield shake(1, 20);
    yield sfx("star");
    yield captionClear("all");
    yield caption("sub", "后来他说,从那天起,\n他反而不再盯着星星看了。");
    yield waitA();
    yield captionClear("all");
    yield hide("star1");
    yield hide("star2");
    yield caption("sub", "←→ 走下讲台");
    yield control("evan", 24, 1.2);
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 7 · One Piece — 2018 → 2020.9.18
// ---------------------------------------------------------------------------
const onepiece = defineScene({
  id: "onepiece",
  main: image("art/bg_deskstorm.png"),
  backdrop: "#000000",
  actors: {
    flag: sprite("art/spr_flag.png", { w: 32, h: 32 }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield fadeIn(40);
    yield caption("chip", "重写 Vue 3 · 两年");
    yield rasterWave("main", 2);
    yield shake(1, 120);
    yield caption("sub", "三十份 RFC,\n两千六百次提交。");
    yield waitA();
    yield captionClear("sub");
    yield dialog("风暴", "别动我们熟悉的写法!");
    yield dialog("他", "那就谁都不落下:\n旧的写法,永远保留。");
    yield rasterOff();
    yield caption("chip", "2020年9月18日");
    yield show("flag", 52, 150);
    yield moveTo("flag", 52, 44, 100, "inout");
    yield sfx("confirm");
    yield waitTweens();
    yield caption("sub", "Vue 3 起航,代号:\nOne Piece。");
    yield waitA();
    yield captionClear("all");
    yield wait(10);
    yield fadeOut(16, "white");
  }),
});

// ---------------------------------------------------------------------------
// 8 · 闪电 — 2020.4,Vite
// ---------------------------------------------------------------------------
const vite = defineScene({
  id: "vite",
  sky: gradient("#1a1030", "#43307a", "#c98a4b", "#f5d08a"),
  backdrop: "#1a1030",
  actors: {
    bolt: sprite("art/spr_bolt.png", { w: 32, h: 32, screen: true }),
    orb1: sprite("art/spr_orb.png", { w: 32, h: 32, ghost: true, screen: true }),
    orb2: sprite("art/spr_orb.png", { w: 32, h: 32, ghost: true, screen: true }),
    orb3: sprite("art/spr_orb.png", { w: 32, h: 32, ghost: true, screen: true }),
    orb4: sprite("art/spr_orb.png", { w: 32, h: 32, ghost: true, screen: true }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield fadeIn(24, "white");
    yield caption("chip", "2020年4月");
    yield show("bolt", 104, 44);
    yield affineOn("bolt");
    yield zoom(0.3, 1);
    yield zoom(1.5, 50, "out");
    yield spinTo(360, 70, "inout");
    yield sfx("whoosh");
    yield waitTweens();
    yield caption("sub", "同一个春天,他放出了\n一道法语的『快』:Vite。");
    yield waitA();
    yield captionClear("sub");
    yield caption("card", "npm create vite");
    yield wait(50);
    yield captionClear("all");
    yield caption("sub", "冷启动:0.3 秒。");
    yield wait(60);
    yield show("orb1", -20, 30);
    yield show("orb2", 250, 40);
    yield show("orb3", -20, 100);
    yield show("orb4", 250, 110);
    yield moveTo("orb1", 84, 52, 90, "inout");
    yield moveTo("orb2", 140, 52, 100, "inout");
    yield moveTo("orb3", 84, 84, 110, "inout");
    yield moveTo("orb4", 140, 84, 120, "inout");
    yield waitTweens();
    yield captionClear("sub");
    yield caption("sub", "一个个框架,\n都插上了这道闪电。");
    yield waitA();
    yield captionClear("sub");
    yield caption("sub", "有人管它叫:\nJavaScript 的联合国。");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------
// 9 · 启航 — 新加坡,2024 → 2026
// ---------------------------------------------------------------------------
const fleet = defineScene({
  id: "fleet",
  main: image("art/bg_harbor.png"),
  far: image("art/far_waves.png", { scroll: 0.5, y: 112, vx: -0.25 }),
  backdrop: "#000000",
  wave: { layer: "far", amp: 1 },
  actors: {
    ship: sprite("art/spr_ship.png", { w: 64, h: 32 }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield letterbox(12, 1);
    yield fadeIn(50);
    yield caption("chip", "新加坡 · 2024");
    yield wait(20);
    yield show("ship", -70, 84);
    yield moveTo("ship", 150, 82, 300, "inout");
    yield pan(80, 300, "inout");
    yield caption("sub", "独木舟换成了一艘船:\nVoidZero。");
    yield waitA();
    yield captionClear("sub");
    yield caption("sub", "船员们用 Rust 打龙骨:\nRolldown,Oxc。");
    yield waitA();
    yield captionClear("sub");
    yield pan(144, 240, "inout");
    yield moveTo("ship", 260, 80, 260, "inout");
    yield caption("chip", "2026年6月");
    yield caption("sub", "船驶进了一座橙色的港湾。\n桅杆上的旗语没有换:");
    yield waitA();
    yield captionClear("all");
    yield caption("card", "OPEN & NEUTRAL");
    yield wait(80);
    yield captionClear("all");
    yield caption("sub", "船还是那艘船,\n海图仍然对所有人公开。");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(50);
  }),
});

// ---------------------------------------------------------------------------
// 10 · 山丘与片尾
// ---------------------------------------------------------------------------
const coda = defineScene({
  id: "coda",
  main: image("art/bg_hill.png"),
  backdrop: "#000000",
  actors: {
    evan: sprite("art/spr_evan.png", { w: 32, h: 32, at: [96, 76] }),
    wife: sprite("art/spr_wife.png", { w: 32, h: 32, at: [130, 76] }),
    kid1: sprite("art/spr_child.png", { w: 32, h: 32, at: [118, 82] }),
    kid2: sprite("art/spr_child.png", { w: 32, h: 32, at: [148, 82], flip: true }),
    vlogo: sprite("art/spr_vuelogo.png", { w: 64, h: 64, ghost: true, screen: true }),
  },
  play: cue(function* () {
    yield setVar("nav", 0);
    yield letterbox(14, 1);
    yield fadeIn(70);
    yield show("evan");
    yield show("wife");
    yield show("kid1");
    yield show("kid2");
    yield wait(30);
    yield caption("sub", "框架是渐进式的。\n人生,原来也是。");
    yield waitA();
    yield captionClear("sub");
    yield show("vlogo", 120, 28);
    yield affineOn("vlogo");
    yield zoom(0.4, 1);
    yield zoom(0.9, 120, "inout");
    yield spinTo(180, 240, "inout");
    yield caption("card", "0.9 Animatrix · 2014");
    yield wait(70);
    yield caption("card", "1.0 Evangelion · 2015");
    yield wait(70);
    yield caption("card", "2.0 Ghost in the Shell");
    yield wait(70);
    yield caption("card", "3.0 One Piece · 2020");
    yield wait(70);
    yield caption("card", "3.5 Gurren Lagann · 2024");
    yield wait(70);
    yield caption("card", "U — ???");
    yield wait(50);
    yield caption("sub", "下一集,还没有写完。");
    yield waitA();
    yield captionClear("all");
    yield caption("card", "渐进人生");
    yield caption("sub", "A FAN TRIBUTE · 2026\n谢谢你,尤大。");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(70);
    yield gotoScene("title");
  }),
});

export default defineFilm({
  title: "渐进人生 A PROGRESSIVE LIFE",
  scenes: [title, paint486, aquarium, nyc, seed, letter, stars, onepiece, vite, fleet, coda],
});
