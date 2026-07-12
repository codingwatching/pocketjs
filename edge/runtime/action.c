/* edge/runtime/action.c — the side-scrolling run-and-gun core.
 *
 * OP_ACTION blocks in WAITING_ACTION on the current scene's EdgeStage. The
 * player actor keeps living in its Spr slot (so cutscene ops can pose the same
 * sprite before/after); this core drives that slot's x/y/frame directly and
 * owns dedicated OAM ranges for enemies, bullets and afterimages.
 *
 * Controls: D-pad run, A jump, B shoot (auto-melee inside ACT_MELEE_RANGE),
 * hold R = Sandevistan while the gauge lasts: the world updates 1 of
 * C_ACT_SLOW_DIV frames, the player every frame, the BG palette swaps to the
 * compiler's tinted copy and ghost poses trail the player.
 *
 * The stage never fails the film: at 0 HP the player respawns at the last
 * gate passed (deaths_var++). Result: C_ACT_CLEARED, or C_ACT_BOSS_PHASE the
 * moment the boss' HP crosses its scripted threshold. */
#include "edge.h"

Act act;

#define SPAWN_AHEAD 268 /* activate spawns this close to the player */
#define CONTACT_W 14    /* player/enemy overlap half-width */
#define BULLET_KILL_MARGIN 40

static const EdgeProto *eproto(const ActEnemy *e) {
  return &g.sc->protos[e->proto];
}

static void hud_sync(void) {
  const EdgeStage *st = act.st;
  g.vars[st->kills_var] = (s16)(g.vars[st->kills_var]); /* interned; kills updated on kill */
}

static void player_frame(u8 f) {
  g.spr[act.pl_slot].mode = 0;
  g.spr[act.pl_slot].frame = f;
}

/* --- spawning ------------------------------------------------------------------- */
static void enemy_activate(u8 spawn_idx) {
  const EdgeStage *st = act.st;
  const EdgeSpawn *sp = &st->spawns[spawn_idx];
  int i;
  for (i = 0; i < C_MAX_ENEMIES; i++) {
    ActEnemy *e = &act.en[i];
    if (e->active) continue;
    e->active = 1;
    e->spawn_idx = spawn_idx;
    e->behavior = sp->behavior;
    e->proto = sp->proto;
    e->hp = sp->hp;
    e->wave = sp->wave;
    e->x = sp->x;
    e->y = st->ground_y;
    /* drones hover so a straight shot (muzzle at y-14) grazes them: a 16px
     * drone's bottom near ground-18 puts its body across the bullet line */
    e->home_y = (s16)(st->ground_y - 18);
    if (sp->behavior == C_EB_DRONE) e->y = e->home_y;
    e->vx_q4 = 0;
    e->vy_q4 = 0;
    e->timer = (u8)(g.rng & 31);
    e->tele = 0;
    e->face = (u8)(sp->x > act.x);
    e->anim = 0;
    act.alive++;
    if (sp->behavior == C_EB_BOSS) act.boss_hp = sp->hp;
    return;
  }
}

static void bullet_fire(s16 x, s16 y, s16 vx_q4, s16 vy_q4, u8 from_enemy, u8 tile) {
  int i;
  for (i = 0; i < C_MAX_BULLETS; i++) {
    ActBullet *b = &act.bl[i];
    if (b->active) continue;
    b->active = 1;
    b->from_enemy = from_enemy;
    b->tile = tile;
    b->x = x;
    b->y = y;
    b->vx_q4 = vx_q4;
    b->vy_q4 = vy_q4;
    return;
  }
}

static void aim_at_player(const ActEnemy *e, s16 speed_q4, s16 *vx, s16 *vy) {
  s16 dx = (s16)(act.x - e->x);
  s16 dy = (s16)((act.y - 14) - e->y);
  s16 ax = dx < 0 ? (s16)-dx : dx;
  *vx = dx < 0 ? (s16)-speed_q4 : speed_q4;
  /* shallow vertical lead, quantized so shots stay dodgeable */
  if (dy > 24 && ax > 32) *vy = (s16)(speed_q4 / 2);
  else if (dy < -24 && ax > 32) *vy = (s16)(-speed_q4 / 2);
  else *vy = 0;
}

