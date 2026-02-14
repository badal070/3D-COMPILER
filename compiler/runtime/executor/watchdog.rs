// runtime/executor/watchdog.rs
// Your insurance policy
// Monitors: execution time, step count, memory usage, NaN propagation
// Kills execution politely but firmly

use crate::error::{RuntimeError, WatchdogError, WatchdogErrorKind};
use std::time::{Duration, Instant};

/// Watchdog monitors execution and enforces limits
pub struct Watchdog {
    /// Maximum steps allowed
    max_steps: u64,
    /// Maximum execution time
    max_execution_time: Duration,
    /// When execution started
    start_time: Option<Instant>,
    /// Current step count
    current_step: u64,
    /// Has NaN been detected
    nan_detected: bool,
    /// Enabled checks
    config: WatchdogConfig,
}

#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    pub check_steps: bool,
    pub check_time: bool,
    pub check_nan: bool,
    pub check_memory: bool,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_steps: true,
            check_time: true,
            check_nan: true,
            check_memory: false, // Memory checking is platform-specific
        }
    }
}

impl Watchdog {
    pub fn new(max_steps: u64, max_execution_time_ms: u64) -> Self {
        Self {
            max_steps,
            max_execution_time: Duration::from_millis(max_execution_time_ms),
            start_time: None,
            current_step: 0,
            nan_detected: false,
            config: WatchdogConfig::default(),
        }
    }

    pub fn with_config(mut self, config: WatchdogConfig) -> Self {
        self.config = config;
        self
    }

    /// Start monitoring
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.current_step = 0;
        self.nan_detected = false;
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        self.start_time = None;
    }

    /// Reset counters
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.nan_detected = false;
    }

    /// Record a step
    pub fn step(&mut self) {
        self.current_step += 1;
    }

    /// Record NaN detection
    pub fn record_nan(&mut self) {
        self.nan_detected = true;
    }

    /// Check all limits
    pub fn check(&self) -> Result<(), RuntimeError> {
        if self.config.check_steps {
            self.check_step_limit()?;
        }

        if self.config.check_time {
            self.check_time_limit()?;
        }

        if self.config.check_nan && self.nan_detected {
            return Err(RuntimeError::WatchdogTriggered(WatchdogError {
                kind: WatchdogErrorKind::StepLimit,
                limit: 0,
                actual: 0,
            }));
        }

        Ok(())
    }

    fn check_step_limit(&self) -> Result<(), RuntimeError> {
        if self.current_step >= self.max_steps {
            return Err(RuntimeError::WatchdogTriggered(WatchdogError {
                kind: WatchdogErrorKind::StepLimit,
                limit: self.max_steps,
                actual: self.current_step,
            }));
        }
        Ok(())
    }

    fn check_time_limit(&self) -> Result<(), RuntimeError> {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            if elapsed > self.max_execution_time {
                return Err(RuntimeError::WatchdogTriggered(WatchdogError {
                    kind: WatchdogErrorKind::TimeLimit,
                    limit: self.max_execution_time.as_millis() as u64,
                    actual: elapsed.as_millis() as u64,
                }));
            }
        }
        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> WatchdogStats {
        let elapsed = self
            .start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO);

        WatchdogStats {
            current_step: self.current_step,
            max_steps: self.max_steps,
            elapsed_ms: elapsed.as_millis() as u64,
            max_time_ms: self.max_execution_time.as_millis() as u64,
            nan_detected: self.nan_detected,
        }
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> WatchdogProgress {
        let step_progress = if self.max_steps > 0 {
            (self.current_step as f64 / self.max_steps as f64).min(1.0)
        } else {
            0.0
        };

        let time_progress = if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let ratio = elapsed.as_secs_f64() / self.max_execution_time.as_secs_f64();
            ratio.min(1.0)
        } else {
            0.0
        };

        WatchdogProgress {
            step_progress,
            time_progress,
            overall_progress: step_progress.max(time_progress),
        }
    }

    /// Is execution near limits
    pub fn is_near_limit(&self, threshold: f64) -> bool {
        let progress = self.progress();
        progress.overall_progress >= threshold
    }
}

#[derive(Debug, Clone)]
pub struct WatchdogStats {
    pub current_step: u64,
    pub max_steps: u64,
    pub elapsed_ms: u64,
    pub max_time_ms: u64,
    pub nan_detected: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct WatchdogProgress {
    pub step_progress: f64,
    pub time_progress: f64,
    pub overall_progress: f64,
}

impl WatchdogStats {
    pub fn step_utilization(&self) -> f64 {
        if self.max_steps > 0 {
            self.current_step as f64 / self.max_steps as f64
        } else {
            0.0
        }
    }

    pub fn time_utilization(&self) -> f64 {
        if self.max_time_ms > 0 {
            self.elapsed_ms as f64 / self.max_time_ms as f64
        } else {
            0.0
        }
    }

    pub fn steps_remaining(&self) -> u64 {
        self.max_steps.saturating_sub(self.current_step)
    }

    pub fn time_remaining_ms(&self) -> u64 {
        self.max_time_ms.saturating_sub(self.elapsed_ms)
    }
}
