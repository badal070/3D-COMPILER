// runtime/constraint/enforcement.rs
// Applies constraint corrections
// Maintains invariants
// Rejects invalid states
// If constraints conflict, execution stops. No lying.

use super::solver::{CorrectableValue, CorrectionDelta, CorrectionKind, SolverResult};
use crate::error::{RuntimeError, RuntimeResult, StateError, StateErrorKind};
use crate::state::{ObjectState, Quaternion, Vector3, WorldState};

/// Constraint enforcer
pub struct ConstraintEnforcer {
    /// Damping factor for corrections
    damping: f64,
    /// Validate after enforcement
    validate: bool,
}

impl ConstraintEnforcer {
    pub fn new() -> Self {
        Self {
            damping: 1.0,
            validate: true,
        }
    }

    pub fn with_damping(mut self, damping: f64) -> Self {
        self.damping = damping.clamp(0.0, 1.0);
        self
    }

    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Enforce constraint corrections
    pub fn enforce(
        &self,
        result: &SolverResult,
        state: &mut WorldState,
    ) -> RuntimeResult<EnforcementResult> {
        let mut enforcement_result = EnforcementResult {
            objects_corrected: 0,
            total_corrections: 0,
            max_correction: 0.0,
        };

        // Apply corrections to each object
        for (obj_id, corrections) in &result.corrections {
            let object = state.get_object_mut(obj_id).ok_or_else(|| {
                RuntimeError::InvalidState(StateError {
                    kind: StateErrorKind::InvalidObject,
                    object_id: Some(obj_id.clone()),
                    details: "Object not found".to_string(),
                })
            })?;

            // Apply each correction
            for correction in corrections {
                let correction_magnitude = self.apply_correction(object, correction)?;
                enforcement_result.max_correction =
                    enforcement_result.max_correction.max(correction_magnitude);
                enforcement_result.total_corrections += 1;
            }

            enforcement_result.objects_corrected += 1;
        }

        // Validate state if enabled
        if self.validate {
            state.validate().map_err(|e| {
                RuntimeError::InvalidState(StateError {
                    kind: StateErrorKind::InvariantViolation,
                    object_id: None,
                    details: e,
                })
            })?;
        }

        // Check for NaN propagation
        if state.has_nan() {
            return Err(RuntimeError::InvalidState(StateError {
                kind: StateErrorKind::InvariantViolation,
                object_id: None,
                details: "NaN detected after constraint enforcement".to_string(),
            }));
        }

        Ok(enforcement_result)
    }

    fn apply_correction(
        &self,
        object: &mut ObjectState,
        correction: &CorrectionDelta,
    ) -> RuntimeResult<f64> {
        match (&correction.kind, &correction.value) {
            (CorrectionKind::Position, CorrectableValue::Vector3(delta)) => {
                let magnitude =
                    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                object.position.x += delta[0] * self.damping;
                object.position.y += delta[1] * self.damping;
                object.position.z += delta[2] * self.damping;
                Ok(magnitude)
            }
            (CorrectionKind::Orientation, CorrectableValue::Quaternion(delta)) => {
                // Apply quaternion delta (simplified - would use proper quaternion math)
                let magnitude = (delta[0] * delta[0]
                    + delta[1] * delta[1]
                    + delta[2] * delta[2]
                    + delta[3] * delta[3])
                    .sqrt();
                object.orientation.w += delta[0] * self.damping;
                object.orientation.x += delta[1] * self.damping;
                object.orientation.y += delta[2] * self.damping;
                object.orientation.z += delta[3] * self.damping;
                object.orientation = object.orientation.normalize();
                Ok(magnitude)
            }
            (CorrectionKind::Scale, CorrectableValue::Vector3(delta)) => {
                let magnitude =
                    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                object.scale.x += delta[0] * self.damping;
                object.scale.y += delta[1] * self.damping;
                object.scale.z += delta[2] * self.damping;
                Ok(magnitude)
            }
            (CorrectionKind::Parameter, CorrectableValue::Scalar(delta)) => {
                // Parameters are stored in WorldState, not ObjectState
                // This would need to be handled differently
                Ok(delta.abs())
            }
            _ => Err(RuntimeError::InvalidState(StateError {
                kind: StateErrorKind::InvariantViolation,
                object_id: Some(object.id.clone()),
                details: "Correction kind and value type mismatch".to_string(),
            })),
        }
    }
}

impl Default for ConstraintEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of enforcement
#[derive(Debug, Clone)]
pub struct EnforcementResult {
    /// Number of objects corrected
    pub objects_corrected: usize,
    /// Total number of corrections applied
    pub total_corrections: usize,
    /// Maximum correction magnitude
    pub max_correction: f64,
}

impl EnforcementResult {
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.max_correction > threshold
    }

    pub fn is_empty(&self) -> bool {
        self.total_corrections == 0
    }
}
