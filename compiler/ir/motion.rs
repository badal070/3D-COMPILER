use crate::ids::{EntityId, MotionId};
use crate::values::{Angle, EasingFunction, Scalar, Time, Vector3};

#[derive(Debug, Clone)]
pub struct Motion {
    pub id: MotionId,
    pub target: EntityId,
    pub kind: MotionKind,
}

#[derive(Debug, Clone)]
pub enum MotionKind {
    Rotation {
        axis: Vector3,
        speed: Angle,  // radians per second
    },
    Translation {
        direction: Vector3,
        speed: f64,  // units per second
    },
    Scale {
        factor: Vector3,
        speed: f64,  // scale change per second
    },
    Oscillation(OscillationMotion),
    Orbital(OrbitalMotion),
    Sequential(SequentialMotion),
    Parallel(ParallelMotion),
    Damped(DampedMotion),
    Periodic(PeriodicMotion),
    Eased(EasedMotion),
    Parametric(ParametricMotion),
}

impl Motion {
    pub fn new(id: MotionId, target: EntityId, kind: MotionKind) -> Self {
        Self { id, target, kind }
    }

    pub fn rotation(id: MotionId, target: EntityId, axis: Vector3, speed: Angle) -> Self {
        Self::new(id, target, MotionKind::Rotation { axis, speed })
    }

    pub fn translation(id: MotionId, target: EntityId, direction: Vector3, speed: f64) -> Self {
        Self::new(id, target, MotionKind::Translation { direction, speed })
    }

    pub fn scale(id: MotionId, target: EntityId, factor: Vector3, speed: f64) -> Self {
        Self::new(id, target, MotionKind::Scale { factor, speed })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillationMotion {
    pub amplitude: f64,
    pub frequency: f64,
    pub phase_offset: f64,
    pub axis: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalMotion {
    pub radius: f64,
    pub angular_speed: f64,
    pub axis: Vector3,
    pub center: Vector3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialMotion {
    pub motions: Vec<MotionId>,
    pub durations: Vec<Time>,
    pub blend_time: Time,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParallelMotion {
    pub motions: Vec<MotionId>,
    pub weights: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DampedMotion {
    pub base_motion: MotionId,
    pub damping_coefficient: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicMotion {
    pub base_motion: MotionId,
    pub period: Time,
    pub repeat_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EasedMotion {
    pub base_motion: MotionId,
    pub easing_function: EasingFunction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParametricMotion {
    pub function_id: String,
    pub custom_parameters: Vec<Scalar>,
}
