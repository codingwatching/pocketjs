//! Per-frame input snapshot (DESIGN.md §15).
//!
//! The runtime samples raw device state into an [`InputSnapshot`] once per
//! frame; simulation reads only the snapshot, never the OS input queue.

/// Logical keyboard keys OpenStrike cares about. Kept small on purpose.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Shift,
    Ctrl,
    R,
    Escape,
    F1,
    F3,
}

/// Mouse buttons.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Button {
    Left,
    Right,
    Middle,
}

const KEY_COUNT: usize = 11;
const BUTTON_COUNT: usize = 3;

fn key_index(k: Key) -> usize {
    match k {
        Key::W => 0,
        Key::A => 1,
        Key::S => 2,
        Key::D => 3,
        Key::Space => 4,
        Key::Shift => 5,
        Key::Ctrl => 6,
        Key::R => 7,
        Key::Escape => 8,
        Key::F1 => 9,
        Key::F3 => 10,
    }
}

fn button_index(b: Button) -> usize {
    match b {
        Button::Left => 0,
        Button::Right => 1,
        Button::Middle => 2,
    }
}

/// State of all inputs for one frame, including edge (just-pressed) detection.
#[derive(Clone, Debug, Default)]
pub struct InputSnapshot {
    keys_down: [bool; KEY_COUNT],
    keys_prev: [bool; KEY_COUNT],
    buttons_down: [bool; BUTTON_COUNT],
    buttons_prev: [bool; BUTTON_COUNT],
    /// Accumulated mouse delta since the last frame, in raw device units.
    pub mouse_delta: (f32, f32),
}

impl InputSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call once at the start of a frame after copying current->prev, then feed
    /// events. Here we roll the current state into `prev` and reset the mouse
    /// delta; callers then set the new pressed state via [`Self::set_key`] etc.
    pub fn begin_frame(&mut self) {
        self.keys_prev = self.keys_down;
        self.buttons_prev = self.buttons_down;
        self.mouse_delta = (0.0, 0.0);
    }

    pub fn set_key(&mut self, k: Key, down: bool) {
        self.keys_down[key_index(k)] = down;
    }

    pub fn set_button(&mut self, b: Button, down: bool) {
        self.buttons_down[button_index(b)] = down;
    }

    pub fn add_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    pub fn key_down(&self, k: Key) -> bool {
        self.keys_down[key_index(k)]
    }

    pub fn key_pressed(&self, k: Key) -> bool {
        let i = key_index(k);
        self.keys_down[i] && !self.keys_prev[i]
    }

    pub fn button_down(&self, b: Button) -> bool {
        self.buttons_down[button_index(b)]
    }

    pub fn button_pressed(&self, b: Button) -> bool {
        let i = button_index(b);
        self.buttons_down[i] && !self.buttons_prev[i]
    }

    /// Movement axes derived from WASD: `x` = right(+)/left(-),
    /// `y` = forward(+)/back(-). Not normalized.
    pub fn move_axes(&self) -> (f32, f32) {
        let x = self.key_down(Key::D) as i32 as f32 - self.key_down(Key::A) as i32 as f32;
        let y = self.key_down(Key::W) as i32 as f32 - self.key_down(Key::S) as i32 as f32;
        (x, y)
    }
}
