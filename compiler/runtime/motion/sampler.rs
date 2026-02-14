// runtime/motion/sampler.rs
// Samples continuous motion
// Interpolates when needed
// Syncs with renderer tick
// Renderer follows runtime, not the other way around

use crate::error::RuntimeResult;
use crate::state::{ObjectId, ObjectState, Quaternion, Vector3, WorldState};
use std::collections::HashMap;

/// Motion sampler - samples object states at arbitrary times
pub struct MotionSampler {
    /// History of object states
    history: HashMap<ObjectId, Vec<StateSnapshot>>,
    /// Maximum history length
    max_history: usize,
}

#[derive(Debug, Clone)]
struct StateSnapshot {
    time: f64,
    position: Vector3,
    orientation: Quaternion,
    velocity: Option<Vector3>,
}

/// A sampled point in time
#[derive(Debug, Clone)]
pub struct SamplePoint {
    pub object_id: ObjectId,
    pub time: f64,
    pub position: Vector3,
    pub orientation: Quaternion,
    pub velocity: Option<Vector3>,
}

impl MotionSampler {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            max_history: 1000,
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Record current state for all objects
    pub fn record(&mut self, state: &WorldState, time: f64) {
        for (id, object) in &state.world.objects {
            let snapshot = StateSnapshot {
                time,
                position: object.position,
                orientation: object.orientation,
                velocity: object.velocity,
            };

            self.history
                .entry(id.clone())
                .or_insert_with(Vec::new)
                .push(snapshot);

            // Trim history if too long
            if let Some(history) = self.history.get_mut(id) {
                if history.len() > self.max_history {
                    history.remove(0);
                }
            }
        }
    }

    /// Sample all objects at a specific time
    pub fn sample_all(&self, state: &WorldState, time: f64) -> RuntimeResult<Vec<SamplePoint>> {
        let mut samples = Vec::new();

        for (id, object) in &state.world.objects {
            if let Some(sample) = self.sample_object(id, time, object) {
                samples.push(sample);
            }
        }

        Ok(samples)
    }

    /// Sample a specific object at a specific time
    pub fn sample_object(
        &self,
        id: &ObjectId,
        time: f64,
        fallback: &ObjectState,
    ) -> Option<SamplePoint> {
        // If no history, use current state
        let history = self.history.get(id)?;
        if history.is_empty() {
            return Some(SamplePoint {
                object_id: id.clone(),
                time,
                position: fallback.position,
                orientation: fallback.orientation,
                velocity: fallback.velocity,
            });
        }

        // Find bracketing snapshots
        let (before, after) = self.find_bracket(history, time);

        match (before, after) {
            (Some(b), Some(a)) => {
                // Interpolate between snapshots
                Some(self.interpolate(id, b, a, time))
            }
            (Some(b), None) => {
                // Extrapolate forward
                Some(self.extrapolate_forward(id, b, time))
            }
            (None, Some(a)) => {
                // Use first snapshot
                Some(SamplePoint {
                    object_id: id.clone(),
                    time,
                    position: a.position,
                    orientation: a.orientation,
                    velocity: a.velocity,
                })
            }
            (None, None) => None,
        }
    }

    fn find_bracket<'a>(
        &self,
        history: &'a [StateSnapshot],
        time: f64,
    ) -> (Option<&'a StateSnapshot>, Option<&'a StateSnapshot>) {
        let mut before = None;
        let mut after = None;

        for snapshot in history {
            if snapshot.time <= time {
                before = Some(snapshot);
            } else if after.is_none() {
                after = Some(snapshot);
                break;
            }
        }

        (before, after)
    }

    fn interpolate(
        &self,
        id: &ObjectId,
        before: &StateSnapshot,
        after: &StateSnapshot,
        time: f64,
    ) -> SamplePoint {
        let dt = after.time - before.time;
        let t = if dt > 0.0 {
            ((time - before.time) / dt).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Linear interpolation for position
        let position = Vector3::new(
            before.position.x + (after.position.x - before.position.x) * t,
            before.position.y + (after.position.y - before.position.y) * t,
            before.position.z + (after.position.z - before.position.z) * t,
        );

        // Spherical interpolation for orientation (simplified)
        let orientation = self.slerp_quaternion(&before.orientation, &after.orientation, t);

        // Interpolate velocity if available
        let velocity = match (before.velocity, after.velocity) {
            (Some(v1), Some(v2)) => Some(Vector3::new(
                v1.x + (v2.x - v1.x) * t,
                v1.y + (v2.y - v1.y) * t,
                v1.z + (v2.z - v1.z) * t,
            )),
            (Some(v), None) | (None, Some(v)) => Some(v),
            (None, None) => None,
        };

        SamplePoint {
            object_id: id.clone(),
            time,
            position,
            orientation,
            velocity,
        }
    }

    fn extrapolate_forward(
        &self,
        id: &ObjectId,
        snapshot: &StateSnapshot,
        time: f64,
    ) -> SamplePoint {
        let dt = time - snapshot.time;

        // Extrapolate using velocity if available
        let position = if let Some(velocity) = snapshot.velocity {
            Vector3::new(
                snapshot.position.x + velocity.x * dt,
                snapshot.position.y + velocity.y * dt,
                snapshot.position.z + velocity.z * dt,
            )
        } else {
            snapshot.position
        };

        SamplePoint {
            object_id: id.clone(),
            time,
            position,
            orientation: snapshot.orientation,
            velocity: snapshot.velocity,
        }
    }

    fn slerp_quaternion(&self, q1: &Quaternion, q2: &Quaternion, t: f64) -> Quaternion {
        // Simplified SLERP - proper implementation would handle dot product sign
        let dot = q1.w * q2.w + q1.x * q2.x + q1.y * q2.y + q1.z * q2.z;

        // Linear interpolation for small angles (faster, good enough for small dt)
        if dot.abs() > 0.9995 {
            return Quaternion::new(
                q1.w + (q2.w - q1.w) * t,
                q1.x + (q2.x - q1.x) * t,
                q1.y + (q2.y - q1.y) * t,
                q1.z + (q2.z - q1.z) * t,
            )
            .normalize();
        }

        // Spherical interpolation
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();

        if sin_theta.abs() < 1e-6 {
            return *q1; // Quaternions too close
        }

        let w1 = ((1.0 - t) * theta).sin() / sin_theta;
        let w2 = (t * theta).sin() / sin_theta;

        Quaternion::new(
            q1.w * w1 + q2.w * w2,
            q1.x * w1 + q2.x * w2,
            q1.y * w1 + q2.y * w2,
            q1.z * w1 + q2.z * w2,
        )
        .normalize()
    }

    /// Clear history for an object
    pub fn clear_object(&mut self, id: &ObjectId) {
        self.history.remove(id);
    }

    /// Clear all history
    pub fn clear_all(&mut self) {
        self.history.clear();
    }

    /// Get number of snapshots for an object
    pub fn snapshot_count(&self, id: &ObjectId) -> usize {
        self.history.get(id).map(|h| h.len()).unwrap_or(0)
    }

    /// Get total number of snapshots
    pub fn total_snapshots(&self) -> usize {
        self.history.values().map(|h| h.len()).sum()
    }
}

impl Default for MotionSampler {
    fn default() -> Self {
        Self::new()
    }
}
