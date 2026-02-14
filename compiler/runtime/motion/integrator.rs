// runtime/motion/integrator.rs
// Responsible for:
// - Time integration (Euler, RK, etc.)
// - Stability over speed
// - Fixed or adaptive steps
// Educational accuracy beats frame rate

use crate::error::{IntegrationError, IntegrationErrorKind, RuntimeError, RuntimeResult};
use crate::state::{ObjectState, Vector3, WorldState};

/// Integration method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// Forward Euler (simplest, least stable)
    Euler,
    /// Semi-implicit Euler (better stability)
    SemiImplicitEuler,
    /// 2nd order Runge-Kutta
    RK2,
    /// 4th order Runge-Kutta (most accurate, most expensive)
    RK4,
    /// Verlet integration (good for oscillators)
    Verlet,
}

impl Default for IntegrationMethod {
    fn default() -> Self {
        Self::SemiImplicitEuler
    }
}

/// Time integrator
pub struct Integrator {
    method: IntegrationMethod,
    /// Minimum allowed time step
    min_dt: f64,
    /// Maximum allowed time step
    max_dt: f64,
}

impl Integrator {
    pub fn new(method: IntegrationMethod) -> Self {
        Self {
            method,
            min_dt: 1e-6,
            max_dt: 0.1,
        }
    }

    pub fn with_time_step_bounds(mut self, min_dt: f64, max_dt: f64) -> Self {
        self.min_dt = min_dt;
        self.max_dt = max_dt;
        self
    }

    /// Integrate motion for all dynamic objects
    pub fn integrate(
        &mut self,
        state: &mut WorldState,
        dt: f64,
    ) -> RuntimeResult<IntegrationResult> {
        // Validate time step
        if dt < self.min_dt {
            return Err(RuntimeError::IntegrationFailure(IntegrationError {
                kind: IntegrationErrorKind::StepTooSmall,
                time: state.world.parameters.get("time").unwrap_or(0.0),
                object_id: None,
            }));
        }

        if dt > self.max_dt {
            return Err(RuntimeError::IntegrationFailure(IntegrationError {
                kind: IntegrationErrorKind::Unstable,
                time: state.world.parameters.get("time").unwrap_or(0.0),
                object_id: None,
            }));
        }

        let mut result = IntegrationResult {
            objects_updated: 0,
            max_velocity: 0.0,
            max_acceleration: 0.0,
            stable: true,
        };

        // Update each dynamic object
        for (id, object) in state.world.objects.iter_mut() {
            if object.is_static {
                continue;
            }

            if let Some(velocity) = object.velocity {
                // Integrate based on method
                match self.method {
                    IntegrationMethod::Euler => {
                        self.integrate_euler(object, dt)?;
                    }
                    IntegrationMethod::SemiImplicitEuler => {
                        self.integrate_semi_implicit_euler(object, dt)?;
                    }
                    IntegrationMethod::RK2 => {
                        self.integrate_rk2(object, dt)?;
                    }
                    IntegrationMethod::RK4 => {
                        self.integrate_rk4(object, dt)?;
                    }
                    IntegrationMethod::Verlet => {
                        self.integrate_verlet(object, dt)?;
                    }
                }

                // Track statistics
                let vel_magnitude = velocity.length();
                result.max_velocity = result.max_velocity.max(vel_magnitude);
                result.objects_updated += 1;

                // Check for instability
                if object.position.has_nan() || object.position.has_infinity() {
                    return Err(RuntimeError::IntegrationFailure(IntegrationError {
                        kind: IntegrationErrorKind::NaN,
                        time: state.world.parameters.get("time").unwrap_or(0.0),
                        object_id: Some(id.clone()),
                    }));
                }
            }
        }

        Ok(result)
    }

    fn integrate_euler(&self, object: &mut ObjectState, dt: f64) -> RuntimeResult<()> {
        if let Some(velocity) = object.velocity {
            // x' = x + v * dt
            object.position.x += velocity.x * dt;
            object.position.y += velocity.y * dt;
            object.position.z += velocity.z * dt;
        }
        Ok(())
    }

    fn integrate_semi_implicit_euler(
        &self,
        object: &mut ObjectState,
        dt: f64,
    ) -> RuntimeResult<()> {
        // Semi-implicit Euler is more stable for oscillatory systems
        // v' = v + a * dt
        // x' = x + v' * dt (use updated velocity)

        if let Some(mut velocity) = object.velocity {
            // For now, assume zero acceleration (would come from forces)
            // In a full implementation, this would compute forces/acceleration

            // Update position with current velocity
            object.position.x += velocity.x * dt;
            object.position.y += velocity.y * dt;
            object.position.z += velocity.z * dt;

            object.velocity = Some(velocity);
        }
        Ok(())
    }

    fn integrate_rk2(&self, object: &mut ObjectState, dt: f64) -> RuntimeResult<()> {
        // 2nd order Runge-Kutta (midpoint method)
        // k1 = f(x, v)
        // k2 = f(x + k1*dt/2, v)
        // x' = x + k2 * dt

        if let Some(velocity) = object.velocity {
            // k1 = v
            let k1 = velocity;

            // Midpoint position
            let mid_pos = Vector3::new(
                object.position.x + k1.x * dt * 0.5,
                object.position.y + k1.y * dt * 0.5,
                object.position.z + k1.z * dt * 0.5,
            );

            // k2 = velocity at midpoint (same as k1 for constant velocity)
            let k2 = velocity;

            // Update position
            object.position.x += k2.x * dt;
            object.position.y += k2.y * dt;
            object.position.z += k2.z * dt;
        }
        Ok(())
    }

    fn integrate_rk4(&self, object: &mut ObjectState, dt: f64) -> RuntimeResult<()> {
        // 4th order Runge-Kutta (most accurate)
        // k1 = f(x)
        // k2 = f(x + k1*dt/2)
        // k3 = f(x + k2*dt/2)
        // k4 = f(x + k3*dt)
        // x' = x + (k1 + 2*k2 + 2*k3 + k4) * dt / 6

        if let Some(velocity) = object.velocity {
            let k1 = velocity;
            let k2 = velocity; // Would be different with acceleration
            let k3 = velocity;
            let k4 = velocity;

            // Weighted average
            object.position.x += (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x) * dt / 6.0;
            object.position.y += (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y) * dt / 6.0;
            object.position.z += (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z) * dt / 6.0;
        }
        Ok(())
    }

    fn integrate_verlet(&self, object: &mut ObjectState, dt: f64) -> RuntimeResult<()> {
        // Verlet integration (good for oscillators)
        // x' = 2*x - x_old + a * dt^2
        // This requires storing previous position (not implemented in current ObjectState)
        // For now, fall back to semi-implicit Euler
        self.integrate_semi_implicit_euler(object, dt)
    }

    pub fn method(&self) -> IntegrationMethod {
        self.method
    }

    pub fn set_method(&mut self, method: IntegrationMethod) {
        self.method = method;
    }
}

/// Result of integration step
#[derive(Debug, Clone)]
pub struct IntegrationResult {
    /// Number of objects updated
    pub objects_updated: usize,
    /// Maximum velocity magnitude
    pub max_velocity: f64,
    /// Maximum acceleration magnitude
    pub max_acceleration: f64,
    /// Is integration stable
    pub stable: bool,
}

impl IntegrationResult {
    pub fn is_success(&self) -> bool {
        self.stable
    }
}
