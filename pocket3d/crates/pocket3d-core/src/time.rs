//! Fixed-timestep clock (DESIGN.md §6).
//!
//! The runtime accumulates real time, runs zero or more fixed simulation
//! updates, then renders with an interpolation `alpha`.

/// Information for a single fixed simulation tick.
#[derive(Clone, Copy, Debug)]
pub struct TickInfo {
    /// Fixed timestep in seconds (e.g. `1/60`).
    pub dt: f32,
    /// Monotonic tick index since startup.
    pub tick: u64,
}

/// Accumulator that turns variable real-time frames into fixed simulation
/// ticks plus a render interpolation factor.
#[derive(Clone, Debug)]
pub struct FixedClock {
    dt: f32,
    accumulator: f32,
    tick: u64,
    /// Cap on how many ticks may run in one frame to avoid a spiral of death.
    max_ticks_per_frame: u32,
}

impl FixedClock {
    /// Create a clock running at `hz` simulation ticks per second.
    pub fn new(hz: f32) -> Self {
        assert!(hz > 0.0, "tick rate must be positive");
        Self {
            dt: 1.0 / hz,
            accumulator: 0.0,
            tick: 0,
            max_ticks_per_frame: 8,
        }
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Feed `frame_dt` real seconds. Returns the [`TickInfo`]s that should be
    /// simulated this frame (possibly zero, possibly several).
    pub fn advance(&mut self, frame_dt: f32) -> Vec<TickInfo> {
        // Clamp pathological frame times (e.g. after a breakpoint).
        let frame_dt = frame_dt.clamp(0.0, self.dt * self.max_ticks_per_frame as f32);
        self.accumulator += frame_dt;
        let mut ticks = Vec::new();
        while self.accumulator >= self.dt && (ticks.len() as u32) < self.max_ticks_per_frame {
            self.accumulator -= self.dt;
            ticks.push(TickInfo {
                dt: self.dt,
                tick: self.tick,
            });
            self.tick += 1;
        }
        ticks
    }

    /// Render interpolation factor in `[0, 1)` — how far we are between the
    /// last simulated tick and the next.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / self.dt).clamp(0.0, 1.0)
    }
}
