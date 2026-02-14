// runtime/state/world_state.rs
// Global immutable + mutable state
// Object registry, active constraints, execution flags
// Must be serializable. No exceptions.

use super::{ObjectId, ObjectState, ParameterState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// All objects in the world
    pub objects: HashMap<ObjectId, ObjectState>,
    /// Global and local parameters
    pub parameters: ParameterState,
    /// Active constraints
    pub constraints: Vec<ActiveConstraint>,
    /// Execution flags
    pub flags: ExecutionFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConstraint {
    pub id: String,
    pub kind: ConstraintKind,
    pub objects: Vec<ObjectId>,
    pub parameters: Vec<String>,
    /// Constraint equation (symbolic or compiled)
    pub equation: String,
    /// Priority for conflict resolution (higher = more important)
    pub priority: i32,
    /// Whether constraint is currently enabled
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Equality constraint: f(x) = 0
    Equality,
    /// Inequality constraint: f(x) >= 0
    Inequality,
    /// Distance constraint
    Distance,
    /// Angle constraint
    Angle,
    /// Custom constraint
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFlags {
    /// Is execution paused
    pub paused: bool,
    /// Is stepping mode enabled
    pub stepping: bool,
    /// Should validate state after each step
    pub validate_steps: bool,
    /// Should record snapshots
    pub record_snapshots: bool,
    /// Custom flags
    pub custom: HashMap<String, bool>,
}

impl Default for ExecutionFlags {
    fn default() -> Self {
        Self {
            paused: false,
            stepping: false,
            validate_steps: true,
            record_snapshots: true,
            custom: HashMap::new(),
        }
    }
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            parameters: ParameterState::new(),
            constraints: Vec::new(),
            flags: ExecutionFlags::default(),
        }
    }

    /// Add object to world
    pub fn add_object(&mut self, id: ObjectId, state: ObjectState) -> Result<(), String> {
        if self.objects.contains_key(&id) {
            return Err(format!("Object {} already exists", id));
        }
        self.objects.insert(id, state);
        Ok(())
    }

    /// Remove object from world
    pub fn remove_object(&mut self, id: &ObjectId) -> Result<ObjectState, String> {
        self.objects
            .remove(id)
            .ok_or_else(|| format!("Object {} not found", id))
    }

    /// Get object state
    pub fn get_object(&self, id: &ObjectId) -> Option<&ObjectState> {
        self.objects.get(id)
    }

    /// Get mutable object state
    pub fn get_object_mut(&mut self, id: &ObjectId) -> Option<&mut ObjectState> {
        self.objects.get_mut(id)
    }

    /// Add constraint
    pub fn add_constraint(&mut self, constraint: ActiveConstraint) {
        self.constraints.push(constraint);
        // Sort by priority (higher first)
        self.constraints.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove constraint
    pub fn remove_constraint(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .constraints
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| format!("Constraint {} not found", id))?;
        self.constraints.remove(index);
        Ok(())
    }

    /// Get enabled constraints
    pub fn enabled_constraints(&self) -> impl Iterator<Item = &ActiveConstraint> {
        self.constraints.iter().filter(|c| c.enabled)
    }

    /// Validate world state
    pub fn validate(&self) -> Result<(), String> {
        // Validate all objects
        for (id, obj) in &self.objects {
            obj.validate()
                .map_err(|e| format!("Object {} invalid: {}", id, e))?;
        }

        // Validate parameters
        self.parameters.validate()?;

        // Validate constraints reference valid objects
        for constraint in &self.constraints {
            for obj_id in &constraint.objects {
                if !self.objects.contains_key(obj_id) {
                    return Err(format!(
                        "Constraint {} references unknown object {}",
                        constraint.id, obj_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check for NaN in any state
    pub fn has_nan(&self) -> bool {
        self.objects.values().any(|obj| obj.has_nan()) || self.parameters.has_nan()
    }

    /// Check for infinity in any state
    pub fn has_infinity(&self) -> bool {
        self.objects.values().any(|obj| obj.has_infinity()) || self.parameters.has_infinity()
    }

    /// Get state summary for debugging
    pub fn summary(&self) -> WorldStateSummary {
        WorldStateSummary {
            object_count: self.objects.len(),
            parameter_count: self.parameters.values().len(),
            constraint_count: self.constraints.len(),
            enabled_constraint_count: self.enabled_constraints().count(),
            has_nan: self.has_nan(),
            has_infinity: self.has_infinity(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldStateSummary {
    pub object_count: usize,
    pub parameter_count: usize,
    pub constraint_count: usize,
    pub enabled_constraint_count: usize,
    pub has_nan: bool,
    pub has_infinity: bool,
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}
