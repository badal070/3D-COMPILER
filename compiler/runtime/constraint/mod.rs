// runtime/constraint/mod.rs
// Constraint runtime system

pub mod enforcement;
pub mod failure;
pub mod solver;

pub use enforcement::{ConstraintEnforcer, EnforcementResult};
pub use failure::{FailureAction, FailureClassifier};
pub use solver::{ConstraintSolver, SolverConfig, SolverResult};

use crate::error::RuntimeResult;
use crate::state::WorldState;

/// Constraint system - evaluates and enforces constraints
pub struct ConstraintSystem {
    solver: ConstraintSolver,
    enforcer: ConstraintEnforcer,
    classifier: FailureClassifier,
}

impl ConstraintSystem {
    pub fn new(config: SolverConfig) -> Self {
        Self {
            solver: ConstraintSolver::new(config),
            enforcer: ConstraintEnforcer::new(),
            classifier: FailureClassifier::new(),
        }
    }

    /// Solve constraints and enforce corrections
    pub fn solve_and_enforce(&mut self, state: &mut WorldState) -> RuntimeResult<SolverResult> {
        // Solve constraints
        let result = self.solver.solve(state)?;

        // If solving succeeded, enforce corrections
        if result.converged {
            self.enforcer.enforce(&result, state)?;
        } else {
            // Classify failure and determine action
            let action = self.classifier.classify(&result);
            match action {
                FailureAction::Retry => {
                    // Could retry with relaxed tolerance
                }
                FailureAction::Reject => {
                    return Err(crate::error::RuntimeError::ConstraintFailure(
                        crate::error::ConstraintError {
                            kind: crate::error::ConstraintErrorKind::NoConvergence,
                            constraint_id: None,
                            iteration: result.iterations,
                            residual: result.residual,
                        },
                    ));
                }
                FailureAction::Accept => {
                    // Accept partial solution
                    self.enforcer.enforce(&result, state)?;
                }
            }
        }

        Ok(result)
    }

    pub fn solver(&self) -> &ConstraintSolver {
        &self.solver
    }

    pub fn solver_mut(&mut self) -> &mut ConstraintSolver {
        &mut self.solver
    }
}

impl Default for ConstraintSystem {
    fn default() -> Self {
        Self::new(SolverConfig::default())
    }
}
