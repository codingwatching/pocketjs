// SEE YOU ON THE MOON —《月球见》
// A Cyberpunk: Edgerunners fan-tribute GBA game, built on @pocketjs/edge.
//
// Unaffiliated, non-commercial fan work. Original Chinese dialogue throughout;
// a handful of famous lines are echoed in translation (see game/dossier.md,
// which also carries the source ledger and every compression liberty taken).
// Music: 8-bit fan covers of "I Really Want to Stay at Your House"
// (Rosa Walton feat. Hallie Coggins), streamed as DirectSound PCM.
//
// Structure (12 scenes, action-first):
//   title → apartment → clinic → ALLEY(action) → train(world) →
//   hideout(world) → WAREHOUSE(action) → rooftop(song) → STREET(action) →
//   LAB(action) → TOWER(boss) → moon(credits)

import {
  defineFilm, defineScene, cue, image, gradient, sprite, track,
  fadeIn, fadeOut, wait, waitA, waitTweens, caption, captionClear, dialog, choice,
  pan, panY, letterbox, mosaicTo, shake, alpha, zoom, show, hide, animate, moveTo,
  walkTo, mash, counter, affineOn, sfx, setFlag, hasFlag, setVar, addVar, varEq, varGt,
  rasterWave, rasterOff, world, meterShow, meterHide, warp, face, walk,
  action, music, autoScroll,
} from "@pocketjs/edge";

