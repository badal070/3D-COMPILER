// runtime/state/time_state.rs
// Tracks: current time, delta time, time domain bounds, paused/stepped modes
// Time is explicit. Never implicit.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeState {
    /// Current simulation time
    pub current_time: f64,
    /// Last time step delta
    pub delta_time: f64,
    /// Accumulated real time (for FPS tracking)
    pub real_time: f64,
    /// Time domain bounds
    pub bounds: TimeBounds,
    /// Execution mode
    pub mode: TimeMode,
    /// Total steps executed
    pub step_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeBounds {
    /// Minimum time (usually 0)
    pub min: f64,
    /// Maximum time (if bounded)
    pub max: Option<f64>,
    /// Should time wrap around
    pub wraps: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeMode {
    /// Running normally
    Running,
    /// Paused
    Paused,
    /// Single-step mode
    Stepping,
}

impl Default for TimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeState {
    pub fn new() -> Self {
        Self {
            current_time: 0.0,
            delta_time: 0.0,
            real_time: 0.0,
            bounds: TimeBounds::default(),
            mode: TimeMode::Paused,
            step_count: 0,
        }
    }

    pub fn with_bounds(mut self, min: f64, max: Option<f64>, wraps: bool) -> Self {
        self.bounds = TimeBounds { min, max, wraps };
        self
    }

    pub fn start(&mut self) {
        self.mode = TimeMode::Running;
    }

    pub fn pause(&mut self) {
        self.mode = TimeMode::Paused;
    }

    pub fn step(&mut self) {
        self.mode = TimeMode::Stepping;
    }

    pub fn reset(&mut self) {
        self.current_time = self.bounds.min;
        self.delta_time = 0.0;
        self.real_time = 0.0;
        self.step_count = 0;
        self.mode = TimeMode::Paused;
    }

    /// Advance time by delta
    pub fn advance(&mut self, dt: f64) -> Result<(), String> {
        if dt < 0.0 {
            return Err("Cannot advance time by negative delta".to_string());
        }
        if dt.is_nan() {
            return Err("Time delta is NaN".to_string());
        }
        if dt.is_infinite() {
            return Err("Time delta is infinite".to_string());
        }

        self.delta_time = dt;
        self.current_time += dt;
        self.step_count += 1;

        // Handle time bounds
        if let Some(max) = self.bounds.max {
            if self.bounds.wraps {
                let range = max - self.bounds.min;
                while self.current_time > max {
                    self.current_time -= range;
                }
            } else if self.current_time > max {
                self.current_time = max;
                self.mode = TimeMode::Paused;
            }
        }

        Ok(())
    }

    /// Add real time (wall clock)
    pub fn add_real_time(&mut self, dt: f64) {
        self.real_time += dt;
    }

    /// Get time scale (sim time / real time)
    pub fn time_scale(&self) -> f64 {
        if self.real_time > 0.0 {
            self.current_time / self.real_time
        } else {
            0.0
        }
    }

    /// Get average FPS
    pub fn average_fps(&self) -> f64 {
        if self.real_time > 0.0 {
            self.step_count as f64 / self.real_time
        } else {
            0.0
        }
    }

    /// Is time at end of bounds
    pub fn at_end(&self) -> bool {
        if let Some(max) = self.bounds.max {
            (self.current_time - max).abs() < 1e-10
        } else {
            false
        }
    }

    /// Is time paused or stepping
    pub fn is_paused(&self) -> bool {
        matches!(self.mode, TimeMode::Paused | TimeMode::Stepping)
    }

    /// Can advance time
    pub fn can_advance(&self) -> bool {
        match self.mode {
            TimeMode::Running => true,
            TimeMode::Stepping => true,
            TimeMode::Paused => false,
        }
    }

    /// Validate time state
    pub fn validate(&self) -> Result<(), String> {
        if self.current_time.is_nan() {
            return Err("Current time is NaN".to_string());
        }
        if self.current_time.is_infinite() {
            return Err("Current time is infinite".to_string());
        }
        if self.delta_time.is_nan() {
            return Err("Delta time is NaN".to_string());
        }
        if self.delta_time.is_infinite() {
            return Err("Delta time is infinite".to_string());
        }

        if self.current_time < self.bounds.min {
            return Err(format!(
                "Current time {} below minimum {}",
                self.current_time, self.bounds.min
            ));
        }

        if let Some(max) = self.bounds.max {
            if !self.bounds.wraps && self.current_time > max {
                return Err(format!(
                    "Current time {} exceeds maximum {}",
                    self.current_time, max
                ));
            }
        }

        Ok(())
    }

    /// Get progress through time bounds (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if let Some(max) = self.bounds.max {
            let range = max - self.bounds.min;
            if range > 0.0 {
                ((self.current_time - self.bounds.min) / range).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl Default for TimeBounds {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: None,
            wraps: false,
        }
    }
}

impl TimeBounds {
    pub fn new(min: f64, max: Option<f64>) -> Self {
        Self {
            min,
            max,
            wraps: false,
        }
    }

    pub fn bounded(min: f64, max: f64) -> Self {
        Self {
            min,
            max: Some(max),
            wraps: false,
        }
    }

    pub fn wrapping(min: f64, max: f64) -> Self {
        Self {
            min,
            max: Some(max),
            wraps: true,
        }
    }

    pub fn unbounded() -> Self {
        Self {
            min: 0.0,
            max: None,
            wraps: false,
        }
    }
}
