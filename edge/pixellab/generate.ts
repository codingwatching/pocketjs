// edge/pixellab/generate.ts — generate every game asset through PixelLab and
// cache it under game/art/ (committed, so builds never re-bill).
//   bun pixellab/generate.ts [--force] [--only name[,name]]
//
// Then:
//   bun pixellab/walkers.ts       top-down walker sheets (world scenes)
//   bun pixellab/actionsheets.ts  side-view action sheets (player/enemies/boss)
//
// Prompts are descriptive, not trademarked: no logos, no franchise names, no
// trade dress. This is an unaffiliated fan tribute; the cast is described in
// plain visual language.

import { pixflux, balance, type PixfluxOpts } from "./client.ts";

const OUT = new URL("../game/art/", import.meta.url).pathname;

interface Spec extends Omit<PixfluxOpts, "width" | "height"> {
  name: string;
  w: number;
  h: number;
}

const MAP_STYLE =
  "seen directly from above like a Game Boy RPG town map, clean 16x16 tile alignment, top-down 2D RPG interior map";
const STAGE_STYLE =
  "side view 2D action platformer stage, flat ground along the bottom, detailed pixel art game background, neon-soaked cyberpunk megacity at night";
const SPRITE = "tiny full body pixel art sprite, head to toe visible, Game Boy Advance sprite";

// --- the cast, in plain visual language (per the research dossier) ----------------
export const DAVID =
  "lean latino teen, dark brown quiff hair with shaved sides, open yellow paramedic bomber jacket with green emblem, bare chest with gold chain necklaces, gray pants, white sneakers";
export const LUCY =
  "pale slender young woman, white asymmetric bob haircut with pastel rainbow tips, cropped white jacket over black high-collar bodysuit with red accents, white shorts, gray stockings, black boots";
export const REBECCA =
  "very short young woman with stark white skin, long turquoise-green twin tails, oversized black high-collar jacket with green accents, red and blue chunky robot arms, huge green and pink shotgun";
export const MAINE =
  "huge towering muscular black man with short blond hair, gunmetal chrome cyborg arms, olive tactical vest";
export const DORIO =
  "very tall muscular woman with short blonde hair, open dark jacket, black shorts, bare chrome metal legs";
export const PILAR =
  "tall lanky man with a black mohawk, golden metal robot hands, goggle rig framing his jaw, sleeveless black vest";
export const FALCO =
  "older man with semi-long dark brown hair parted in the middle, thick curled mustache, brown western waistcoat over white shirt, one silver chrome arm";
export const KIWI =
  "very tall slender woman, light blonde bob haircut, glossy red mask plate covering her lower face, long red coat, cyan web tattoos";
export const GLORIA = "tired woman in her forties, vivid red hair tied back, orange and white paramedic uniform";
export const THUG = "street gang punk, tall red mohawk, spiked leather jacket, one chrome cyber arm, knuckle wraps";
export const GUARD = "corporate security soldier, dark navy armored uniform, mirrored visor helmet, compact rifle";
export const COP = "heavily armored tactical trooper, matte black exo armor, riot helmet with red lenses";
export const DRONE = "small hovering security drone, twin ducted rotors, single red sensor eye, gunmetal shell";
export const TURRET = "compact automated sentry gun turret on a squat tripod, twin barrels, amber sensor";
export const SMASHER =
  "towering heavy full-metal chrome cyborg, skull-like silver faceplate, single red eye, enormous shoulders, gunmetal and black armor plating, arm cannon";

