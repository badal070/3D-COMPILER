use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scalar {
    Float(f64),
    Int(i64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn one() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }
}

// Radians only. Degrees are illegal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(pub f64);

impl Angle {
    pub fn radians(rad: f64) -> Self {
        Angle(rad)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

// Seconds only. Frames are illegal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Time(pub f64);

impl Time {
    pub fn seconds(sec: f64) -> Self {
        Time(sec)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Eq for Time {}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Physics values
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Force(pub Vector3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Energy(pub f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Momentum(pub Vector3);

// Chemistry values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomicNumber(pub u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Electronegativity(pub f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BondOrder(pub u8);

// Robotics values
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointPosition(pub f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointVelocity(pub f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Torque(pub f64);

// Easing functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    Elastic,
    Bounce,
}
