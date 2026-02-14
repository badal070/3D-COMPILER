// runtime/constraint/solver.rs
// Evaluates constraint equations
// Iterative or direct solving
// Deterministic order
// No heuristics unless proven safe

use crate::error::{ConstraintError, ConstraintErrorKind, RuntimeError, RuntimeResult};
use crate::state::world_state::{ActiveConstraint, ConstraintKind};
use crate::state::{ObjectId, Quaternion, Vector3, WorldState};
use std::collections::HashMap;

/// Constraint solver configuration
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// Convergence tolerance
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Solver method
    pub method: SolverMethod,
    /// Relaxation factor (for iterative methods)
    pub relaxation: f64,
    /// Enable line search
    pub line_search: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverMethod {
    /// Gauss-Seidel iteration
    GaussSeidel,
    /// Jacobi iteration
    Jacobi,
    /// Newton-Raphson
    Newton,
    /// Gradient descent
    GradientDescent,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 100,
            method: SolverMethod::GaussSeidel,
            relaxation: 1.0,
            line_search: false,
        }
    }
}

/// Constraint solver
pub struct ConstraintSolver {
    config: SolverConfig,
    /// Constraint evaluation cache
    cache: HashMap<String, f64>,
}

impl ConstraintSolver {
    pub fn new(config: SolverConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    /// Solve constraints
    pub fn solve(&mut self, state: &WorldState) -> RuntimeResult<SolverResult> {
        self.cache.clear();

        let mut result = SolverResult {
            converged: false,
            iterations: 0,
            residual: f64::INFINITY,
            corrections: HashMap::new(),
        };

        // Get enabled constraints sorted by priority
        let constraints: Vec<_> = state.enabled_constraints().collect();
        if constraints.is_empty() {
            result.converged = true;
            result.residual = 0.0;
            return Ok(result);
        }

        // Iterative solving
        for iteration in 0..self.config.max_iterations {
            result.iterations = iteration + 1;

            let mut max_residual = 0.0;
            let mut all_satisfied = true;

            // Evaluate each constraint
            for constraint in &constraints {
                let residual = self.evaluate_constraint(constraint, state)?;
                max_residual = max_residual.max(residual.abs());

                if residual.abs() > self.config.tolerance {
                    all_satisfied = false;

                    // Compute correction
                    let correction = self.compute_correction(constraint, state, residual)?;

                    // Store correction
                    for (obj_id, delta) in correction {
                        result
                            .corrections
                            .entry(obj_id)
                            .or_insert_with(Vec::new)
                            .push(delta);
                    }
                }
            }

            result.residual = max_residual;

            // Check convergence
            if all_satisfied {
                result.converged = true;
                break;
            }

            // Check for divergence
            if max_residual.is_nan() || max_residual.is_infinite() {
                return Err(RuntimeError::ConstraintFailure(ConstraintError {
                    kind: ConstraintErrorKind::Unstable,
                    constraint_id: None,
                    iteration,
                    residual: max_residual,
                }));
            }
        }

        Ok(result)
    }

    fn evaluate_constraint(
        &self,
        constraint: &ActiveConstraint,
        state: &WorldState,
    ) -> RuntimeResult<f64> {
        match constraint.kind {
            // Distance between two objects should match target distance
            ConstraintKind::Distance => {
                if constraint.objects.len() < 2 || constraint.parameters.is_empty() {
                    return Ok(0.0);
                }

                let a_id = &constraint.objects[0];
                let b_id = &constraint.objects[1];
                let param_id = &constraint.parameters[0];

                let a = match state.get_object(a_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };
                let b = match state.get_object(b_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };

                let target = match state.parameters.get(param_id) {
                    Some(v) => v,
                    None => return Ok(0.0),
                };

                let dx = b.position.x - a.position.x;
                let dy = b.position.y - a.position.y;
                let dz = b.position.z - a.position.z;
                let current = (dx * dx + dy * dy + dz * dz).sqrt();

                Ok(current - target)
            }

            // Angle relation (used for gear relations)
            ConstraintKind::Angle => {
                if constraint.objects.len() < 2 || constraint.parameters.is_empty() {
                    return Ok(0.0);
                }

                let driver_id = &constraint.objects[0];
                let driven_id = &constraint.objects[1];
                let param_id = &constraint.parameters[0];

                let driver = match state.get_object(driver_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };
                let driven = match state.get_object(driven_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };

                let ratio = match state.parameters.get(param_id) {
                    Some(v) => v,
                    None => return Ok(0.0),
                };

                let angle_driver = yaw_from_quaternion(&driver.orientation);
                let angle_driven = yaw_from_quaternion(&driven.orientation);

                Ok(angle_driven - ratio * angle_driver)
            }

            // Equality constraint (used for fixed joints)
            ConstraintKind::Equality => {
                if constraint.objects.len() < 2 {
                    return Ok(0.0);
                }

                let parent_id = &constraint.objects[0];
                let child_id = &constraint.objects[1];

                let parent = match state.get_object(parent_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };
                let child = match state.get_object(child_id) {
                    Some(o) => o,
                    None => return Ok(0.0),
                };

                let dx = child.position.x - parent.position.x;
                let dy = child.position.y - parent.position.y;
                let dz = child.position.z - parent.position.z;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                // Residual is simply the separation distance; solver will try to drive it to 0.
                Ok(distance)
            }

            // Other kinds are not yet interpreted
            _ => Ok(0.0),
        }
    }

    fn compute_correction(
        &self,
        constraint: &ActiveConstraint,
        state: &WorldState,
        residual: f64,
    ) -> RuntimeResult<HashMap<ObjectId, CorrectionDelta>> {
        let mut map = HashMap::new();

        match constraint.kind {
            // Pull two objects towards/away from each other along their connecting line
            ConstraintKind::Distance => {
                if constraint.objects.len() < 2 || constraint.parameters.is_empty() {
                    return Ok(map);
                }

                let a_id = &constraint.objects[0];
                let b_id = &constraint.objects[1];
                let _param_id = &constraint.parameters[0];

                let a = match state.get_object(a_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };
                let b = match state.get_object(b_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };

                let dx = b.position.x - a.position.x;
                let dy = b.position.y - a.position.y;
                let dz = b.position.z - a.position.z;
                let current = (dx * dx + dy * dy + dz * dz).sqrt();

                if current == 0.0 {
                    return Ok(map);
                }

                // Direction from A to B
                let dir = Vector3::new(dx / current, dy / current, dz / current);

                // Simple symmetric correction: move each object half the residual
                let step = -0.5 * residual * self.config.relaxation;

                let delta_a = [dir.x * step, dir.y * step, dir.z * step];
                let delta_b = [-dir.x * step, -dir.y * step, -dir.z * step];

                map.insert(
                    a_id.clone(),
                    CorrectionDelta {
                        kind: CorrectionKind::Position,
                        value: CorrectableValue::Vector3(delta_a),
                    },
                );
                map.insert(
                    b_id.clone(),
                    CorrectionDelta {
                        kind: CorrectionKind::Position,
                        value: CorrectableValue::Vector3(delta_b),
                    },
                );
            }

            // For gear relations, adjust the driven gear's orientation to match the target ratio
            ConstraintKind::Angle => {
                if constraint.objects.len() < 2 || constraint.parameters.is_empty() {
                    return Ok(map);
                }

                let driver_id = &constraint.objects[0];
                let driven_id = &constraint.objects[1];
                let param_id = &constraint.parameters[0];

                let driver = match state.get_object(driver_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };
                let driven = match state.get_object(driven_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };

                let ratio = match state.parameters.get(param_id) {
                    Some(v) => v,
                    None => return Ok(map),
                };

                let angle_driver = yaw_from_quaternion(&driver.orientation);
                let angle_driven = yaw_from_quaternion(&driven.orientation);
                let desired_driven = ratio * angle_driver;

                // Residual we got is angle_driven - desired_driven
                let error = angle_driven - desired_driven;
                let delta_angle = -error * self.config.relaxation;

                let dq = quaternion_from_yaw(delta_angle);

                map.insert(
                    driven_id.clone(),
                    CorrectionDelta {
                        kind: CorrectionKind::Orientation,
                        value: CorrectableValue::Quaternion([dq.w, dq.x, dq.y, dq.z]),
                    },
                );
            }

            // For fixed joints, move the child toward the parent position
            ConstraintKind::Equality => {
                if constraint.objects.len() < 2 {
                    return Ok(map);
                }

                let parent_id = &constraint.objects[0];
                let child_id = &constraint.objects[1];

                let parent = match state.get_object(parent_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };
                let child = match state.get_object(child_id) {
                    Some(o) => o,
                    None => return Ok(map),
                };

                let dx = parent.position.x - child.position.x;
                let dy = parent.position.y - child.position.y;
                let dz = parent.position.z - child.position.z;

                let delta = [
                    dx * self.config.relaxation,
                    dy * self.config.relaxation,
                    dz * self.config.relaxation,
                ];

                map.insert(
                    child_id.clone(),
                    CorrectionDelta {
                        kind: CorrectionKind::Position,
                        value: CorrectableValue::Vector3(delta),
                    },
                );
            }

            _ => {}
        }

        Ok(map)
    }

    pub fn config(&self) -> &SolverConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut SolverConfig {
        &mut self.config
    }
}

/// Extract a yaw angle (rotation around Z axis) from a unit quaternion.
fn yaw_from_quaternion(q: &Quaternion) -> f64 {
    // Standard yaw extraction from quaternion
    let siny_cosp = 2.0 * (q.w * q.z + q.x * q.y);
    let cosy_cosp = 1.0 - 2.0 * (q.y * q.y + q.z * q.z);
    siny_cosp.atan2(cosy_cosp)
}

/// Build a quaternion representing a rotation of `angle` radians around Z.
fn quaternion_from_yaw(angle: f64) -> Quaternion {
    let half = angle * 0.5;
    let s = half.sin();
    let c = half.cos();
    Quaternion::new(c, 0.0, 0.0, s)
}

/// Result of constraint solving
#[derive(Debug, Clone)]
pub struct SolverResult {
    /// Did the solver converge
    pub converged: bool,
    /// Number of iterations performed
    pub iterations: usize,
    /// Final residual
    pub residual: f64,
    /// Corrections to apply to objects
    pub corrections: HashMap<ObjectId, Vec<CorrectionDelta>>,
}

/// A correction to apply to an object
#[derive(Debug, Clone)]
pub struct CorrectionDelta {
    pub kind: CorrectionKind,
    pub value: CorrectableValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionKind {
    Position,
    Orientation,
    Scale,
    Parameter,
}

#[derive(Debug, Clone)]
pub enum CorrectableValue {
    Vector3([f64; 3]),
    Quaternion([f64; 4]),
    Scalar(f64),
}

impl SolverResult {
    pub fn is_success(&self) -> bool {
        self.converged && self.residual < 1e-6
    }

    pub fn object_count(&self) -> usize {
        self.corrections.len()
    }

    pub fn total_corrections(&self) -> usize {
        self.corrections.values().map(|v| v.len()).sum()
    }
}
