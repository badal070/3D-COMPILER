// runtime/executor/mod.rs
// Executor subsystem - executes the execution plan

pub mod stage_executor;
pub mod step;
pub mod watchdog;

pub use stage_executor::{ExecutionStage, StageExecutor};
pub use step::{ExecutionStep, StepResult};
pub use watchdog::Watchdog;

use crate::error::RuntimeResult;
use crate::state::RuntimeState;

/// Execution context passed through stages
#[derive(Debug)]
pub struct ExecutionContext {
    /// Current runtime state
    pub state: RuntimeState,
    /// Execution plan to follow
    pub plan: ExecutionPlan,
    /// Current step in plan
    pub current_step: usize,
    /// Watchdog for safety
    pub watchdog: Watchdog,
}

/// Execution plan - ordered stages to execute
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub stages: Vec<ExecutionStage>,
    pub description: String,
    pub highlight_schedule: Vec<crate::state::HighlightScheduleEntry>,
    pub annotations: Vec<crate::state::AnnotationEntry>,
}

impl ExecutionPlan {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            stages: Vec::new(),
            description: description.into(),
            highlight_schedule: Vec::new(),
            annotations: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: ExecutionStage) {
        self.stages.push(stage);
    }

    pub fn validate(&self) -> RuntimeResult<()> {
        if self.stages.is_empty() {
            return Err(crate::error::RuntimeError::InvalidPlan(
                "Execution plan has no stages".to_string(),
            ));
        }
        Ok(())
    }
}

impl ExecutionContext {
    pub fn new(state: RuntimeState, plan: ExecutionPlan, watchdog: Watchdog) -> Self {
        Self {
            state,
            plan,
            current_step: 0,
            watchdog,
        }
    }

    pub fn advance_step(&mut self) {
        self.current_step += 1;
    }

    pub fn is_complete(&self) -> bool {
        self.current_step >= self.plan.stages.len()
    }

    pub fn current_stage(&self) -> Option<&ExecutionStage> {
        self.plan.stages.get(self.current_step)
    }
}
