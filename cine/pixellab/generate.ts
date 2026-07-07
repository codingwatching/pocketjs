// cine/pixellab/generate.ts — generate every film asset through PixelLab and
// cache it under film/art/ (committed, so builds never re-bill).
//   bun pixellab/generate.ts [--force] [--only name[,name]]
//
// Prompts are deliberately franchise-neutral: scenes and emblems are described
// generically (a green gem shaped like a letter, a lightning gem, a sailing
// ship) — this is an original fan tribute, not asset cloning.

import { pixflux, balance, type PixfluxOpts } from "./client.ts";

const OUT = new URL("../film/art/", import.meta.url).pathname;

interface Spec extends Omit<PixfluxOpts, "width" | "height"> {
  name: string;
  w: number;
  h: number;
}

const PIXEL_STYLE = "clean 16-bit pixel art, limited palette, crisp pixels";

export const SPECS: Spec[] = [
  // --- backgrounds (main layers) ---------------------------------------------
  {
    name: "bg_room90s",
    w: 240,
    h: 160,
    description:
      `1990s Chinese apartment interior at dusk, side view: wooden desk with a beige retro computer and boxy CRT monitor on the right, bookshelf and posters on the wall, window with distant water-town rooftops on the left, warm lamp light, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42001,
  },
  {
    name: "bg_classroom",
    w: 240,
    h: 160,
    description:
      `early-2000s classroom interior, side view: rows of desks in silhouette at the bottom, a large bright projector screen glowing pale blue on the front wall, chalkboard edge, dark room lit by the screen, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42002,
  },
  {
    name: "bg_nyc",
    w: 384,
    h: 160,
    description:
      `rainy city night panorama, side view: on the left a lit bus stop with a bench, in the middle dark brownstone buildings with fire escapes, on the right a dorm room window glowing warm with a desk lamp and a small desk visible inside, wet street reflections, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42003,
  },
  {
    name: "bg_office",
    w: 240,
    h: 160,
    description:
      `modern tech office at night, side view: a long desk with two glowing monitors, one desk lamp on, big windows behind showing a city skyline with tiny lights, empty chairs, blue-dark ambience with one warm pool of light, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42004,
  },
  {
    name: "bg_kitchen",
    w: 240,
    h: 160,
    description:
      `small cozy apartment kitchen in the evening, side view: a wooden table with two chairs in the center, kettle on the stove, fridge with sticky notes, warm yellow light from a ceiling lamp, window with dark blue night outside, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42005,
  },
  {
    name: "bg_stage",
    w: 240,
    h: 160,
    description:
      `dark conference stage, side view: a wide glowing presentation screen high on the back wall, a small podium on the right, two spotlight beams, front rows of audience heads as dark silhouettes at the bottom, teal and dark blue palette, ${PIXEL_STYLE}`,
    negative: "text, letters, faces",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42006,
  },
  {
    name: "bg_deskstorm",
    w: 240,
    h: 160,
    description:
      `home office at night during a thunderstorm, side view: a desk with a bright monitor in a dark room, a large window behind it streaked with rain, storm clouds and a flash of lightning outside, papers on the floor, a small toy on the shelf, cold blue palette with one bright screen, ${PIXEL_STYLE}`,
    negative: "people, text, letters",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42007,
  },
  {
    name: "bg_harbor",
    w: 384,
    h: 160,
    description:
      `calm sea at dawn panorama, side view: open water with gentle waves in the foreground, a distant harbor on the right horizon with orange cranes and warehouses catching the sunrise, soft pink and gold sky low over the waterline, ${PIXEL_STYLE}`,
    negative: "people, text, letters, boats",
    view: "side",
    shading: "medium shading",
    detail: "medium detail",
    seed: 42008,
  },
  {
    name: "bg_hill",
    w: 240,
    h: 160,
    description:
      `grassy hill at sunset, side view: the hill crest as a dark green silhouette across the lower third, one small tree on the left, a tiny city skyline far below on the right edge, huge warm gradient sky taking most of the frame, a few first stars, ${PIXEL_STYLE}`,
    negative: "people, text, letters, sun disc",
    view: "side",
    shading: "basic shading",
    detail: "medium detail",
    seed: 42009,
  },
  // --- far / parallax strips (transparent background) ---------------------------
  {
    name: "far_skyline",
    w: 240,
    h: 80,
    description:
      `distant city skyline silhouette at dusk, flat dark navy building shapes with a few tiny lit windows, skyline occupies the bottom half, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    shading: "flat shading",
    detail: "low detail",
    seed: 42010,
  },
  {
    name: "far_waves",
    w: 240,
    h: 48,
    description:
      `stylized dark teal sea wave strip, repeating rolling wave crests with lighter foam tips, seen from the side, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    shading: "flat shading",
    detail: "low detail",
    seed: 42011,
  },
  // --- sprites (transparent, side view) ---------------------------------------------
  {
    name: "spr_kid",
    w: 32,
    h: 32,
    description:
      `one small boy in a 1990s red jacket and blue pants, black hair, standing side profile facing right, full body, single character, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple characters, text",
    noBackground: true,
    view: "side",
    direction: "east",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42020,
  },
  {
    name: "spr_evan",
    w: 32,
    h: 32,
    description:
      `one young man with short black hair and glasses, dark green hoodie, jeans, standing side profile facing right, full body, single character, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple characters, text",
    noBackground: true,
    view: "side",
    direction: "east",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42021,
  },
  {
    name: "spr_dad",
    w: 32,
    h: 32,
    description:
      `one middle-aged man in a plain 1990s shirt and trousers, short black hair, standing side profile facing left, full body, single character, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple characters, text",
    noBackground: true,
    view: "side",
    direction: "west",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42022,
  },
  {
    name: "spr_wife",
    w: 32,
    h: 32,
    description:
      `full body tiny sprite of one young woman standing, head to toe visible, shoulder-length black hair, warm beige cardigan and long skirt, side profile facing left, feet at the bottom, single small character, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple characters, text",
    noBackground: true,
    view: "side",
    direction: "west",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 52023,
  },
  {
    name: "spr_child",
    w: 32,
    h: 32,
    description:
      `one toddler in yellow overalls, tiny, standing side profile facing right, full body, single character, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple characters, text",
    noBackground: true,
    view: "side",
    direction: "east",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42024,
  },
  {
    name: "spr_fish",
    w: 32,
    h: 32,
    description:
      `one tropical fish swimming right, orange and white, simple cute shape, transparent background, ${PIXEL_STYLE}`,
    negative: "multiple fish, text",
    noBackground: true,
    view: "side",
    direction: "east",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42025,
  },
  {
    name: "spr_letter",
    w: 32,
    h: 32,
    description: `one white paper envelope with a red and blue airmail border, slightly tilted, transparent background, ${PIXEL_STYLE}`,
    negative: "text, letters",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42026,
  },
  {
    name: "spr_vgem",
    w: 32,
    h: 32,
    description: `one faceted emerald green gemstone shaped like a sharp letter V chevron, dark slate blue rim, glinting, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    outline: "single color outline",
    shading: "basic shading",
    detail: "medium detail",
    seed: 42027,
  },
  {
    name: "spr_bolt",
    w: 32,
    h: 32,
    description: `one stylized lightning bolt gem, indigo purple outer glow with a golden amber core, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    outline: "single color outline",
    shading: "basic shading",
    detail: "medium detail",
    seed: 42028,
  },
  {
    name: "spr_ship",
    w: 64,
    h: 32,
    description: `one small dark sailing ship with a bright green triangular sail, side profile facing right, floating, transparent background, ${PIXEL_STYLE}`,
    negative: "text, water",
    noBackground: true,
    view: "side",
    direction: "east",
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42029,
  },
  {
    name: "spr_flag",
    w: 32,
    h: 32,
    description: `one rectangular dark navy fabric flag waving to the right from a tall vertical wooden flagpole on the left edge, cloth rippling in the wind, small white circle emblem on the cloth, transparent background, ${PIXEL_STYLE}`,
    negative: "text, skull, arrow, sign",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 52030,
  },
  {
    name: "spr_book",
    w: 32,
    h: 32,
    description: `one thick closed textbook with a grey-green cover, side view, transparent background, ${PIXEL_STYLE}`,
    negative: "text, letters",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    detail: "low detail",
    seed: 42031,
  },
  {
    name: "spr_orb",
    w: 32,
    h: 32,
    description: `one small glowing round orb, soft cyan core with white highlight, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    outline: "lineless",
    shading: "basic shading",
    detail: "low detail",
    seed: 42032,
  },
  {
    name: "spr_star",
    w: 32,
    h: 32,
    description: `one small four-pointed sparkle star, bright green-white, transparent background, ${PIXEL_STYLE}`,
    negative: "text",
    noBackground: true,
    outline: "lineless",
    shading: "flat shading",
    detail: "low detail",
    seed: 42033,
  },
  {
    name: "spr_drawing",
    w: 32,
    h: 32,
    description: `a child's simple drawing of a house and a sun rendered in bright pixels on a dark blue computer screen rectangle, transparent background, ${PIXEL_STYLE}`,
    negative: "text, letters",
    noBackground: true,
    outline: "lineless",
    shading: "flat shading",
    detail: "low detail",
    seed: 42034,
  },
];

if (import.meta.main) {
  const force = process.argv.includes("--force");
  const onlyArg = process.argv.indexOf("--only");
  const only = onlyArg >= 0 ? new Set(process.argv[onlyArg + 1].split(",")) : null;

  console.log("pixellab balance:", await balance());
  const manifest: Record<string, { prompt: string; seed?: number; w: number; h: number }> = {};
  const manifestPath = OUT + "manifest.json";
  try {
    Object.assign(manifest, JSON.parse(await Bun.file(manifestPath).text()));
  } catch {}

  for (const spec of SPECS) {
    const { name, w, h, ...opts } = spec;
    if (only && !only.has(name)) continue;
    const out = OUT + name + ".png";
    if (!force && (await Bun.file(out).exists())) {
      console.log(`  skip ${name} (cached)`);
      continue;
    }
    process.stdout.write(`  gen ${name} ${w}x${h}... `);
    const png = await pixflux({ ...opts, width: w, height: h });
    await Bun.write(out, png);
    manifest[name] = { prompt: spec.description, seed: spec.seed, w, h };
    await Bun.write(manifestPath, JSON.stringify(manifest, null, 2));
    console.log(`ok (${png.length}B)`);
  }
  console.log("done. balance:", await balance());
}
