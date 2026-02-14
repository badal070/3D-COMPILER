//! Frame Synchronization
//!
//! Syncs runtime ticks with render ticks.
//! Handles dropped frames. Prevents visual desync.
//!
//! Renderer runs at its own pace. Runtime doesn't care.

use std::time::{Duration, Instant};

/// Manages frame timing and synchronization
///
/// Responsibilities:
/// - Maintain target framerate
/// - Track dropped frames
/// - Provide interpolation alpha
/// - Isolate render timing from runtime timing
pub struct FrameSync {
    /// Target frames per second
    target_fps: u32,
    /// Duration between frames
    frame_duration: Duration,
    /// Last frame timestamp
    last_frame: Option<Instant>,
    /// Accumulated frame time for interpolation
    accumulator: Duration,
    /// Total frames rendered
    frame_count: u64,
    /// Total frames dropped
    dropped_frames: u64,
}

impl FrameSync {
    /// Create new frame sync with target FPS
    pub fn new(target_fps: u32) -> Self {
        let target_fps = target_fps.max(1); // Minimum 1 FPS
        let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);

        Self {
            target_fps,
            frame_duration,
            last_frame: None,
            accumulator: Duration::ZERO,
            frame_count: 0,
            dropped_frames: 0,
        }
    }

    /// Check if a frame should be rendered now
    ///
    /// Returns true if enough time has passed since last frame
    pub fn should_render(&mut self) -> bool {
        let now = Instant::now();

        if let Some(last) = self.last_frame {
            let elapsed = now.duration_since(last);
            self.accumulator += elapsed;

            // Check if we should render
            if self.accumulator >= self.frame_duration {
                // Track dropped frames
                let frames_passed =
                    self.accumulator.as_secs_f64() / self.frame_duration.as_secs_f64();
                if frames_passed > 1.5 {
                    self.dropped_frames += (frames_passed - 1.0) as u64;
                }

                // Reset accumulator, keeping remainder
                if self.accumulator >= self.frame_duration * 2 {
                    // If we're more than 2 frames behind, reset completely
                    self.accumulator = Duration::ZERO;
                } else {
                    self.accumulator -= self.frame_duration;
                }

                true
            } else {
                false
            }
        } else {
            // First frame always renders
            true
        }
    }

    /// Mark current frame as complete
    ///
    /// Updates timing state
    pub fn frame_complete(&mut self) {
        self.last_frame = Some(Instant::now());
        self.frame_count += 1;
    }

    /// Get interpolation alpha between frames
    ///
    /// Returns value in [0, 1] indicating position between frames
    /// Used for smooth visual interpolation
    pub fn interpolation_alpha(&self) -> f64 {
        if self.accumulator >= self.frame_duration {
            1.0
        } else {
            self.accumulator.as_secs_f64() / self.frame_duration.as_secs_f64()
        }
    }

    /// Get current frames per second target
    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    /// Get actual FPS based on recent frames
    pub fn actual_fps(&self) -> f64 {
        if let Some(last) = self.last_frame {
            let elapsed = last.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                1.0 / elapsed
            } else {
                self.target_fps as f64
            }
        } else {
            0.0
        }
    }

    /// Get total rendered frames
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get total dropped frames
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    /// Get frame drop ratio
    pub fn drop_ratio(&self) -> f64 {
        if self.frame_count > 0 {
            self.dropped_frames as f64 / self.frame_count as f64
        } else {
            0.0
        }
    }

    /// Update target FPS
    pub fn set_target_fps(&mut self, target_fps: u32) {
        self.target_fps = target_fps.max(1);
        self.frame_duration = Duration::from_secs_f64(1.0 / self.target_fps as f64);
    }

    /// Reset timing state
    pub fn reset(&mut self) {
        self.last_frame = None;
        self.accumulator = Duration::ZERO;
        self.frame_count = 0;
        self.dropped_frames = 0;
    }

    /// Get timing statistics
    pub fn stats(&self) -> FrameSyncStats {
        FrameSyncStats {
            target_fps: self.target_fps,
            actual_fps: self.actual_fps(),
            frame_count: self.frame_count,
            dropped_frames: self.dropped_frames,
            drop_ratio: self.drop_ratio(),
        }
    }
}

/// Frame synchronization statistics
#[derive(Debug, Clone, Copy)]
pub struct FrameSyncStats {
    pub target_fps: u32,
    pub actual_fps: f64,
    pub frame_count: u64,
    pub dropped_frames: u64,
    pub drop_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_first_frame_always_renders() {
        let mut sync = FrameSync::new(60);
        assert!(sync.should_render());
    }

    #[test]
    fn test_frame_timing() {
        let mut sync = FrameSync::new(60);

        // First frame
        assert!(sync.should_render());
        sync.frame_complete();

        // Immediately after, should not render
        assert!(!sync.should_render());
    }

    #[test]
    fn test_frame_count() {
        let mut sync = FrameSync::new(60);

        sync.should_render();
        sync.frame_complete();
        assert_eq!(sync.frame_count(), 1);

        sync.should_render();
        sync.frame_complete();
        assert_eq!(sync.frame_count(), 2);
    }

    #[test]
    fn test_interpolation_alpha() {
        let sync = FrameSync::new(60);
        let alpha = sync.interpolation_alpha();

        // Should be in valid range
        assert!(alpha >= 0.0 && alpha <= 1.0);
    }

    #[test]
    fn test_target_fps_update() {
        let mut sync = FrameSync::new(60);
        assert_eq!(sync.target_fps(), 60);

        sync.set_target_fps(30);
        assert_eq!(sync.target_fps(), 30);
    }

    #[test]
    fn test_minimum_fps() {
        let sync = FrameSync::new(0);
        assert_eq!(sync.target_fps(), 1);
    }

    #[test]
    fn test_reset() {
        let mut sync = FrameSync::new(60);

        sync.should_render();
        sync.frame_complete();
        assert!(sync.frame_count() > 0);

        sync.reset();
        assert_eq!(sync.frame_count(), 0);
        assert_eq!(sync.dropped_frames(), 0);
    }

    #[test]
    fn test_stats() {
        let mut sync = FrameSync::new(60);

        sync.should_render();
        sync.frame_complete();

        let stats = sync.stats();
        assert_eq!(stats.target_fps, 60);
        assert_eq!(stats.frame_count, 1);
    }

    #[test]
    #[ignore] // This test requires actual sleep time
    fn test_actual_timing() {
        let mut sync = FrameSync::new(10); // 10 FPS = 100ms per frame

        assert!(sync.should_render());
        sync.frame_complete();

        // Wait less than frame duration
        thread::sleep(Duration::from_millis(50));
        assert!(!sync.should_render());

        // Wait until frame duration passes
        thread::sleep(Duration::from_millis(60));
        assert!(sync.should_render());
    }
}
