// runtime/engine.rs
// Runtime Orchestrator
// Owns the execution loop, advances time, calls execution stages in order
// Handles pause, reset, step
// This file answers: "what happens next?"
// Math evaluation is isolated in RuntimeMathEngine and fed through state snapshots.

use crate::constraint::ConstraintSystem;
use crate::error::{RuntimeError, RuntimeResult};
use crate::executor::{ExecutionPlan, StageExecutor, Watchdog};
use crate::math::{BinaryOperator, Expression, MathValue, RuntimeMathEngine};
use crate::motion::MotionSystem;
use crate::snapshot::{Snapshot, SnapshotHistory};
use crate::state::{RuntimeState, TimeState, WorldState};
use crate::{ExecutionState, RuntimeCommand, RuntimeConfig};

/// Main runtime engine
pub struct RuntimeEngine {
    /// Current execution state
    state: RuntimeState,
    /// Execution state (running, paused, etc.)
    execution_state: ExecutionState,
    /// Configuration
    config: RuntimeConfig,
    /// Stage executor
    stage_executor: StageExecutor,
    /// Constraint system
    constraint_system: ConstraintSystem,
    /// Motion system
    motion_system: MotionSystem,
    /// Runtime math engine for expression/derivative/integral evaluation
    math_engine: RuntimeMathEngine,
    /// Fingerprint of parameter values used for cache invalidation
    last_parameter_fingerprint: Vec<(String, u64)>,
    /// Watchdog
    watchdog: Watchdog,
    /// Snapshot history
    snapshots: Option<SnapshotHistory>,
    /// Current execution plan
    plan: Option<ExecutionPlan>,
}

impl RuntimeEngine {
    pub fn new(config: RuntimeConfig) -> Self {
        let watchdog = Watchdog::new(config.max_steps as u64, config.max_execution_time_ms);

        let snapshots = if config.enable_snapshots {
            Some(SnapshotHistory::new(config.max_snapshots))
        } else {
            None
        };

        let constraint_config = crate::constraint::solver::SolverConfig {
            tolerance: config.constraint_tolerance,
            max_iterations: config.max_constraint_iterations,
            ..Default::default()
        };

        Self {
            state: RuntimeState::new(WorldState::new(), TimeState::new()),
            execution_state: ExecutionState::Idle,
            config,
            stage_executor: StageExecutor::new(),
            constraint_system: ConstraintSystem::new(constraint_config),
            motion_system: MotionSystem::default(),
            math_engine: RuntimeMathEngine::new(),
            last_parameter_fingerprint: Vec::new(),
            watchdog,
            snapshots,
            plan: None,
        }
    }

    /// Initialize with state and execution plan
    pub fn initialize(&mut self, state: RuntimeState, plan: ExecutionPlan) -> RuntimeResult<()> {
        // Validate state
        state
            .validate()
            .map_err(|e| RuntimeError::Configuration(format!("Invalid initial state: {}", e)))?;

        // Validate plan
        plan.validate()?;

        self.state = state;
        self.plan = Some(plan);
        self.execution_state = ExecutionState::Idle;
        self.watchdog.reset();
        self.last_parameter_fingerprint = Self::parameter_fingerprint(&self.state);
        self.math_engine.invalidate_caches();
        self.evaluate_runtime_math()?;

        // Take initial snapshot
        if let Some(snapshots) = &mut self.snapshots {
            snapshots.take_snapshot(self.state.clone(), Some("Initial".to_string()));
        }

        Ok(())
    }

    /// Process a runtime command
    pub fn command(&mut self, cmd: RuntimeCommand) -> RuntimeResult<()> {
        match cmd {
            RuntimeCommand::Start => self.start(),
            RuntimeCommand::Pause => self.pause(),
            RuntimeCommand::Resume => self.resume(),
            RuntimeCommand::Stop => self.stop(),
            RuntimeCommand::Step => self.step(),
            RuntimeCommand::Reset => self.reset(),
        }
    }

    fn start(&mut self) -> RuntimeResult<()> {
        if self.plan.is_none() {
            return Err(RuntimeError::InvalidPlan(
                "No execution plan set".to_string(),
            ));
        }

        self.execution_state = ExecutionState::Running;
        self.state.time.start();
        self.watchdog.start();
        Ok(())
    }

