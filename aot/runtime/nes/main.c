/* aot/runtime/nes/main.c — entry point + per-frame loop (NES). */
#include "nesrt.h"

void main(void) {
  video_boot();
  textbox_init();
  g.pending_enter = -1;
  map_enter(PJ_START_MAP, PJ_START_X, PJ_START_Y, PJ_START_DIR); /* enables NMI */
  debug_flush();

  while (1) {
    frame_sync(); /* wait for NMI (OAM DMA + vbuf flush done there) */

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
    textbox_pump(); /* append this frame's VRAM work for the next NMI */
    scene_draw();
    debug_flush();
    g.frame++;
  }
}