/* --- stage entry / respawn -------------------------------------------------------- */
void action_start(void) {
  const EdgeStage *st = g.sc->stage;
  Spr *s;
  int i;
  if (!st) { /* authoring error surfaced loudly: end the scene */
    vm_push(0);
    return;
  }
  act.active = 1;
  act.st = st;
  act.done = 0;
  act.pl_slot = 0; /* player actor is always slot 0 (first declared actor) */
  s = &g.spr[act.pl_slot];
  {
    /* if a cutscene already posed the actor, start where it stands */
    const EdgeProto *pp = &g.sc->protos[st->player_proto];
    act.x = (s->active && s->x > 0) ? (s16)(s->x + pp->w / 2) : 24;
  }
  s->active = 1;
  s->proto = st->player_proto;
  s->flags = 0;
  act.y = st->ground_y;
  act.x_q4 = (s32)act.x << 4;
  act.y_q4 = (s32)act.y << 4;
  act.vx_q4 = act.vy_q4 = 0;
  act.grounded = 1;
  act.face_left = 0;
  act.shoot_cd = 0;
  act.iframes = 0;
  act.hurt_timer = 0;
  act.sande_on = 0;
  act.sande_frames = 0;
  act.world_tick = 0;
  act.frame = 0;
  act.checkpoint_x = act.x;
  act.next_spawn = 0;
  act.alive = 0;
  act.spawned_dead = 0;
  act.boss_hp = 0;
  act.ghead = 0;
  for (i = 0; i < C_MAX_ENEMIES; i++) act.en[i].active = 0;
  for (i = 0; i < C_MAX_BULLETS; i++) act.bl[i].active = 0;
  for (i = 0; i < C_ACT_GHOSTS; i++) act.gx[i] = -320;
  g.vars[st->hp_var] = st->hp_max;
  g.vars[st->sande_var] = st->sande_max;
  hud_sync();
  g.waiting = WAITING_ACTION;
}

static void action_finish(s16 result) {
  int i;
  act.active = 0;
  act.sande_on = 0;
  /* restore the true palette + park the action OAM ranges */
  dma3_copy32(BG_PAL, g.sc->pal_bg, 512 / 4);
  for (i = 0; i < C_MAX_ENEMIES; i++) oam_shadow[C_OAM_ENEMY + i].attr0 = ATTR0_HIDE;
  for (i = 0; i < C_MAX_BULLETS; i++) oam_shadow[C_OAM_BULLET + i].attr0 = ATTR0_HIDE;
  for (i = 0; i < C_ACT_GHOSTS; i++) oam_shadow[C_OAM_GHOST + i].attr0 = ATTR0_HIDE;
  player_frame(C_PF_IDLE);
  vm_push(result);
  g.waiting = WAITING_RUN;
}

static void player_hurt(u8 dmg, s16 from_x) {
  const EdgeStage *st = act.st;
  if (act.iframes || act.done) return;
  g.vars[st->hp_var] -= dmg;
  act.iframes = C_ACT_IFRAMES;
  act.hurt_timer = 14;
  act.vx_q4 = (s16)(from_x > act.x ? -40 : 40); /* knockback */
  if (!act.grounded) act.vy_q4 = -24;
  sfx_play(C_SFX_WHOOSH);
  g.fx[TW_SHAKE] = 3;
  if (g.vars[st->hp_var] <= 0) {
    int i;
    g.vars[st->deaths_var]++;
    g.vars[st->hp_var] = st->hp_max;
    g.vars[st->sande_var] = st->sande_max;
    act.x = act.checkpoint_x;
    act.x_q4 = (s32)act.x << 4;
    act.y = st->ground_y;
    act.y_q4 = (s32)act.y << 4;
    act.vx_q4 = act.vy_q4 = 0;
    act.iframes = 120;
    /* clear the field so the respawn isn't an instant re-kill */
    for (i = 0; i < C_MAX_BULLETS; i++) act.bl[i].active = 0;
    g.fx[TW_SHAKE] = 5;
  }
}

