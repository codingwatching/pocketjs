/* aot/runtime/nes/vm.c — the event-script stack machine (NES port).
 *
 * Script bytecode lives in the FIXED bank, so the interpreter reads it
 * through a plain pointer with no bank juggling. Semantics mirror
 * runtime/gba/script_vm.c op for op (the cross-target E2E suite drives
 * identical scenarios on every platform). */
#include "nesrt.h"

static u8 rd_u8(void) { return g.vm.code[g.vm.ip++]; }
static u16 rd_u16(void) {
  u16 v = (u16)g.vm.code[g.vm.ip] | ((u16)g.vm.code[g.vm.ip + 1] << 8);
  g.vm.ip += 2;
  return v;
}
static s16 rd_i16(void) { return (s16)rd_u16(); }

static void push(s16 v) {
  if (g.vm.sp < PJGB_VM_MAX_STACK) g.vm.stack[g.vm.sp++] = v;
}
static s16 pop(void) {
  if (g.vm.sp > 0) return g.vm.stack[--g.vm.sp];
  return 0;
}

static void face_player(s8 slot) {
  s16 ax, ay, px, py, dx, dy, adx, ady;
  u8 dir;
  if (slot < 0 || slot >= (s8)g.n_actors) return;
  ax = (s16)g.actors[(u8)slot].x;
  ay = (s16)g.actors[(u8)slot].y;
  px = g.px >> 3;
  py = g.py >> 3;
  dx = px - ax;
  dy = py - ay;
  adx = dx < 0 ? -dx : dx;
  ady = dy < 0 ? -dy : dy;
  if (adx >= ady) dir = dx > 0 ? DIR_RIGHT : DIR_LEFT;
  else dir = dy > 0 ? DIR_DOWN : DIR_UP;
  g.actor_dir[(u8)slot] = dir;
}

void vm_start(u8 script_id, s8 actor_slot) {
  g.vm.code = pj_scripts + pj_script_offs[script_id];
  g.vm.ip = 0;
  g.vm.sp = 0;
  g.vm.active = 1;
  g.vm.suspend = VM_SUSP_NONE;
  g.vm.wait_frames = 0;
  g.vm.actor_slot = actor_slot;
}

u8 vm_active(void) { return g.vm.active; }

void vm_tick(void) {
  if (!g.vm.active) return;

  switch (g.vm.suspend) {
    case VM_SUSP_WAIT:
      if (--g.vm.wait_frames == 0) g.vm.suspend = VM_SUSP_NONE;
      else return;
      break;
    case VM_SUSP_TEXT:
      if (!textbox_active()) g.vm.suspend = VM_SUSP_NONE;
      else return;
      break;
    case VM_SUSP_CHOICE:
      if (choice_result() >= 0) {
        push((s16)choice_result());
        g.vm.suspend = VM_SUSP_NONE;
      } else {
        return;
      }
      break;
    default:
      break;
  }

  for (;;) {
    u8 op = rd_u8();
    switch (op) {
      case OP_END:
        g.vm.active = 0;
        return;
      case OP_NOP:
        break;
      case OP_TEXT: {
        u16 t = rd_u16();
        textbox_show(t);
        g.vm.suspend = VM_SUSP_TEXT;
        return;
      }
      case OP_SET_FLAG: {
        u16 f = rd_u16();
        flag_set1(f);
        break;
      }
      case OP_CLEAR_FLAG: {
        u16 f = rd_u16();
        flag_set0(f);
        break;
      }
      case OP_PUSH_FLAG: {
        u16 f = rd_u16();
        push((s16)flag_get(f));
        break;
      }
      case OP_PUSH_CONST:
        push(rd_i16());
        break;
      case OP_POP:
        pop();
        break;
      case OP_DUP: {
        s16 v = pop();
        push(v);
        push(v);
        break;
      }
      case OP_EQ: {
        s16 b = pop(), a = pop();
        push(a == b ? 1 : 0);
        break;
      }
      case OP_NE: {
        s16 b = pop(), a = pop();
        push(a != b ? 1 : 0);
        break;
      }
      case OP_NOT: {
        s16 a = pop();
        push(a ? 0 : 1);
        break;
      }
      case OP_JUMP: {
        s16 rel = rd_i16();
        g.vm.ip = (u16)((s16)g.vm.ip + rel);
        break;
      }
      case OP_JUMP_IF_FALSE: {
        s16 rel = rd_i16();
        if (!pop()) g.vm.ip = (u16)((s16)g.vm.ip + rel);
        break;
      }
      case OP_CHOICE: {
        u8 n = rd_u8();
        u16 ids[8];
        u8 i;
        for (i = 0; i < n; i++) {
          u16 id = rd_u16();
          if (i < 8) ids[i] = id;
        }
        if (n > 8) n = 8;
        choice_show(n, ids);
        g.vm.suspend = VM_SUSP_CHOICE;
        return;
      }
      case OP_LOCK_PLAYER:
        g.locked = 1;
        break;
      case OP_RELEASE_PLAYER:
        g.locked = 0;
        break;
      case OP_FACE_PLAYER: {
        u8 slot = rd_u8();
        if (slot == 0xff) {
          if (g.vm.actor_slot < 0) break;
          slot = (u8)g.vm.actor_slot;
        }
        face_player((s8)slot);
        break;
      }
      case OP_WARP: {
        u8 m = rd_u8();
        u16 x = rd_u16();
        u16 y = rd_u16();
        u8 d = rd_u8();
        map_enter(m, (u8)x, (u8)y, d);
        break;
      }
      case OP_SET_VAR: {
        u16 i = rd_u16();
        s16 v = rd_i16();
        if (i < BUDGET_MAX_VARS) g.vars[i] = v;
        break;
      }
      case OP_ADD_VAR: {
        u16 i = rd_u16();
        s16 v = rd_i16();
        if (i < BUDGET_MAX_VARS) g.vars[i] += v;
        break;
      }
      case OP_PUSH_VAR: {
        u16 i = rd_u16();
        push(i < BUDGET_MAX_VARS ? g.vars[i] : 0);
        break;
      }
      case OP_GIVE_ITEM:
        rd_u16();
        rd_u8(); /* stub (parity with GBA) */
        break;
      case OP_BATTLE:
        rd_u16();
        push(1); /* stub: "won" */
        break;
      case OP_WAIT: {
        u16 n = rd_u16();
        if (n == 0) break;
        g.vm.wait_frames = n;
        g.vm.suspend = VM_SUSP_WAIT;
        return;
      }
      case OP_PLAY_SFX:
        rd_u16();
        break;
      case OP_LT: {
        s16 b = pop(), a = pop();
        push(a < b ? 1 : 0);
        break;
      }
      case OP_GT: {
        s16 b = pop(), a = pop();
        push(a > b ? 1 : 0);
        break;
      }
      case OP_LE: {
        s16 b = pop(), a = pop();
        push(a <= b ? 1 : 0);
        break;
      }
      case OP_GE: {
        s16 b = pop(), a = pop();
        push(a >= b ? 1 : 0);
        break;
      }
      case OP_RND: {
        u8 n = rd_u8();
        if (g.rng == 0) g.rng = g.frame | 1;
        g.rng = g.rng * 25173u + 13849u;
        push(n ? (s16)((g.rng >> 4) % n) : 0);
        break;
      }
      default:
        g.vm.active = 0;
        return;
    }
  }
}