    fn pause(&mut self) -> RuntimeResult<()> {
        self.execution_state = ExecutionState::Paused;
        self.state.time.pause();
        self.watchdog.stop();
        Ok(())
    }

    fn resume(&mut self) -> RuntimeResult<()> {
        if self.execution_state == ExecutionState::Paused {
            self.execution_state = ExecutionState::Running;
            self.state.time.start();
            self.watchdog.start();
        }
        Ok(())
    }

    fn stop(&mut self) -> RuntimeResult<()> {
        self.execution_state = ExecutionState::Stopped;
        self.state.time.pause();
        self.watchdog.stop();
        Ok(())
    }

    fn step(&mut self) -> RuntimeResult<()> {
        self.state.time.step();
        self.execute_single_step()?;
        self.state.time.pause();
        Ok(())
    }

    fn reset(&mut self) -> RuntimeResult<()> {
        self.state.time.reset();
        self.execution_state = ExecutionState::Idle;
        self.watchdog.reset();

        // Restore from initial snapshot if available
        if let Some(snapshots) = &self.snapshots {
            if let Some(initial) = snapshots.with_label("Initial").next() {
                self.state = initial.state.clone();
            }
        }
        self.last_parameter_fingerprint = Self::parameter_fingerprint(&self.state);
        self.math_engine.invalidate_caches();
        self.evaluate_runtime_math()?;

        Ok(())
    }

    /// Execute a single step
    pub fn execute_single_step(&mut self) -> RuntimeResult<()> {
        // Check watchdog
        self.watchdog.check()?;
        self.watchdog.step();

        // Determine time step
        let dt = match self.config.time_step {
            crate::TimeStep::Fixed(dt) => dt,
            crate::TimeStep::Adaptive { min, max, .. } => {
                // Adaptive time stepping would go here
                min
            }
        };

        // Apply constraints
        self.constraint_system
            .solve_and_enforce(&mut self.state.world)?;

        // Update motion
        self.motion_system.update(&mut self.state.world, dt)?;

        // Advance time
        self.state.time.advance(dt)?;
        self.invalidate_math_caches_if_parameters_changed();
        self.evaluate_runtime_math()?;

        // Validate state if configured
        if self.state.world.flags.validate_steps {
            self.state.validate().map_err(|e| {
                RuntimeError::InvalidState(crate::error::StateError {
                    kind: crate::error::StateErrorKind::InvariantViolation,
                    object_id: None,
                    details: e,
                })
            })?;
        }

        // Check for NaN
        if self.state.world.has_nan() {
            self.watchdog.record_nan();
            return Err(RuntimeError::IntegrationFailure(
                crate::error::IntegrationError {
                    kind: crate::error::IntegrationErrorKind::NaN,
                    time: self.state.time.current_time,
                    object_id: None,
                },
            ));
        }

        // Take snapshot if configured
        if self.state.world.flags.record_snapshots {
            if let Some(snapshots) = &mut self.snapshots {
                snapshots.take_snapshot(self.state.clone(), None);
            }
        }

        Ok(())
    }

    /// Run until time limit or completion
    pub fn run_until(&mut self, target_time: f64) -> RuntimeResult<ExecutionSummary> {
        let start_time = self.state.time.current_time;
        let mut steps = 0;

        self.start()?;

        while self.state.time.current_time < target_time
            && self.execution_state == ExecutionState::Running
        {
            match self.execute_single_step() {
                Ok(_) => {
                    steps += 1;
                }
                Err(e) => {
                    self.execution_state = ExecutionState::Error;
                    return Err(e);
                }
            }
        }

        self.pause()?;

        Ok(ExecutionSummary {
            steps_executed: steps,
            time_elapsed: self.state.time.current_time - start_time,
            final_time: self.state.time.current_time,
            success: true,
        })
    }

    /// Get current state
    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    /// Get mutable state (use with caution)
    pub fn state_mut(&mut self) -> &mut RuntimeState {
        &mut self.state
    }

    /// Get execution state
    pub fn execution_state(&self) -> ExecutionState {
        self.execution_state
    }

    /// Get watchdog stats
    pub fn watchdog_stats(&self) -> crate::executor::watchdog::WatchdogStats {
        self.watchdog.stats()
    }

    /// Get snapshot history
    pub fn snapshots(&self) -> Option<&SnapshotHistory> {
        self.snapshots.as_ref()
    }

