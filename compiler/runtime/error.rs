// runtime/error.rs
// Structured, typed, explainable errors
// Never panic. Panic is amateur hour.

use std::fmt;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Constraint solving failed
    ConstraintFailure(ConstraintError),
    /// Integration produced invalid state
    IntegrationFailure(IntegrationError),
    /// Watchdog triggered
    WatchdogTriggered(WatchdogError),
    /// Invalid state detected
    InvalidState(StateError),
    /// Execution plan invalid
    InvalidPlan(String),
    /// Resource limit exceeded
    ResourceLimit(String),
    /// Configuration error
    Configuration(String),
    /// Internal logic error (should never happen)
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct ConstraintError {
    pub kind: ConstraintErrorKind,
    pub constraint_id: Option<String>,
    pub iteration: usize,
    pub residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintErrorKind {
    /// Constraints are contradictory
    Conflict,
    /// Failed to converge within iteration limit
    NoConvergence,
    /// Numerical instability detected
    Unstable,
    /// Constraint equation evaluation failed
    EvaluationFailed,
}

#[derive(Debug, Clone)]
pub struct IntegrationError {
    pub kind: IntegrationErrorKind,
    pub time: f64,
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationErrorKind {
    /// NaN detected in state
    NaN,
    /// Infinity detected in state
    Infinity,
    /// Step size became too small
    StepTooSmall,
    /// Stability limit exceeded
    Unstable,
}

#[derive(Debug, Clone)]
pub struct WatchdogError {
    pub kind: WatchdogErrorKind,
    pub limit: u64,
    pub actual: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogErrorKind {
    /// Exceeded step count
    StepLimit,
    /// Exceeded execution time
    TimeLimit,
    /// Exceeded memory usage
    MemoryLimit,
}

#[derive(Debug, Clone)]
pub struct StateError {
    pub kind: StateErrorKind,
    pub object_id: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateErrorKind {
    /// Object state is invalid
    InvalidObject,
    /// Parameter out of valid range
    InvalidParameter,
    /// Time state is inconsistent
    InvalidTime,
    /// State violates invariant
    InvariantViolation,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::ConstraintFailure(e) => write!(f, "Constraint failure: {}", e),
            RuntimeError::IntegrationFailure(e) => write!(f, "Integration failure: {}", e),
            RuntimeError::WatchdogTriggered(e) => write!(f, "Watchdog triggered: {}", e),
            RuntimeError::InvalidState(e) => write!(f, "Invalid state: {}", e),
            RuntimeError::InvalidPlan(msg) => write!(f, "Invalid execution plan: {}", msg),
            RuntimeError::ResourceLimit(msg) => write!(f, "Resource limit exceeded: {}", msg),
            RuntimeError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            RuntimeError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at iteration {} (residual: {:.2e})",
            self.kind, self.iteration, self.residual
        )?;
        if let Some(id) = &self.constraint_id {
            write!(f, " [constraint: {}]", id)?;
        }
        Ok(())
    }
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at time {:.4}", self.kind, self.time)?;
        if let Some(id) = &self.object_id {
            write!(f, " [object: {}]", id)?;
        }
        Ok(())
    }
}

impl fmt::Display for WatchdogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: limit={}, actual={}",
            self.kind, self.limit, self.actual
        )
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.details)?;
        if let Some(id) = &self.object_id {
            write!(f, " [object: {}]", id)?;
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

impl From<String> for RuntimeError {
    fn from(s: String) -> Self {
        RuntimeError::Internal(s)
    }
}

/// Classify error recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRecovery {
    /// Error is recoverable, can pause and inspect
    Recoverable,
    /// Error is fatal, must reset
    Fatal,
    /// Error requires user intervention
    RequiresIntervention,
}

impl RuntimeError {
    pub fn recovery_strategy(&self) -> ErrorRecovery {
        match self {
            RuntimeError::ConstraintFailure(e) => match e.kind {
                ConstraintErrorKind::Conflict => ErrorRecovery::RequiresIntervention,
                ConstraintErrorKind::NoConvergence => ErrorRecovery::Recoverable,
                ConstraintErrorKind::Unstable => ErrorRecovery::Fatal,
                ConstraintErrorKind::EvaluationFailed => ErrorRecovery::Fatal,
            },
            RuntimeError::IntegrationFailure(_) => ErrorRecovery::Fatal,
            RuntimeError::WatchdogTriggered(_) => ErrorRecovery::Recoverable,
            RuntimeError::InvalidState(_) => ErrorRecovery::Fatal,
            RuntimeError::InvalidPlan(_) => ErrorRecovery::RequiresIntervention,
            RuntimeError::ResourceLimit(_) => ErrorRecovery::Recoverable,
            RuntimeError::Configuration(_) => ErrorRecovery::RequiresIntervention,
            RuntimeError::Internal(_) => ErrorRecovery::Fatal,
        }
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self.recovery_strategy(), ErrorRecovery::Fatal)
    }
}
