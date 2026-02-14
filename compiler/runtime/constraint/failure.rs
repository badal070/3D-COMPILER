// runtime/constraint/failure.rs
// Classifies failures: Recoverable (pause), Fatal (reset), Reject-and-report
// Clear failure taxonomy saves debugging years

use super::solver::SolverResult;

/// Failure classifier
pub struct FailureClassifier {
    /// Tolerance for accepting partial solutions
    partial_solution_tolerance: f64,
    /// Minimum iterations before giving up
    min_iterations: usize,
}

impl FailureClassifier {
    pub fn new() -> Self {
        Self {
            partial_solution_tolerance: 1e-3,
            min_iterations: 10,
        }
    }

    pub fn with_partial_tolerance(mut self, tolerance: f64) -> Self {
        self.partial_solution_tolerance = tolerance;
        self
    }

    /// Classify solver failure and determine action
    pub fn classify(&self, result: &SolverResult) -> FailureAction {
        // Already converged - not a failure
        if result.converged {
            return FailureAction::Accept;
        }

        // Check for catastrophic failure
        if result.residual.is_nan() || result.residual.is_infinite() {
            return FailureAction::Reject;
        }

        // Check if we gave it enough iterations
        if result.iterations < self.min_iterations {
            return FailureAction::Retry;
        }

        // Check if partial solution is acceptable
        if result.residual < self.partial_solution_tolerance {
            return FailureAction::Accept;
        }

        // Check if making progress
        if result.iterations >= self.min_iterations * 2 {
            // If we've done lots of iterations and still not converged,
            // probably won't converge
            return FailureAction::Reject;
        }

        // Default: retry with different parameters
        FailureAction::Retry
    }

    /// Get detailed failure analysis
    pub fn analyze(&self, result: &SolverResult) -> FailureAnalysis {
        let action = self.classify(result);

        let reason = if result.residual.is_nan() {
            FailureReason::NumericalInstability
        } else if result.residual.is_infinite() {
            FailureReason::Unbounded
        } else if result.iterations >= self.min_iterations * 2 {
            FailureReason::NoProgress
        } else if result.residual > self.partial_solution_tolerance {
            FailureReason::InsufficientIterations
        } else {
            FailureReason::ConflictingConstraints
        };

        FailureAnalysis {
            action,
            reason,
            iterations: result.iterations,
            residual: result.residual,
            suggestion: self.suggest_fix(&reason, result),
        }
    }

    fn suggest_fix(&self, reason: &FailureReason, result: &SolverResult) -> String {
        match reason {
            FailureReason::NumericalInstability => {
                "Reduce time step or use more stable integration method".to_string()
            }
            FailureReason::Unbounded => {
                "Constraints may be under-constrained. Add additional constraints.".to_string()
            }
            FailureReason::NoProgress => {
                format!(
                    "Solver not converging after {} iterations. Try different solver method or relax tolerance.",
                    result.iterations
                )
            }
            FailureReason::InsufficientIterations => "Increase maximum iteration count".to_string(),
            FailureReason::ConflictingConstraints => {
                "Constraints may be contradictory. Review constraint priorities and equations."
                    .to_string()
            }
        }
    }
}

impl Default for FailureClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Action to take on constraint failure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureAction {
    /// Retry with different parameters
    Retry,
    /// Reject the state entirely
    Reject,
    /// Accept partial solution
    Accept,
}

/// Detailed failure analysis
#[derive(Debug, Clone)]
pub struct FailureAnalysis {
    pub action: FailureAction,
    pub reason: FailureReason,
    pub iterations: usize,
    pub residual: f64,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// NaN or Inf detected
    NumericalInstability,
    /// Residual growing without bound
    Unbounded,
    /// Residual not decreasing
    NoProgress,
    /// Need more iterations
    InsufficientIterations,
    /// Constraints are contradictory
    ConflictingConstraints,
}

impl FailureAnalysis {
    pub fn is_recoverable(&self) -> bool {
        matches!(self.action, FailureAction::Retry | FailureAction::Accept)
    }

    pub fn is_fatal(&self) -> bool {
        !self.is_recoverable()
    }

    pub fn severity(&self) -> FailureSeverity {
        match self.reason {
            FailureReason::NumericalInstability => FailureSeverity::Critical,
            FailureReason::Unbounded => FailureSeverity::Critical,
            FailureReason::ConflictingConstraints => FailureSeverity::High,
            FailureReason::NoProgress => FailureSeverity::Medium,
            FailureReason::InsufficientIterations => FailureSeverity::Low,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for FailureAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Constraint failure: {:?} ({:?})\n  Iterations: {}\n  Residual: {:.2e}\n  Suggestion: {}",
            self.reason,
            self.severity(),
            self.iterations,
            self.residual,
            self.suggestion
        )
    }
}
