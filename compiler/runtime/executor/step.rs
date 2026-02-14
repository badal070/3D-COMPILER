// runtime/executor/step.rs
// Defines a single execution step
// Small steps prevent big disasters

use crate::error::RuntimeResult;
use crate::state::RuntimeState;

/// A single execution step
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub kind: StepKind,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Initialize state
    Initialize,
    /// Apply constraints
    ApplyConstraints,
    /// Update motion/dynamics
    UpdateMotion,
    /// Validate state
    ValidateState,
    /// Synchronize derived properties
    Synchronize,
    /// Custom step
    Custom,
}

/// Result of executing a step
#[derive(Debug)]
pub struct StepResult {
    pub success: bool,
    pub state: RuntimeState,
    pub metrics: StepMetrics,
}

#[derive(Debug, Clone)]
pub struct StepMetrics {
    /// Time taken to execute step (microseconds)
    pub execution_time_us: u64,
    /// Number of iterations (for solvers)
    pub iterations: usize,
    /// Residual error (for solvers)
    pub residual: f64,
    /// Number of objects updated
    pub objects_updated: usize,
    /// Number of constraints evaluated
    pub constraints_evaluated: usize,
}

impl Default for StepMetrics {
    fn default() -> Self {
        Self {
            execution_time_us: 0,
            iterations: 0,
            residual: 0.0,
            objects_updated: 0,
            constraints_evaluated: 0,
        }
    }
}

impl ExecutionStep {
    pub fn new(kind: StepKind, description: impl Into<String>) -> Self {
        Self {
            kind,
            description: description.into(),
        }
    }

    pub fn initialize() -> Self {
        Self::new(StepKind::Initialize, "Initialize state")
    }

    pub fn apply_constraints() -> Self {
        Self::new(StepKind::ApplyConstraints, "Apply constraints")
    }

    pub fn update_motion() -> Self {
        Self::new(StepKind::UpdateMotion, "Update motion")
    }

    pub fn validate_state() -> Self {
        Self::new(StepKind::ValidateState, "Validate state")
    }

    pub fn synchronize() -> Self {
        Self::new(StepKind::Synchronize, "Synchronize derived properties")
    }
}

impl StepResult {
    pub fn success(state: RuntimeState, metrics: StepMetrics) -> Self {
        Self {
            success: true,
            state,
            metrics,
        }
    }

    pub fn failure(state: RuntimeState, metrics: StepMetrics) -> Self {
        Self {
            success: false,
            state,
            metrics,
        }
    }
}

/// Execution pipeline - ordered sequence of steps
pub struct ExecutionPipeline {
    steps: Vec<ExecutionStep>,
}

impl ExecutionPipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(&mut self, step: ExecutionStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[ExecutionStep] {
        &self.steps
    }

    /// Standard pipeline: constraints → motion → validate
    pub fn standard() -> Self {
        let mut pipeline = Self::new();
        pipeline.add_step(ExecutionStep::apply_constraints());
        pipeline.add_step(ExecutionStep::update_motion());
        pipeline.add_step(ExecutionStep::validate_state());
        pipeline
    }

    /// Initialize pipeline: init → constraints → sync
    pub fn initialize() -> Self {
        let mut pipeline = Self::new();
        pipeline.add_step(ExecutionStep::initialize());
        pipeline.add_step(ExecutionStep::apply_constraints());
        pipeline.add_step(ExecutionStep::synchronize());
        pipeline
    }
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        Self::new()
    }
}