    /// Take manual snapshot
    pub fn take_snapshot(&mut self, label: Option<String>) -> Option<&Snapshot> {
        self.snapshots
            .as_mut()
            .map(|s| s.take_snapshot(self.state.clone(), label))
    }

    /// Restore from snapshot
    pub fn restore_snapshot(&mut self, id: u64) -> RuntimeResult<()> {
        let snapshot = self
            .snapshots
            .as_ref()
            .and_then(|s| s.get(id))
            .ok_or_else(|| RuntimeError::Configuration("Snapshot not found".to_string()))?;

        self.state = snapshot.state.clone();
        self.last_parameter_fingerprint = Self::parameter_fingerprint(&self.state);
        self.math_engine.invalidate_caches();
        self.evaluate_runtime_math()?;
        Ok(())
    }

    /// Get engine statistics
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            execution_state: self.execution_state,
            current_time: self.state.time.current_time,
            step_count: self.state.time.step_count,
            object_count: self.state.world.objects.len(),
            parameter_count: self.state.world.parameters.values().len(),
            constraint_count: self.state.world.constraints.len(),
            snapshot_count: self.snapshots.as_ref().map(|s| s.count()).unwrap_or(0),
        }
    }
}

/// Execution summary
#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    pub steps_executed: usize,
    pub time_elapsed: f64,
    pub final_time: f64,
    pub success: bool,
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub execution_state: ExecutionState,
    pub current_time: f64,
    pub step_count: u64,
    pub object_count: usize,
    pub parameter_count: usize,
    pub constraint_count: usize,
    pub snapshot_count: usize,
}

impl EngineStats {
    pub fn is_running(&self) -> bool {
        self.execution_state == ExecutionState::Running
    }

    pub fn is_paused(&self) -> bool {
        self.execution_state == ExecutionState::Paused
    }

    pub fn has_error(&self) -> bool {
        self.execution_state == ExecutionState::Error
    }
}

impl RuntimeEngine {
    fn parameter_fingerprint(state: &RuntimeState) -> Vec<(String, u64)> {
        let mut items: Vec<(String, u64)> = state
            .world
            .parameters
            .all()
            .iter()
            .map(|(id, parameter)| (id.clone(), parameter.value.to_bits()))
            .collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    }

    fn invalidate_math_caches_if_parameters_changed(&mut self) {
        let current = Self::parameter_fingerprint(&self.state);
        if current != self.last_parameter_fingerprint {
            self.math_engine.invalidate_caches();
            self.last_parameter_fingerprint = current;
        }
    }

    fn evaluate_runtime_math(&mut self) -> RuntimeResult<()> {
        let current_time = self.state.time.current_time;
        let mut variables = self.state.world.parameters.values();
        variables.insert("t".to_string(), current_time);
        let gain = variables.get("gain").copied().unwrap_or(1.0);

        // expression: sin(t)
        let wave_expression = Expression::Binary(
            Box::new(Expression::Number(gain)),
            BinaryOperator::Multiply,
            Box::new(Expression::FunctionCall(
                "sin".to_string(),
                vec![Expression::Variable("t".to_string())],
            )),
        );
        let wave = self
            .math_engine
            .evaluate_expression(&wave_expression, &variables)?;

        // derivative: d/dt (t^3)
        let cubic = Expression::Binary(
            Box::new(Expression::Variable("t".to_string())),
            BinaryOperator::Power,
            Box::new(Expression::Number(3.0)),
        );
        let derivative_expr = self.math_engine.evaluate_derivative(&cubic, "t")?;
        let derivative_value = self
            .math_engine
            .evaluate_expression(&derivative_expr, &variables)?;

        // integral: ∫[0,1] x^2 dx
        let integral_expr = Expression::Binary(
            Box::new(Expression::Variable("x".to_string())),
            BinaryOperator::Power,
            Box::new(Expression::Number(2.0)),
        );
        let integral_value = self
            .math_engine
            .evaluate_integral(&integral_expr, "x", 0.0, 1.0)?;

        self.state
            .math_values
            .insert("engine.time".to_string(), MathValue::Real(current_time));
        self.state
            .math_values
            .insert("engine.wave".to_string(), wave);
        self.state
            .math_values
            .insert("engine.d_wave_dt".to_string(), derivative_value);
        self.state.math_values.insert(
            "engine.integral_x2_0_1".to_string(),
            MathValue::Real(integral_value),
        );

        Ok(())
    }
}