static void enemy_hurt(ActEnemy *e, u8 dmg) {
  const EdgeStage *st = act.st;
  if (dmg >= e->hp) {
    e->hp = 0;
    e->active = 0;
    act.alive--;
    act.spawned_dead++;
    g.vars[st->kills_var]++;
    sfx_play(C_SFX_STAR);
    g.fx[TW_SHAKE] = 2;
    if (e->behavior == C_EB_BOSS) act.boss_hp = 0;
    return;
  }
  e->hp = (u8)(e->hp - dmg);
  if (e->behavior == C_EB_BOSS) {
    act.boss_hp = e->hp;
    if (st->boss_phase_hp && e->hp <= st->boss_phase_hp && !act.done) {
      act.done = 1; /* scripted takeover — story decides what "winning" means */
      action_finish(C_ACT_BOSS_PHASE);
    }
  }
  sfx_play(C_SFX_BLIP);
}

/* --- per-frame world (enemies + bullets), skipped on sandevistan frames ---------- */
static void enemies_update(void) {
  const EdgeStage *st = act.st;
  int i;
  for (i = 0; i < C_MAX_ENEMIES; i++) {
    ActEnemy *e = &act.en[i];
    const EdgeProto *p;
    s16 dx;
    if (!e->active) continue;
    p = eproto(e);
    dx = (s16)(act.x - e->x);
    e->face = (u8)(dx < 0);
    e->timer++;
    e->anim++;
    switch (e->behavior) {
      case C_EB_THUG:
        if (e->tele) { /* lunge in flight */
          e->tele--;
          e->x += e->face ? -3 : 3;
        } else if (dx > -C_ACT_MELEE_RANGE && dx < C_ACT_MELEE_RANGE) {
          if (e->timer > 40) {
            e->timer = 0;
            e->tele = 14; /* lunge */
          }
        } else {
          e->x += dx < 0 ? -1 : 1;
        }
        break;
      case C_EB_GUNNER:
        if (dx > -44 && dx < 44) e->x += dx < 0 ? 1 : -1; /* keep range */
        if (e->timer == 80 || e->timer == 88 || e->timer == 96) {
          s16 vx, vy;
          aim_at_player(e, C_ACT_EBULLET_VX, &vx, &vy);
          bullet_fire(e->x, (s16)(e->y - 14), vx, vy, 1, C_UIT_EBULLET);
          sfx_play(C_SFX_BLIP);
        }
        if (e->timer > 96) e->timer = 0;
        break;
      case C_EB_DRONE: {
        extern s8 sin8[256];
        e->y = (s16)(e->home_y + (sin8[(u8)(e->anim * 2)] >> 5));
        if (dx > 24) e->x += 1;
        else if (dx < -24) e->x -= 1;
        if (e->timer > 100) {
          s16 vx, vy;
          e->timer = 0;
          aim_at_player(e, C_ACT_EBULLET_VX, &vx, &vy);
          bullet_fire(e->x, (s16)(e->y + 4), vx, (s16)(vy + 10), 1, C_UIT_EBULLET);
          sfx_play(C_SFX_BLIP);
        }
        break;
      }
      case C_EB_TURRET:
        if (e->timer == 60 || e->timer == 75) {
          s16 vx, vy;
          aim_at_player(e, C_ACT_EBULLET_VX, &vx, &vy);
          bullet_fire(e->x, (s16)(e->y - 12), vx, vy, 1, C_UIT_EBULLET);
          sfx_play(C_SFX_BLIP);
        }
        if (e->timer > 75) e->timer = 0;
        break;
      case C_EB_BOSS: {
        /* cycle: stalk -> spread volley -> stalk -> telegraphed charge -> slam */
        u8 phase2 = (u8)(act.boss_hp * 2 < st->spawns[st->boss].hp); /* below half: faster */
        u8 cycle = phase2 ? 160 : 200;
        u8 t = (u8)(e->timer % cycle);
        if (e->tele) { /* charging */
          e->tele--;
          e->x += e->face ? -4 : 4;
          if (e->tele == 0) { /* slam shockwaves both ways along the ground */
            bullet_fire(e->x, (s16)(st->ground_y - 4), 40, 0, 1, C_UIT_SHOCK);
            bullet_fire(e->x, (s16)(st->ground_y - 4), -40, 0, 1, C_UIT_SHOCK);
            g.fx[TW_SHAKE] = 4;
            sfx_play(C_SFX_WHOOSH);
          }
        } else if (t == 70 || t == 78 || t == 86) { /* spread volley */
          s16 vx, vy;
          aim_at_player(e, (s16)(C_ACT_EBULLET_VX + (phase2 ? 8 : 0)), &vx, &vy);
          bullet_fire(e->x, (s16)(e->y - 40), vx, (s16)(vy - 8), 1, C_UIT_EBULLET);
          bullet_fire(e->x, (s16)(e->y - 40), vx, vy, 1, C_UIT_EBULLET);
          bullet_fire(e->x, (s16)(e->y - 40), vx, (s16)(vy + 8), 1, C_UIT_EBULLET);
          sfx_play(C_SFX_BLIP);
        } else if (t == (phase2 ? 120 : 150)) {
          e->tele = 26; /* charge (drawn as ATTACK frame = telegraph) */
          sfx_play(C_SFX_CONFIRM);
        } else {
          e->x += dx < 0 ? -1 : (dx > 0 ? 1 : 0);
          if (phase2 && (e->timer & 1)) e->x += dx < 0 ? -1 : 1;
        }
        break;
      }
    }
    /* contact damage */
    {
      s16 adx = (s16)(act.x - e->x);
      s16 ady = (s16)(act.y - e->y);
      s16 hw = (s16)(p->w / 2 + 2);
      if (adx > -hw && adx < hw && ady > -(s16)p->h && ady < 12)
        player_hurt(e->behavior == C_EB_BOSS ? 2 : 1, e->x);
    }
  }
}

