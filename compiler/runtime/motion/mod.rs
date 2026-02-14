// runtime/motion/mod.rs
// Motion subsystem - time integration and sampling

pub mod integrator;
pub mod sampler;

pub use integrator::{IntegrationMethod, IntegrationResult, Integrator};
pub use sampler::{MotionSampler, SamplePoint};

use crate::error::RuntimeResult;
use crate::state::{ObjectState, WorldState};

/// Motion system - handles all motion updates
pub struct MotionSystem {
    integrator: Integrator,
    sampler: MotionSampler,
}

impl MotionSystem {
    pub fn new(method: IntegrationMethod) -> Self {
        Self {
            integrator: Integrator::new(method),
            sampler: MotionSampler::new(),
        }
    }

    /// Update motion for all dynamic objects
    pub fn update(&mut self, state: &mut WorldState, dt: f64) -> RuntimeResult<IntegrationResult> {
        self.integrator.integrate(state, dt)
    }

    /// Sample motion at a specific time
    pub fn sample_at(&self, state: &WorldState, time: f64) -> RuntimeResult<Vec<SamplePoint>> {
        self.sampler.sample_all(state, time)
    }

    pub fn integrator(&self) -> &Integrator {
        &self.integrator
    }

    pub fn sampler(&self) -> &MotionSampler {
        &self.sampler
    }
}

impl Default for MotionSystem {
    fn default() -> Self {
        Self::new(IntegrationMethod::default())
    }
}
