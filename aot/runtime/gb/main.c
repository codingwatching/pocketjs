// aot/runtime/gb/main.c — entry point + per-frame loop (Game Boy).
#include "gbrt.h"

void main(void) {
  video_boot();
  textbox_init();
  g.pending_enter = -1;
  map_enter(PJ_START_MAP, PJ_START_X, PJ_START_Y, PJ_START_DIR);
  debug_flush();

  while (1) {
    wait_vbl_done();
    // vblank-only work first: scroll regs + queued VRAM writes
    SCX_REG = (u8)g.cam_x;
    SCY_REG = (u8)g.cam_y;
    textbox_pump();

    input_poll();
    if (!vm_active() && g.pending_enter >= 0) {
      u8 sid = (u8)g.pending_enter;
      g.pending_enter = -1;
      vm_start(sid, -1);
    }
    vm_tick();
    if (!vm_active()) player_update();
    textbox_tick();
    choice_tick();
    camera_update();
    scene_draw();
    debug_flush();
    g.frame++;
  }
}