static void bullets_update(void) {
  int i;
  for (i = 0; i < C_MAX_BULLETS; i++) {
    ActBullet *b = &act.bl[i];
    s16 sx;
    if (!b->active) continue;
    b->x += b->vx_q4 / 16;
    b->y += b->vy_q4 / 16;
    /* leftover q4 fraction is deliberately dropped: bullets are fast */
    sx = (s16)(b->x - g.fx[TW_CAM_X]);
    if (sx < -BULLET_KILL_MARGIN || sx > 240 + BULLET_KILL_MARGIN || b->y < -16 ||
        b->y > 176) {
      b->active = 0;
      continue;
    }
    if (b->from_enemy) {
      s16 dx = (s16)(b->x - act.x);
      s16 dy = (s16)(b->y - (act.y - 14));
      if (dx > -8 && dx < 8 && dy > -16 && dy < 16) {
        b->active = 0;
        player_hurt(1, b->x);
      }
    } else {
      int j;
      for (j = 0; j < C_MAX_ENEMIES; j++) {
        ActEnemy *e = &act.en[j];
        const EdgeProto *p;
        s16 dx, dy;
        if (!e->active) continue;
        p = eproto(e);
        dx = (s16)(b->x - e->x);
        dy = (s16)(b->y - (e->y - p->h / 2));
        /* vertical tolerance padded by 6px so grazing hits count (forgiving) */
        if (dx > -(s16)(p->w / 2) && dx < (s16)(p->w / 2) && dy > -(s16)(p->h / 2 + 6) &&
            dy < (s16)(p->h / 2 + 6)) {
          b->active = 0;
          enemy_hurt(e, 1);
          break;
        }
      }
    }
  }
}

