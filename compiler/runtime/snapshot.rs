// runtime/snapshot.rs
// Saves runtime state, enables rewind, comparison, debugging
// Snapshots are how learning becomes inspectable

use crate::state::RuntimeState;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A snapshot of runtime state at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique identifier
    pub id: u64,
    /// Simulation time when snapshot was taken
    pub time: f64,
    /// Real time when snapshot was taken (microseconds)
    pub timestamp: u64,
    /// Complete runtime state
    pub state: RuntimeState,
    /// Optional label for this snapshot
    pub label: Option<String>,
    /// Metadata
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Step count when snapshot was taken
    pub step: u64,
    /// Objects in snapshot
    pub object_count: usize,
    /// Parameters in snapshot
    pub parameter_count: usize,
    /// Estimated memory size (bytes)
    pub size_bytes: usize,
}

/// Snapshot history manager
pub struct SnapshotHistory {
    snapshots: VecDeque<Snapshot>,
    max_snapshots: usize,
    next_id: u64,
}

impl SnapshotHistory {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots,
            next_id: 0,
        }
    }

    /// Take a snapshot of current state
    pub fn take_snapshot(&mut self, state: RuntimeState, label: Option<String>) -> &Snapshot {
        let id = self.next_id;
        self.next_id += 1;

        let metadata = SnapshotMetadata {
            step: state.time.step_count,
            object_count: state.world.objects.len(),
            parameter_count: state.world.parameters.values().len(),
            size_bytes: self.estimate_size(&state),
        };

        let snapshot = Snapshot {
            id,
            time: state.time.current_time,
            timestamp: Self::current_timestamp(),
            state,
            label,
            metadata,
        };

        // Add to history
        self.snapshots.push_back(snapshot);

        // Trim if exceeds limit
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }

        // Return reference to newly added snapshot
        self.snapshots.back().unwrap()
    }

    /// Get snapshot by ID
    pub fn get(&self, id: u64) -> Option<&Snapshot> {
        self.snapshots.iter().find(|s| s.id == id)
    }

    /// Get most recent snapshot
    pub fn latest(&self) -> Option<&Snapshot> {
        self.snapshots.back()
    }

    /// Get snapshot at specific time (or closest)
    pub fn at_time(&self, time: f64) -> Option<&Snapshot> {
        self.snapshots.iter().min_by(|a, b| {
            let diff_a = (a.time - time).abs();
            let diff_b = (b.time - time).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
    }

    /// Get all snapshots
    pub fn all(&self) -> impl Iterator<Item = &Snapshot> {
        self.snapshots.iter()
    }

    /// Get snapshots in time range
    pub fn in_range(&self, start: f64, end: f64) -> impl Iterator<Item = &Snapshot> {
        self.snapshots
            .iter()
            .filter(move |s| s.time >= start && s.time <= end)
    }

    /// Get snapshots with label
    pub fn with_label(&self, label: &str) -> impl Iterator<Item = &Snapshot> {
        self.snapshots
            .iter()
            .filter(move |s| s.label.as_deref() == Some(label))
    }

    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }

    /// Remove oldest snapshots to stay under limit
    pub fn trim_to_limit(&mut self, limit: usize) {
        while self.snapshots.len() > limit {
            self.snapshots.pop_front();
        }
    }

    /// Get count of snapshots
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// Get total memory used by snapshots (estimated)
    pub fn total_size_bytes(&self) -> usize {
        self.snapshots.iter().map(|s| s.metadata.size_bytes).sum()
    }

    /// Get history statistics
    pub fn stats(&self) -> HistoryStats {
        let total_size = self.total_size_bytes();
        let time_span =
            if let (Some(first), Some(last)) = (self.snapshots.front(), self.snapshots.back()) {
                last.time - first.time
            } else {
                0.0
            };

        HistoryStats {
            snapshot_count: self.snapshots.len(),
            max_snapshots: self.max_snapshots,
            total_size_bytes: total_size,
            time_span,
            average_size_bytes: if !self.snapshots.is_empty() {
                total_size / self.snapshots.len()
            } else {
                0
            },
        }
    }

    fn estimate_size(&self, state: &RuntimeState) -> usize {
        // Rough estimate of serialized size
        // Object state: ~200 bytes each
        // Parameter: ~50 bytes each
        // Constraint: ~100 bytes each
        let object_size = state.world.objects.len() * 200;
        let param_size = state.world.parameters.values().len() * 50;
        let constraint_size = state.world.constraints.len() * 100;
        object_size + param_size + constraint_size + 1000 // + overhead
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}

#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub snapshot_count: usize,
    pub max_snapshots: usize,
    pub total_size_bytes: usize,
    pub time_span: f64,
    pub average_size_bytes: usize,
}

impl HistoryStats {
    pub fn utilization(&self) -> f64 {
        if self.max_snapshots > 0 {
            self.snapshot_count as f64 / self.max_snapshots as f64
        } else {
            0.0
        }
    }

    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn average_size_kb(&self) -> f64 {
        self.average_size_bytes as f64 / 1024.0
    }
}

/// Snapshot comparison result
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub id_a: u64,
    pub id_b: u64,
    pub time_diff: f64,
    pub objects_added: Vec<String>,
    pub objects_removed: Vec<String>,
    pub objects_modified: Vec<String>,
    pub parameters_changed: Vec<String>,
}

impl Snapshot {
    /// Compare with another snapshot
    pub fn diff(&self, other: &Snapshot) -> SnapshotDiff {
        let mut objects_added = Vec::new();
        let mut objects_removed = Vec::new();
        let mut objects_modified = Vec::new();

        // Find added/removed/modified objects
        for id in self.state.world.objects.keys() {
            if !other.state.world.objects.contains_key(id) {
                objects_removed.push(id.clone());
            } else {
                // Check if modified (simplified - would check actual values)
                objects_modified.push(id.clone());
            }
        }

        for id in other.state.world.objects.keys() {
            if !self.state.world.objects.contains_key(id) {
                objects_added.push(id.clone());
            }
        }

        // Find changed parameters
        let mut parameters_changed = Vec::new();
        for (key, value) in self.state.world.parameters.values() {
            if let Some(other_value) = other.state.world.parameters.get(&key) {
                if (value - other_value).abs() > 1e-10 {
                    parameters_changed.push(key);
                }
            }
        }

        SnapshotDiff {
            id_a: self.id,
            id_b: other.id,
            time_diff: other.time - self.time,
            objects_added,
            objects_removed,
            objects_modified,
            parameters_changed,
        }
    }
}