// ---------------------------------------------------------------------------------
// S0 — title
// ---------------------------------------------------------------------------------
const title = defineScene({
  id: "title",
  main: image("art/bg_rooftop.png"),
  backdrop: "#04040c",
  letterbox: 24,
  play: cue(function* () {
    yield fadeIn(50);
    yield wait(30);
    yield caption("card", "SEE YOU ON THE MOON\n《月球见》");
    yield wait(40);
    yield caption("chip", "EDGERUNNERS 同人致敬");
    yield caption("sub", "非官方同人作品·剧本原创\n按 A 开始");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S1 — apartment: Gloria and the crash (Ep 1 compressed)
// ---------------------------------------------------------------------------------
const apartment = defineScene({
  id: "apartment",
  main: image("art/bg_apartment.png"),
  backdrop: "#060612",
  actors: {
    david: sprite("art/wd_david.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
    gloria: sprite("art/wd_gloria.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    fallen: sprite("art/spr_gloria_fallen.png", { w: 32, h: 32 }),
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("chip", "夜之城·圣多明戈 2075");
    yield show("david", 60, 112);
    yield show("gloria", 150, 112);
    yield dialog("GLORIA", "又打架了? 大卫,\n学费不是大风刮来的。");
    yield dialog("DAVID", "那学院里全是公司崽,\n他们看我们的眼神像看垃圾。");
    yield dialog("GLORIA", "忍着。总有一天,\n你会站上荒坂塔的顶层。");
    yield dialog("DAVID", "那是你的梦, 妈。");
    yield dialog("GLORIA", "上车吧, 送你上学。");
    yield captionClear("chip");
    yield fadeOut(30);
    // the crash — conveyed, not shown
    yield wait(20);
    yield shake(6, 50);
    yield sfx("whoosh");
    yield caption("card", "帮派火并·流弹\n一场再普通不过的车祸");
    yield wait(80);
    yield captionClear("all");
    yield fadeIn(40);
    yield hide("gloria");
    yield hide("david");
    yield show("fallen", 120, 112);
    yield caption("sub", "创伤小组降落, 扫了一眼\n她不是白金客户。又飞走了。");
    yield waitA();
    yield show("david", 80, 112);
    yield walkTo("david", 104, 60);
    yield dialog("DAVID", "妈?! 妈——");
    yield caption("sub", "第二天, 廉价诊所。\n葛洛莉亚没有醒来。");
    yield waitA();
    yield captionClear("all");
    yield fadeOut(50);
  }),
});

// ---------------------------------------------------------------------------------
// S2 — clinic: the Sandevistan (Ep 1 end)
// ---------------------------------------------------------------------------------
const clinic = defineScene({
  id: "clinic",
  main: image("art/bg_clinic.png"),
  backdrop: "#080410",
  actors: {
    david: sprite("art/wd_david.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("chip", "后巷诊所");
    yield show("david", 104, 112);
    yield caption("sub", "妈妈的遗物里有件军规义体:\n斯安威斯坦, 时间加速器。");
    yield waitA();
    yield dialog("DOC", "军用货。普通人的身体\n撑不过三天就得进焚化炉。");
    yield dialog("DAVID", "装。");
    yield dialog("DOC", "你妈刚走, 你就急着找死?");
    yield dialog("DAVID", "我一无所有了, 医生。\n装上它。");
    yield captionClear("all");
    yield mosaicTo(10, 30);
    yield shake(4, 40);
    yield sfx("star");
    yield wait(40);
    yield mosaicTo(0, 30);
    yield caption("card", "SANDEVISTAN\n脊椎连接·完成");
    yield wait(70);
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S3 — ALLEY: tutorial action stage (fan liberty: Tyger Claws ambush)
// ---------------------------------------------------------------------------------
const alley = defineScene({
  id: "alley",
  main: image("art/stage_alley.png", { wide: true }),
  backdrop: "#0a0618",
  actors: {
    david: sprite("art/act_david.png", { w: 32, h: 32, frames: 8 }),
    thug: sprite("art/en_thug.png", { w: 32, h: 32, frames: 4 }),
  },
  action: {
    player: { actor: "david", hp: 6, sande: 32 },
    ground: 140,
    gates: [
      { x: 170, wave: 1 },
      { x: 290, wave: 2 },
    ],
    spawns: [
      { type: "thug", actor: "thug", x: 210, hp: 2, wave: 1 },
      { type: "thug", actor: "thug", x: 320, hp: 2, wave: 2 },
      { type: "thug", actor: "thug", x: 350, hp: 3, wave: 2 },
    ],
    exit: "clear",
  },
  play: cue(function* () {
    yield fadeIn(30);
    yield caption("chip", "回街区的路上");
    yield dialog("混混", "哟, 学院的小少爷。\n把外套和芯片都留下。");
    yield dialog("DAVID", "…今天真不是好日子。");
    yield captionClear("all");
    yield caption("sub", "十字键移动 A跳 B射击近战\n按住R: 义体超频·子弹时间");
    yield meterShow(0, "hp", 160, 4, 6);
    yield meterShow(1, "sande", 160, 14, 32);
    yield action();
    yield meterHide(0);
    yield meterHide(1);
    yield captionClear("all");
    if (yield varGt("deaths", 0)) {
      yield caption("sub", "身体在报警, 但时间…\n时间站在他这边。");
    } else {
      yield caption("sub", "十秒。他们甚至没碰到他。");
    }
    yield waitA();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S4 — train: meeting Lucy (Ep 2)
// ---------------------------------------------------------------------------------
const train = defineScene({
  id: "train",
  main: image("art/map_train.png"),
  backdrop: "#08101c",
  actors: {
    david: sprite("art/wd_david.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
    lucy: sprite("art/wd_lucy.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
  },
  world: {
    grid: [
      "####################",
      "####################",
      "#########..#########",
      "#########..#########",
      "##..............l.##",
      "#p................d#",
      "##................##",
      "##................##",
      "#########..#########",
      "####################",
    ],
    player: { actor: "david", at: "p", dir: "right" },
    npcs: {
      lucy: {
        actor: "lucy",
        at: "l",
        dir: "left",
        talk: cue(function* () {
          if (yield hasFlag("met_lucy")) {
            yield dialog("LUCY", "跟紧点, 别像个游客。");
            return;
          }
          yield setFlag("met_lucy");
          yield caption("sub", "指尖擦过口袋, 世界慢了。\n你在时间里抓住她的手腕。");
          yield waitA();
          yield captionClear("all");
          yield dialog("LUCY", "…斯安威斯坦?\n有意思, 你不是公司的人。");
          yield dialog("DAVID", "你在偷我东西。");
          yield dialog("LUCY", "偷的是你旁边那位\n公司高管的加密芯片。");
          yield dialog("LUCY", "他的保镖上车了。想活命\n就跟我走, 快。");
          yield caption("sub", "去车厢尽头的门。");
          yield waitA();
          yield captionClear("all");
        }),
      },
    },
    spots: {
      window: {
        at: [9, 1, 2, 1],
        run: cue(function* () {
          yield caption("sub", "车窗外月亮悬在城市上空。\n妈妈说, 顶层的人离它最近。");
          yield waitA();
          yield captionClear("all");
        }),
      },
    },
    exits: { door: { at: "d", value: 1 } },
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("chip", "NCART 高架列车");
    yield world();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S5 — hideout: Maine's crew (Ep 2-3 compressed)
// ---------------------------------------------------------------------------------
const hideout = defineScene({
  id: "hideout",
  main: image("art/map_hideout.png"),
  backdrop: "#0c0810",
  actors: {
    david: sprite("art/wd_david.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
    lucy: sprite("art/wd_lucy.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
    maine: sprite("art/wd_maine.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    rebecca: sprite("art/wd_rebecca.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    dorio: sprite("art/wd_dorio.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    pilar: sprite("art/wd_pilar.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    kiwi: sprite("art/wd_kiwi.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
    falco: sprite("art/wd_falco.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
  },
  world: {
    grid: [
      "####################",
      "####################",
      "####################",
      "#..................#",
      "#..m....o......k...#",
      "#..................#",
      "#..................#",
      "#.....#######......#",
      "#.....#######..l...#",
      "#.##..............##",
      "#.##..r..q.......###",
      "#..................#",
      "#...........f......#",
      "#.........p........#",
      "#########d##########",
    ],
    player: { actor: "david", at: "p", dir: "up" },
    npcs: {
      maine: {
        actor: "maine",
        at: "m",
        dir: "down",
        talk: cue(function* () {
          if (yield hasFlag("joined")) {
            yield dialog("MAINE", "仓库见, 新人。\n别迟到。");
            return;
          }
          yield dialog("MAINE", "露西说你抢了她要的芯片,\n还带着一根军规脊椎。");
          yield dialog("DAVID", "我要入伙。");
          yield dialog("MAINE", "边缘行者不是过家家。\n为什么?");
          const why = yield choice(["我要活下去", "我要往上爬", "为了她"]);
          if (why === 2) {
            yield dialog("MAINE", "哈! 至少够诚实。");
          } else {
            yield dialog("MAINE", "在夜之城, 这两句\n是同一句话。");
          }
          yield dialog("MAINE", "试用任务: 军用科技的仓库,\n拿到货, 活着回来。");
          yield dialog("MAINE", "薪水平分, 后背互相照应。\n这是规矩。");
          yield setFlag("joined");
          yield caption("sub", "走出后门, 前往仓库。");
          yield waitA();
          yield captionClear("all");
        }),
      },
      rebecca: {
        actor: "rebecca",
        at: "r",
        dir: "right",
        talk: cue(function* () {
          yield dialog("REBECCA", "新来的? 枪拿稳点,\n别挡我姐的道。");
          yield dialog("REBECCA", "…开玩笑的。死了太浪费,\n你脸蛋还不错。");
        }),
      },
      dorio: {
        actor: "dorio",
        at: "o",
        dir: "down",
        talk: cue(function* () {
          yield dialog("DORIO", "缅因嘴硬心软。\n他说平分, 就真的平分。");
        }),
      },
      pilar: {
        actor: "pilar",
        at: "q",
        dir: "down",
        talk: cue(function* () {
          yield dialog("PILAR", "嘿新人, 猜猜这双手\n多少钱? 猜不到就请喝酒。");
        }),
      },
      kiwi: {
        actor: "kiwi",
        at: "k",
        dir: "down",
        talk: cue(function* () {
          yield dialog("KIWI", "夜之城的规矩只有一条:\n别相信任何人。");
        }),
      },
      falco: {
        actor: "falco",
        at: "f",
        dir: "down",
        talk: cue(function* () {
          yield dialog("FALCO", "车我来开。你只管\n活着跑回来。");
        }),
      },
      lucy: {
        actor: "lucy",
        at: "l",
        dir: "left",
        talk: cue(function* () {
          yield dialog("LUCY", "别死在第一单, 大卫。\n那样太无聊了。");
        }),
      },
    },
    exits: { back: { at: "d", value: 1 } },
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("chip", "酒吧后间·据点");
    yield world();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S6 — WAREHOUSE: the trial job (Ep 3 compressed)
// ---------------------------------------------------------------------------------
const warehouse = defineScene({
  id: "warehouse",
  main: image("art/stage_warehouse.png", { wide: true }),
  backdrop: "#060a12",
  actors: {
    david: sprite("art/act_david.png", { w: 32, h: 32, frames: 8 }),
    guard: sprite("art/en_guard.png", { w: 32, h: 32, frames: 4 }),
    drone: sprite("art/en_drone.png", { w: 32, h: 32, frames: 4 }),
    turret: sprite("art/en_turret.png", { w: 32, h: 32, frames: 4 }),
    rebecca: sprite("art/wd_rebecca.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
  },
  action: {
    player: { actor: "david", hp: 6, sande: 32 },
    ground: 140,
    gates: [
      { x: 150, wave: 1 },
      { x: 280, wave: 2 },
    ],
    spawns: [
      { type: "gunner", actor: "guard", x: 190, hp: 2, wave: 1 },
      { type: "drone", actor: "drone", x: 240, hp: 2, wave: 1 },
      { type: "gunner", actor: "guard", x: 310, hp: 3, wave: 2 },
      { type: "turret", actor: "turret", x: 356, hp: 4, wave: 2 },
      { type: "drone", actor: "drone", x: 330, hp: 2, wave: 2 },
    ],
    exit: "clear",
  },
  play: cue(function* () {
    yield fadeIn(30);
    yield caption("chip", "军用科技·四号仓库");
    yield show("rebecca", 40, 104);
    yield dialog("REBECCA", "警报响了, 新人!\n货在最里面, 杀出去!");
    yield hide("rebecca");
    yield captionClear("all");
    yield meterShow(0, "hp", 160, 4, 6);
    yield meterShow(1, "sande", 160, 14, 32);
    yield action();
    yield meterHide(0);
    yield meterHide(1);
    yield captionClear("all");
    yield show("rebecca", 300, 104);
    if (yield varGt("deaths", 0)) {
      yield dialog("REBECCA", "活着就行! 缅因说了,\n死人拿不到分成。");
    } else {
      yield dialog("REBECCA", "哇哦。一滴血都没流?\n我开始喜欢你了, 新人。");
    }
    yield dialog("DAVID", "货到手了。回去吧。");
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S7 — rooftop: the moon, the promise, the song (Ep 2+4 merged) ★ insert song
// ---------------------------------------------------------------------------------
const rooftop = defineScene({
  id: "rooftop",
  main: image("art/bg_rooftop.png"),
  backdrop: "#04040c",
  letterbox: 20,
  actors: {
    duo: sprite("art/spr_duo_ledge.png", { w: 64, h: 32 }),
  },
  play: cue(function* () {
    yield music("stay");
    yield fadeIn(60);
    yield caption("chip", "据点楼顶·凌晨三点");
    yield show("duo", 88, 96);
    yield wait(90);
    yield dialog("LUCY", "小时候我总在网络里\n看同一段脑舞。");
    yield dialog("LUCY", "月球。银色的地面,\n刺眼的太阳, 没有网。");
    yield dialog("DAVID", "为什么是月球?");
    yield dialog("LUCY", "在这座城市, 连月亮\n都是别人卖给你的广告。");
    yield dialog("LUCY", "但真正的月球上, 没有人\n能追踪你。自由。");
    yield wait(40);
    yield dialog("DAVID", "那我带你去。");
    yield dialog("LUCY", "…你知道票价吗, 笨蛋?");
    const p = yield choice(["我保证", "一定带你去"]);
    yield setFlag("promise");
    if (p === 0) {
      yield dialog("DAVID", "我带你去月球。我保证。");
    } else {
      yield dialog("DAVID", "等着吧。总有一天,\n我们一起去。");
    }
    yield dialog("LUCY", "…笨蛋。");
    yield captionClear("all");
    yield caption("sub", "♪ I Really Want to Stay\nAt Your House ♪");
    yield wait(180);
    yield captionClear("all");
    yield wait(120);
    yield fadeOut(80);
    yield music("off");
  }),
});

// ---------------------------------------------------------------------------------
// S8 — STREET: the Tanaka job falls apart; Maine's end (Ep 5-6 compressed)
// ---------------------------------------------------------------------------------
const street = defineScene({
  id: "street",
  main: image("art/stage_street.png", { wide: true }),
  backdrop: "#0a0814",
  actors: {
    david: sprite("art/act_david.png", { w: 32, h: 32, frames: 8 }),
    cop: sprite("art/en_cop.png", { w: 32, h: 32, frames: 4 }),
    drone: sprite("art/en_drone.png", { w: 32, h: 32, frames: 4 }),
    maine: sprite("art/wd_maine.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
  },
  action: {
    player: { actor: "david", hp: 8, sande: 40 },
    ground: 140,
    gates: [
      { x: 130, wave: 1 },
      { x: 230, wave: 2 },
      { x: 310, wave: 3 },
    ],
    spawns: [
      { type: "gunner", actor: "cop", x: 170, hp: 3, wave: 1 },
      { type: "gunner", actor: "cop", x: 200, hp: 3, wave: 1 },
      { type: "drone", actor: "drone", x: 260, hp: 2, wave: 2 },
      { type: "gunner", actor: "cop", x: 300, hp: 3, wave: 2 },
      { type: "gunner", actor: "cop", x: 340, hp: 3, wave: 3 },
      { type: "drone", actor: "drone", x: 360, hp: 2, wave: 3 },
    ],
    exit: "clear",
  },
  play: cue(function* () {
    yield fadeIn(30);
    yield caption("chip", "日本町·撤离路线");
    yield caption("sub", "田中的任务是个陷阱。\n公司在收网, 全员突围!");
    yield waitA();
    yield captionClear("all");
    yield meterShow(0, "hp", 160, 4, 8);
    yield meterShow(1, "sande", 160, 14, 40);
    yield action();
    yield meterHide(0);
    yield meterHide(1);
    yield captionClear("all");
    // Maine's last stand
    yield show("maine", 330, 104);
    yield walkTo("david", 280, 40);
    yield dialog("MAINE", "多里奥没跟上来。\n我回去找她。");
    yield dialog("DAVID", "缅因, 你的义体已经——");
    yield dialog("MAINE", "我知道。我早就知道。");
    yield dialog("MAINE", "听着, 大卫。这里是\n我的终点站——但不是你的。");
    yield dialog("MAINE", "继续跑。别停下来。");
    yield captionClear("all");
    yield wait(30);
    yield fadeOut(20, "white");
    yield shake(6, 60);
    yield sfx("whoosh");
    yield wait(60);
    yield caption("card", "那天之后, 没有人再见过\n缅因和多里奥。");
    yield wait(100);
    yield captionClear("all");
    yield fadeIn(1);
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S9 — LAB: six months later; the cyberskeleton raid (Ep 7-9 compressed)
// ---------------------------------------------------------------------------------
const lab = defineScene({
  id: "lab",
  main: image("art/stage_lab.png", { wide: true }),
  backdrop: "#0a0e16",
  actors: {
    david: sprite("art/act_david.png", { w: 32, h: 32, frames: 8 }),
    guard: sprite("art/en_guard.png", { w: 32, h: 32, frames: 4 }),
    turret: sprite("art/en_turret.png", { w: 32, h: 32, frames: 4 }),
    drone: sprite("art/en_drone.png", { w: 32, h: 32, frames: 4 }),
    skel: sprite("art/spr_cyberskel.png", { w: 32, h: 32 }),
  },
  action: {
    player: { actor: "david", hp: 10, sande: 48 },
    ground: 140,
    gates: [
      { x: 120, wave: 1 },
      { x: 260, wave: 2 },
    ],
    spawns: [
      { type: "gunner", actor: "guard", x: 160, hp: 3, wave: 1 },
      { type: "turret", actor: "turret", x: 200, hp: 4, wave: 1 },
      { type: "drone", actor: "drone", x: 180, hp: 2, wave: 1 },
      { type: "gunner", actor: "guard", x: 300, hp: 3, wave: 2 },
      { type: "gunner", actor: "guard", x: 330, hp: 3, wave: 2 },
      { type: "turret", actor: "turret", x: 364, hp: 4, wave: 2 },
      { type: "drone", actor: "drone", x: 344, hp: 2, wave: 2 },
    ],
    exit: "clear",
  },
  play: cue(function* () {
    yield fadeIn(30);
    yield caption("chip", "六个月后");
    yield caption("sub", "大卫成了传说, 也成了实验品\n候选。基维出卖了所有人。");
    yield waitA();
    yield caption("sub", "露西被抓走了。通讯里\n她的声音说: 想活下来——");
    yield waitA();
    yield captionClear("all");
    yield show("skel", 40, 104);
    yield dialog("???", "「想活下来, 就穿上\n机械骨骼。」");
    yield dialog("DAVID", "…那不是露西。\n但没有别的路了。");
    yield hide("skel");
    yield mosaicTo(8, 20);
    yield shake(5, 40);
    yield sfx("star");
    yield mosaicTo(0, 20);
    yield caption("sub", "机械骨骼·同步率异常\n免疫抑制剂: 九倍剂量");
    yield waitA();
    yield captionClear("all");
    yield meterShow(0, "hp", 160, 4, 10);
    yield meterShow(1, "sande", 160, 14, 48);
    yield action();
    yield meterHide(0);
    yield meterHide(1);
    yield captionClear("all");
    yield rasterWave("main", 2);
    yield caption("sub", "视野边缘开始闪烁。\n他知道这意味着什么。");
    yield waitA();
    yield rasterOff();
    yield captionClear("all");
    yield fadeOut(40);
  }),
});

// ---------------------------------------------------------------------------------
// S10 — TOWER: Adam Smasher (Ep 10). The fight ends scripted: canon says you lose.
// ---------------------------------------------------------------------------------
const tower = defineScene({
  id: "tower",
  main: image("art/stage_pad.png", { wide: true }),
  backdrop: "#0c0a14",
  actors: {
    david: sprite("art/act_david.png", { w: 32, h: 32, frames: 8 }),
    smasher: sprite("art/boss_smasher.png", { w: 64, h: 64, frames: 4 }),
    cop: sprite("art/en_cop.png", { w: 32, h: 32, frames: 4 }),
    lucy: sprite("art/wd_lucy.png", { w: 32, h: 32, frames: 12, walkFpd: 4 }),
    rebecca: sprite("art/wd_rebecca.png", { w: 32, h: 32, frames: 3, walkFpd: 1 }),
  },
  action: {
    player: { actor: "david", hp: 10, sande: 48 },
    ground: 140,
    gates: [{ x: 120, wave: 1 }],
    spawns: [
      { type: "gunner", actor: "cop", x: 150, hp: 3, wave: 1 },
      { type: "gunner", actor: "cop", x: 180, hp: 3, wave: 1 },
      { type: "boss", actor: "smasher", x: 300, hp: 40 },
    ],
    bossPhaseHp: 20,
    exit: "clear",
  },
  play: cue(function* () {
    yield fadeIn(40);
    yield caption("chip", "荒坂塔·停机坪");
    yield show("rebecca", 60, 104);
    yield dialog("REBECCA", "我数了一下, 一整个\n公司想弄死我们。");
    yield dialog("REBECCA", "…正合我意! 上吧!");
    yield hide("rebecca");
    yield dialog("DAVID", "妈, 你看。\n我到顶层了。");
    yield captionClear("all");
    yield meterShow(0, "hp", 160, 4, 10);
    yield meterShow(1, "sande", 160, 14, 48);
    const endfight = yield action();
    yield setVar("endfight", endfight);
    yield meterHide(0);
    yield meterHide(1);
    yield captionClear("all");
    // scripted: the Sandevistan burns out; canon takes over
    yield rasterWave("main", 3);
    yield mosaicTo(6, 30);
    yield shake(5, 50);
    yield dialog("SMASHER", "初级货色的义体。\n你差点让我觉得有趣。");
    yield caption("sub", "斯安威斯坦过热·熔断\n免疫系统: 崩溃");
    yield waitA();
    yield caption("sub", "瑞贝卡的枪声停了。\n停机坪安静得可怕。");
    yield waitA();
    yield captionClear("all");
    yield mosaicTo(0, 20);
    yield rasterOff();
    yield show("lucy", 40, 104);
    yield walkTo("lucy", 96, 50);
    yield dialog("LUCY", "大卫! 飞行器就在这,\n我们还能——");
    yield dialog("DAVID", "露西。你从来不需要我救。");
    yield dialog("DAVID", "我只是想看你\n站在月球上。");
    yield dialog("DAVID", "对不起。这次\n不能陪你去了。");
    yield dialog("LUCY", "我要的从来不是月球。\n我要的是你活着!");
    yield dialog("DAVID", "……月球见, 露西。");
    yield captionClear("all");
    yield wait(40);
    yield fadeOut(90, "white");
    yield wait(30);
  }),
});

// ---------------------------------------------------------------------------------
// S11 — moon: epilogue + credits ★ song reprise
// ---------------------------------------------------------------------------------
const moon = defineScene({
  id: "moon",
  main: image("art/bg_moon.png"),
  backdrop: "#000008",
  letterbox: 20,
  actors: {
    lucy: sprite("art/spr_lucy_suit.png", { w: 32, h: 32 }),
  },
  play: cue(function* () {
    yield music("stay45");
    yield fadeIn(90, "white");
    yield caption("chip", "月球·静海");
    yield show("lucy", -20, 112);
    yield walkTo("lucy", 100, 160);
    yield wait(60);
    yield caption("sub", "银色的地面。刺眼的太阳。\n和脑舞里一模一样。");
    yield waitA();
    yield dialog("LUCY", "看到了吗, 大卫。\n我不是一个人来的。");
    yield wait(60);
    yield caption("sub", "「哇——你看!\n我能感觉到太阳!」");
    yield waitA();
    yield captionClear("all");
    yield wait(60);
    // credits
    yield caption("card", "SEE YOU ON THE MOON\n《月球见》");
    yield wait(140);
    yield caption("card", "CYBERPUNK: EDGERUNNERS\n同人致敬·非官方作品");
    yield wait(140);
    yield caption("card", "原作: Trigger x CDPR\n剧本为原创同人写作");
    yield wait(140);
    yield caption("card", "插曲: I Really Want to\nStay at Your House");
    yield wait(140);
    yield caption("card", "曲: Rosa Walton\n8-bit 翻奏仅私人使用");
    yield wait(140);
    yield caption("card", "引擎: @pocketjs/edge\nGBA 上见");
    yield wait(140);
    yield captionClear("all");
    if (yield hasFlag("promise")) {
      yield caption("card", "他遵守了约定。");
      yield wait(160);
    }
    yield captionClear("all");
    yield fadeOut(80);
  }),
});

// ---------------------------------------------------------------------------------
export default defineFilm({
  title: "SEE YOU ON THE MOON",
  scenes: [title, apartment, clinic, alley, train, hideout, warehouse, rooftop, street, lab, tower, moon],
  music: {
    stay: track("music/stay-gxscc.raw", { loop: true }),
    stay45: track("music/stay-raxlen-45s.raw", { loop: true }),
  },
});