/* --- the frame ------------------------------------------------------------------- */
void action_service(void) {
  const EdgeStage *st = act.st;
  Spr *s = &g.spr[act.pl_slot];
  const EdgeProto *pp = &g.sc->protos[st->player_proto];
  u8 world_frame;
  s16 run_vx;
  s16 gate_x = 0x7fff;
  int i;

  if (!act.active) return;
  act.frame++;

  /* -- sandevistan gauge ---------------------------------------------------------- */
  {
    u8 want = (u8)(st->sande_max && key_held(KEY_R) && g.vars[st->sande_var] > 0);
    if (want && !act.sande_on) sfx_play(C_SFX_WHOOSH);
    if (!want && act.sande_on) dma3_copy32(BG_PAL, g.sc->pal_bg, 512 / 4);
    if (want && !act.sande_on && st->pal_sande) dma3_copy32(BG_PAL, st->pal_sande, 512 / 4);
    act.sande_on = want;
    if (act.sande_on) {
      if (++act.sande_frames >= C_ACT_SANDE_DRAIN) {
        act.sande_frames = 0;
        g.vars[st->sande_var]--;
      }
    } else if (g.vars[st->sande_var] < st->sande_max) {
      if (++act.sande_frames >= C_ACT_SANDE_REGEN) {
        act.sande_frames = 0;
        g.vars[st->sande_var]++;
      }
    }
  }
  world_frame = (u8)(!act.sande_on || (act.frame % C_ACT_SLOW_DIV) == 0);

  /* -- spawns + gates -------------------------------------------------------------- */
  while (act.next_spawn < st->n_spawns && st->spawns[act.next_spawn].x < act.x + SPAWN_AHEAD) {
    if (act.alive < C_MAX_ENEMIES) {
      enemy_activate(act.next_spawn);
      act.next_spawn++;
    } else break;
  }
  for (i = 0; i < st->n_gates; i++) {
    const EdgeGate *gt = &st->gates[i];
    int j;
    u8 live = 0;
    for (j = 0; j < C_MAX_ENEMIES; j++)
      if (act.en[j].active && act.en[j].wave == gt->wave) live = 1;
    /* the gate holds while its wave has activated members alive, or members
     * still queued at/behind the gate line */
    if (!live) {
      for (j = act.next_spawn; j < st->n_spawns; j++)
        if (st->spawns[j].wave == gt->wave && st->spawns[j].x < gt->x + SPAWN_AHEAD) live = 1;
    }
    if (live && gt->x < gate_x) gate_x = gt->x;
    if (!live && act.checkpoint_x < gt->x) act.checkpoint_x = gt->x; /* checkpoint */
  }

  /* -- player --------------------------------------------------------------------- */
  run_vx = act.sande_on ? C_ACT_SANDE_VX : C_ACT_RUN_VX;
  if (act.hurt_timer) {
    act.hurt_timer--;
  } else {
    if (key_held(KEY_LEFT)) {
      act.vx_q4 = (s16)-run_vx;
      act.face_left = 1;
    } else if (key_held(KEY_RIGHT)) {
      act.vx_q4 = run_vx;
      act.face_left = 0;
    } else {
      act.vx_q4 = 0;
    }
    if (key_pressed(KEY_A) && act.grounded) {
      act.vy_q4 = C_ACT_JUMP_VY;
      act.grounded = 0;
      sfx_play(C_SFX_CONFIRM);
    }
  }
  /* gravity + integrate */
  if (!act.grounded) act.vy_q4 = (s16)(act.vy_q4 + C_ACT_GRAVITY);
  act.x_q4 += act.vx_q4;
  act.y_q4 += act.vy_q4;
  act.x = (s16)(act.x_q4 >> 4);
  act.y = (s16)(act.y_q4 >> 4);

  /* bounds: stage [8, length-8], gate line */
  {
    s16 max_x = (s16)(st->length - 8);
    if (gate_x != 0x7fff && gate_x < max_x) max_x = gate_x;
    if (act.x < 8) act.x = 8;
    if (act.x > max_x) act.x = max_x;
    act.x_q4 = (s32)act.x << 4;
  }
  /* ground + one-way platforms (land only while falling) */
  {
    s16 floor_y = st->ground_y;
    for (i = 0; i < st->n_plats; i++) {
      const EdgePlat *pl = &st->plats[i];
      if (act.x >= pl->x && act.x < pl->x + pl->w && act.vy_q4 >= 0 && act.y >= pl->y &&
          act.y <= pl->y + 6 && pl->y < floor_y)
        floor_y = pl->y;
    }
    if (act.vy_q4 >= 0 && act.y >= floor_y) {
      act.y = floor_y;
      act.y_q4 = (s32)act.y << 4;
      act.vy_q4 = 0;
      act.grounded = 1;
    } else if (act.y < floor_y) {
      act.grounded = 0;
    }
  }

  /* -- shooting -------------------------------------------------------------------- */
  if (act.shoot_cd) act.shoot_cd--;
  if (!act.hurt_timer && key_pressed(KEY_B) && !act.shoot_cd) {
    u8 melee = 0;
    act.shoot_cd = act.sande_on ? (u8)(C_ACT_SHOOT_CD / 2) : C_ACT_SHOOT_CD;
    for (i = 0; i < C_MAX_ENEMIES; i++) {
      ActEnemy *e = &act.en[i];
      s16 dx;
      if (!e->active) continue;
      dx = (s16)(e->x - act.x);
      if (dx > -C_ACT_MELEE_RANGE && dx < C_ACT_MELEE_RANGE &&
          (act.y - e->y) > -28 && (act.y - e->y) < 20) {
        enemy_hurt(e, 2); /* point-blank: pistol-whip */
        melee = 1;
        break;
      }
    }
    if (!act.done && !melee) {
      /* hold UP for a steep rising diagonal — the anti-drone shot */
      s16 bvy = key_held(KEY_UP) ? (s16)(-C_ACT_BULLET_VX * 3 / 4) : 0;
      bullet_fire((s16)(act.x + (act.face_left ? -10 : 10)), (s16)(act.y - 14),
                  (s16)(act.face_left ? -C_ACT_BULLET_VX : C_ACT_BULLET_VX), bvy, 0,
                  C_UIT_PBULLET);
      sfx_play(C_SFX_BLIP);
    }
  }
  if (act.done) return; /* boss-phase finish fired from enemy_hurt */

  /* -- world tick (slowed under sandevistan) --------------------------------------- */
  if (world_frame) {
    enemies_update();
    bullets_update();
    act.world_tick++;
  }
  if (act.done) return;

  /* -- iframes + camera + sprite pose ---------------------------------------------- */
  if (act.iframes) act.iframes--;
  {
    s16 target = (s16)(act.x - 104);
    s16 cmax = (s16)(st->length - 240);
    if (target < 0) target = 0;
    if (target > cmax) target = cmax;
    g.fx[TW_CAM_X] += (s16)((target - g.fx[TW_CAM_X]) >> 3);
  }
  {
    u8 f;
    if (act.hurt_timer) f = C_PF_HURT;
    else if (!act.grounded) f = C_PF_JUMP;
    else if (act.shoot_cd > C_ACT_SHOOT_CD - 5) f = C_PF_SHOOT;
    else if (act.vx_q4) f = (u8)(C_PF_RUN0 + ((act.frame >> (act.sande_on ? 2 : 3)) & 3));
    else f = C_PF_IDLE;
    player_frame(f);
    s->x = (s16)(act.x - pp->w / 2);
    s->y = (s16)(act.y - pp->h);
    s->flags = act.face_left ? (u8)(s->flags | C_SPR_HFLIP) : (u8)(s->flags & ~C_SPR_HFLIP);
    /* iframe blink */
    s->active = (u8)(!(act.iframes & 2));
  }
  /* afterimage ring (every 4th frame while sandevistan is engaged) */
  if (act.sande_on && (act.frame & 3) == 0) {
    act.gx[act.ghead] = s->x;
    act.gy[act.ghead] = s->y;
    act.gframe[act.ghead] = g.spr[act.pl_slot].frame;
    act.gface[act.ghead] = act.face_left;
    act.ghead = (u8)((act.ghead + 1) % C_ACT_GHOSTS);
  }

  /* -- exit ------------------------------------------------------------------------- */
  {
    u8 cleared = 0;
    if (st->exit_kind == C_AEXIT_CLEAR)
      cleared = (u8)(act.next_spawn >= st->n_spawns && act.alive == 0);
    else
      cleared = (u8)(act.x >= st->length - 12);
    if (cleared) action_finish(C_ACT_CLEARED);
  }
}

