// runtime/executor/stage_executor.rs
// Executes: Init stage, Static solve, Dynamic update, Sync stage
// Stages match the execution plan exactly. No improvisation.

use super::{ExecutionStep, StepMetrics, StepResult};
use crate::error::{RuntimeError, RuntimeResult};
use crate::state::RuntimeState;
use std::time::Instant;

/// Execution stages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStage {
    /// Initialize all objects and parameters
    Init,
    /// Solve static constraints (one-time)
    StaticSolve,
    /// Dynamic update (time-stepping)
    DynamicUpdate,
    /// Synchronize derived properties
    Sync,
    /// Custom stage
    Custom(String),
}

/// Stage executor
pub struct StageExecutor {
    current_stage: Option<ExecutionStage>,
}

impl StageExecutor {
    pub fn new() -> Self {
        Self {
            current_stage: None,
        }
    }

    /// Execute a stage
    pub fn execute(
        &mut self,
        stage: ExecutionStage,
        state: RuntimeState,
    ) -> RuntimeResult<StepResult> {
        self.current_stage = Some(stage.clone());
        let start = Instant::now();

        let result = match stage {
            ExecutionStage::Init => self.execute_init(state),
            ExecutionStage::StaticSolve => self.execute_static_solve(state),
            ExecutionStage::DynamicUpdate => self.execute_dynamic_update(state),
            ExecutionStage::Sync => self.execute_sync(state),
            ExecutionStage::Custom(name) => {
                return Err(RuntimeError::InvalidPlan(format!(
                    "Custom stage '{}' not implemented",
                    name
                )))
            }
        };

        let elapsed = start.elapsed();
        let mut metrics = result
            .as_ref()
            .map(|r| r.metrics.clone())
            .unwrap_or_default();
        metrics.execution_time_us = elapsed.as_micros() as u64;

        result.map(|mut r| {
            r.metrics = metrics;
            r
        })
    }

    fn execute_init(&self, mut state: RuntimeState) -> RuntimeResult<StepResult> {
        // Validate initial state
        state.validate().map_err(|e| {
            RuntimeError::InvalidState(crate::error::StateError {
                kind: crate::error::StateErrorKind::InvariantViolation,
                object_id: None,
                details: e,
            })
        })?;

        // Initialize time
        if state.time.current_time != state.time.bounds.min {
            state.time.current_time = state.time.bounds.min;
        }

        let metrics = StepMetrics {
            objects_updated: state.world.objects.len(),
            ..Default::default()
        };

        Ok(StepResult::success(state, metrics))
    }

    fn execute_static_solve(&self, state: RuntimeState) -> RuntimeResult<StepResult> {
        // For static solve, we apply constraints once without time evolution
        // This is typically used for initial positioning

        let mut metrics = StepMetrics::default();
        metrics.constraints_evaluated = state.world.enabled_constraints().count();

        // Static constraints should be solved in constraint module
        // Here we just validate the result
        state.validate().map_err(|e| {
            RuntimeError::InvalidState(crate::error::StateError {
                kind: crate::error::StateErrorKind::InvariantViolation,
                object_id: None,
                details: e,
            })
        })?;

        Ok(StepResult::success(state, metrics))
    }

    fn execute_dynamic_update(&self, state: RuntimeState) -> RuntimeResult<StepResult> {
        // Dynamic update advances time and updates object states
        // Motion integration happens in motion module

        let mut metrics = StepMetrics {
            objects_updated: state
                .world
                .objects
                .values()
                .filter(|obj| !obj.is_static)
                .count(),
            ..Default::default()
        };

        // Validate after update
        state.validate().map_err(|e| {
            RuntimeError::InvalidState(crate::error::StateError {
                kind: crate::error::StateErrorKind::InvariantViolation,
                object_id: None,
                details: e,
            })
        })?;

        Ok(StepResult::success(state, metrics))
    }

    fn execute_sync(&self, mut state: RuntimeState) -> RuntimeResult<StepResult> {
        // Synchronize derived properties
        // This updates all derived parameters based on current state

        let mut updated = 0;

        // Update derived parameters
        for param in state.world.parameters.all().values() {
            if param.derived {
                // Derivation evaluation happens in parameter system
                updated += 1;
            }
        }

        // Update object derived properties
        for obj in state.world.objects.values_mut() {
            // Example: update bounding volumes, cached transforms, etc.
            updated += 1;
        }

        let metrics = StepMetrics {
            objects_updated: updated,
            ..Default::default()
        };

        Ok(StepResult::success(state, metrics))
    }

    pub fn current_stage(&self) -> Option<&ExecutionStage> {
        self.current_stage.as_ref()
    }
}

impl Default for StageExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionStage {
    pub fn name(&self) -> &str {
        match self {
            ExecutionStage::Init => "Init",
            ExecutionStage::StaticSolve => "StaticSolve",
            ExecutionStage::DynamicUpdate => "DynamicUpdate",
            ExecutionStage::Sync => "Sync",
            ExecutionStage::Custom(name) => name,
        }
    }

    pub fn is_initialization(&self) -> bool {
        matches!(self, ExecutionStage::Init | ExecutionStage::StaticSolve)
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self, ExecutionStage::DynamicUpdate)
    }
}