export const SPECS: Spec[] = [
  // --- cine backgrounds (240x160) ----------------------------------------------
  {
    name: "bg_apartment",
    w: 240,
    h: 160,
    description:
      "small cramped one-room megacity apartment at night, side view interior, bunk bed, tiny kitchen corner, laundry on a line, one big window filling the back wall with dense neon city towers and holographic ads outside, warm lamp inside, detailed pixel art game background",
    negative: "people, person, text, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71001,
  },
  {
    name: "bg_rooftop",
    w: 240,
    h: 160,
    description:
      "rooftop of a megacity skyscraper at night seen from the roof surface, low concrete ledge in the foreground, vast glittering neon city sprawling far below, huge full moon dominating the sky, thin clouds, cold blue night with pink neon glow on the horizon, detailed pixel art game background, wide shot",
    negative: "people, person, text, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71002,
  },
  {
    name: "bg_moon",
    w: 240,
    h: 160,
    description:
      "gray lunar surface with fine dust and small craters, black star-filled sky, the blue Earth hanging large and luminous above the horizon, a small domed lunar base far in the distance, quiet and vast, detailed pixel art game background",
    negative: "people, person, astronaut, text, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71003,
  },
  {
    name: "bg_clinic",
    w: 240,
    h: 160,
    description:
      "back-alley cybernetics clinic interior at night, side view, reclined operating chair under a ring of surgical lights, racks of chrome body parts and cables on the walls, monitors with green readouts, grimy floor, moody teal and magenta lighting, detailed pixel art game background",
    negative: "people, person, text, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71004,
  },
  // --- action stages (512x160 / 384x160, side view) ------------------------------
  {
    name: "stage_alley",
    w: 384,
    h: 160,
    description: `${STAGE_STYLE}, narrow back alley between towering buildings, wet asphalt reflecting pink and cyan neon signs, dumpsters, tangled cables overhead, steam vents, fire escapes, distant towers glowing between the buildings`,
    negative: "people, person, text, letters, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71010,
  },
  {
    name: "stage_warehouse",
    w: 384,
    h: 160,
    description: `${STAGE_STYLE}, corporate cargo warehouse interior at night, stacked shipping containers and crates, yellow gantry crane overhead, hazard stripes on the concrete floor, cold blue industrial lights, chain-link fencing, loading dock doors`,
    negative: "people, person, text, letters, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71011,
  },
  {
    name: "stage_street",
    w: 384,
    h: 160,
    description: `${STAGE_STYLE}, wide downtown avenue at night, holographic billboards and neon storefronts, parked futuristic cars, overturned barricades, palm trees under sodium lights, elevated rail line crossing above, smoke in the distance`,
    negative: "people, person, text, letters, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71012,
  },
  {
    name: "stage_lab",
    w: 384,
    h: 160,
    description: `${STAGE_STYLE}, sterile corporate laboratory corridor, white and gray panels, glass walls into server rooms glowing blue, robotic arms idle at workstations, red warning lights strobing, cable trays along the ceiling, polished floor`,
    negative: "people, person, text, letters, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71013,
  },
  {
    name: "stage_pad",
    w: 384,
    h: 160,
    description: `${STAGE_STYLE}, flat rooftop landing pad floor running continuously along the entire bottom edge of the image, helipad deck plating with painted markings as the walkable ground, low guard rails behind, antenna masts and vent stacks, distant skyscraper tops and thin clouds far below the railing, cold dawn sky in orange and steel blue`,
    negative: "people, person, aircraft, pit, gap, chasm, text, letters, logo, watermark",
    detail: "highly detailed",
    shading: "detailed shading",
    seed: 71114,
  },
  // --- world maps (top-down) -------------------------------------------------------
  {
    name: "map_train",
    w: 320,
    h: 160,
    description: `top-down 2D RPG map of a futuristic metro train car interior, ${MAP_STYLE}, sleek silver car walls forming the border, rows of paired seats along both sides leaving a walkable center aisle, sliding doors at both ends, luggage racks, small floor lights, teal and gray palette`,
    negative: "people, person, text, side view, perspective, isometric",
    view: "high top-down",
    detail: "highly detailed",
    shading: "basic shading",
    seed: 71020,
  },
  {
    name: "map_hideout",
    w: 320,
    h: 240,
    description: `top-down 2D RPG map of a dive bar back room turned mercenary hideout, ${MAP_STYLE}, dark brick walls forming the border, a long bar counter with stools along the top wall, a pool table in the middle, a weapons workbench, a torn couch and low table, neon sign glow spilling across the floor, crates in a corner`,
    negative: "people, person, text, side view, perspective, isometric",
    view: "high top-down",
    detail: "highly detailed",
    shading: "basic shading",
    seed: 71021,
  },
  // --- top-down walker stills (32x32, assembled by walkers.ts) ---------------------
  ...(
    [
      ["david", DAVID, 71030, ["s", "n", "e"]],
      ["lucy", LUCY, 71033, ["s", "n", "e"]],
      ["maine", MAINE, 71036, ["s", "n", "e"]],
      ["rebecca", REBECCA, 71039, ["s", "n", "e"]],
      ["falco", FALCO, 71042, ["s"]],
      ["kiwi", KIWI, 71043, ["s"]],
      ["gloria", GLORIA, 71044, ["s"]],
      ["dorio", DORIO, 71045, ["s"]],
      ["pilar", PILAR, 71046, ["s"]],
    ] as [string, string, number, string[]][]
  ).flatMap(([who, look, seed, dirs]) =>
    dirs.map((sfx, i) => ({
      name: `walk_${who}_${sfx}`,
      w: 32,
      h: 32,
      description: `${SPRITE} of ${look}, standing`,
      negative: "portrait, bust, cropped, text",
      noBackground: true,
      outline: "single color black outline" as const,
      shading: "basic shading" as const,
      view: "low top-down" as const,
      direction: (sfx === "s" ? "south" : sfx === "n" ? "north" : "east") as PixfluxOpts["direction"],
      seed: seed + i,
    })),
  ),
  // --- side-view action stills (assembled by actionsheets.ts) ----------------------
  // player poses (32x32): idle / jump / shoot / hurt — run frames come from
  // animate-with-text off the idle still.
  {
    name: "act_david_idle",
    w: 32,
    h: 32,
    description: `${SPRITE} of ${DAVID}, side view facing right, standing relaxed holding a compact pistol low`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71050,
  },
  {
    name: "act_david_jump",
    w: 32,
    h: 32,
    description: `${SPRITE} of ${DAVID}, side view facing right, mid-air jump with knees tucked, dynamic`,
    negative: "portrait, bust, cropped, text, front view, standing",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71051,
  },
  {
    name: "act_david_shoot",
    w: 32,
    h: 32,
    description: `${SPRITE} of ${DAVID}, side view facing right, two-handed pistol aimed straight forward, muzzle flash`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71052,
  },
  {
    name: "act_david_hurt",
    w: 32,
    h: 32,
    description: `${SPRITE} of ${DAVID}, side view facing right, flinching backward in pain, arms raised`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71053,
  },
  // enemies: idle + attack stills per type (32x32); walk frames animated
  ...(
    [
      ["thug", THUG, "lunging forward throwing a punch with the chrome arm", 71060],
      ["guard", GUARD, "kneeling and firing the rifle, small muzzle flash", 71062],
      ["cop", COP, "braced and firing a heavy rifle, small muzzle flash", 71064],
      ["drone", DRONE, "tilted forward, red eye flaring, firing a shot downward", 71066],
      ["turret", TURRET, "barrels recoiling with a small muzzle flash", 71068],
    ] as [string, string, string, number][]
  ).flatMap(([who, look, attack, seed]) => [
    {
      name: `en_${who}_idle`,
      w: 32,
      h: 32,
      description: `${SPRITE} of ${look}, side view facing right, standing`,
      negative: "portrait, bust, cropped, text, front view",
      noBackground: true,
      outline: "single color black outline" as const,
      shading: "basic shading" as const,
      direction: "east" as const,
      seed,
    },
    {
      name: `en_${who}_attack`,
      w: 32,
      h: 32,
      description: `${SPRITE} of ${look}, side view facing right, ${attack}`,
      negative: "portrait, bust, cropped, text, front view",
      noBackground: true,
      outline: "single color black outline" as const,
      shading: "basic shading" as const,
      direction: "east" as const,
      seed: seed + 1,
    },
  ]),
  // boss (64x64): idle + attack stills; walk animated
  {
    name: "boss_smasher_idle",
    w: 64,
    h: 64,
    description: `${SPRITE} of ${SMASHER}, side view facing right, standing menacingly, full body head to toe`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "detailed shading",
    direction: "east",
    seed: 71070,
  },
  {
    name: "boss_smasher_attack",
    w: 64,
    h: 64,
    description: `${SPRITE} of ${SMASHER}, side view facing right, arm cannon raised and firing, muzzle flash, full body head to toe`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "detailed shading",
    direction: "east",
    seed: 71071,
  },
  // cutscene sprites
  {
    name: "spr_duo_ledge",
    w: 64,
    h: 32,
    description:
      "tiny pixel art sprite of two people sitting side by side on a concrete rooftop ledge seen from behind, on the left a lean young man with undercut dark hair and yellow-black jacket, on the right a slender woman with silver-white bob hair and white jacket, feet dangling, night",
    negative: "portrait, bust, cropped, text, front view, faces",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    seed: 71080,
  },
  {
    name: "spr_lucy_suit",
    w: 32,
    h: 32,
    description: `${SPRITE} of a slender woman in a sleek white and red spacesuit without helmet, silver-white bob hair with pastel tips, walking, side view facing right`,
    negative: "portrait, bust, cropped, text",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71081,
  },
  {
    name: "spr_gloria_fallen",
    w: 32,
    h: 32,
    description:
      "tiny pixel art sprite of a woman in an orange and white paramedic uniform lying injured on the ground, side view, full body",
    negative: "portrait, bust, cropped, text, standing",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    seed: 71082,
  },
  {
    name: "spr_cyberskel",
    w: 32,
    h: 32,
    description: `${SPRITE} of a lean young man with undercut dark hair fused into a skeletal chrome exoskeleton frame, glowing cyan joints, side view facing right, standing`,
    negative: "portrait, bust, cropped, text, front view",
    noBackground: true,
    outline: "single color black outline",
    shading: "basic shading",
    direction: "east",
    seed: 71083,
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