/* --- OAM -------------------------------------------------------------------------- */
void action_draw(void) {
  s16 cam = g.fx[TW_CAM_X];
  int i;
  if (!act.active) return;

  for (i = 0; i < C_MAX_ENEMIES; i++) {
    ActEnemy *e = &act.en[i];
    ObjAttr *o = &oam_shadow[C_OAM_ENEMY + i];
    const EdgeProto *p;
    s16 sx, sy;
    u8 frame;
    if (!e->active) {
      o->attr0 = ATTR0_HIDE;
      continue;
    }
    p = eproto(e);
    sx = (s16)(e->x - p->w / 2 - cam);
    sy = (s16)(e->y - p->h - g.fx[TW_CAM_Y]);
    if (sx <= -(s16)p->w * 2 || sx >= 240 + p->w) {
      o->attr0 = ATTR0_HIDE;
      continue;
    }
    if (e->tele || (e->behavior == C_EB_GUNNER && (e->timer >= 76 && e->timer <= 98)))
      frame = C_EF_ATTACK;
    else if (e->vx_q4 || e->behavior == C_EB_THUG || e->behavior == C_EB_BOSS)
      frame = (u8)((e->anim >> 4) & 1 ? C_EF_WALK0 : C_EF_WALK1);
    else
      frame = (u8)((e->anim >> 5) & 1 ? C_EF_IDLE : C_EF_WALK0);
    if (frame >= p->frames) frame = 0;
    o->attr0 = (u16)(ATTR0_Y(sy) | ((p->w == p->h) ? ATTR0_SQUARE : (p->w > p->h ? ATTR0_WIDE : ATTR0_TALL)));
    o->attr1 = (u16)(ATTR1_X(sx) | (e->face ? ATTR1_HFLIP : 0) |
                     ATTR1_SIZE(p->w == p->h ? (p->w == 8 ? 0 : p->w == 16 ? 1 : p->w == 32 ? 2 : 3)
                                             : (p->w == 32 && p->h == 16 ? 2 : 3)));
    o->attr2 = (u16)(ATTR2_TILE(p->tile_base + frame * ((p->w / 8) * (p->h / 8))) | ATTR2_PRIO(1) |
                     ATTR2_PALBANK(p->palbank));
  }

  for (i = 0; i < C_MAX_BULLETS; i++) {
    ActBullet *b = &act.bl[i];
    ObjAttr *o = &oam_shadow[C_OAM_BULLET + i];
    if (!b->active) {
      o->attr0 = ATTR0_HIDE;
      continue;
    }
    if (b->tile == C_UIT_SHOCK) {
      o->attr0 = (u16)(ATTR0_Y(b->y - 4) | ATTR0_WIDE);
      o->attr1 = (u16)(ATTR1_X(b->x - 8 - cam) | ATTR1_SIZE(0)); /* 16x8 */
    } else {
      o->attr0 = (u16)(ATTR0_Y(b->y - 4) | ATTR0_SQUARE);
      o->attr1 = (u16)(ATTR1_X(b->x - 4 - cam) | ATTR1_SIZE(0)); /* 8x8 */
    }
    o->attr2 = (u16)(ATTR2_TILE(C_OBJ_UI_BASE + b->tile) | ATTR2_PRIO(1) |
                     ATTR2_PALBANK(C_PALBANK_OBJ_UI));
  }

  /* sandevistan afterimages: ghost-blended copies of recent player poses */
  {
    const EdgeProto *pp = &g.sc->protos[act.st->player_proto];
    for (i = 0; i < C_ACT_GHOSTS; i++) {
      ObjAttr *o = &oam_shadow[C_OAM_GHOST + i];
      s16 sx = (s16)(act.gx[i] - cam);
      if (!act.sande_on || sx <= -64 || sx >= 240) {
        o->attr0 = ATTR0_HIDE;
        continue;
      }
      o->attr0 = (u16)(ATTR0_Y(act.gy[i] - g.fx[TW_CAM_Y]) | ATTR0_BLEND |
                       ((pp->w == pp->h) ? ATTR0_SQUARE : ATTR0_TALL));
      o->attr1 = (u16)(ATTR1_X(sx) | (act.gface[i] ? ATTR1_HFLIP : 0) |
                       ATTR1_SIZE(pp->w == 8 ? 0 : pp->w == 16 ? 1 : pp->w == 32 ? 2 : 3));
      o->attr2 = (u16)(ATTR2_TILE(pp->tile_base + act.gframe[i] * ((pp->w / 8) * (pp->h / 8))) |
                       ATTR2_PRIO(1) | ATTR2_PALBANK(pp->palbank));
    }
  }
}
