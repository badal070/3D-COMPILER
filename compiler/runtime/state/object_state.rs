// runtime/state/object_state.rs
// Stores position, orientation, scale, derived properties
// Rules:
// - No rendering info
// - No backend-specific fields

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ObjectId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectState {
    /// Object identifier
    pub id: ObjectId,
    /// Object type (for semantic understanding)
    pub kind: ObjectKind,
    /// Position in 3D space
    pub position: Vector3,
    /// Orientation (quaternion)
    pub orientation: Quaternion,
    /// Scale (non-uniform allowed)
    pub scale: Vector3,
    /// Linear velocity (if dynamic)
    pub velocity: Option<Vector3>,
    /// Angular velocity (if dynamic)
    pub angular_velocity: Option<Vector3>,
    /// Derived properties (computed from constraints/motion)
    pub derived: HashMap<String, f64>,
    /// Object is static (never moves)
    pub is_static: bool,
    /// Object is visible
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectKind {
    Point,
    Line,
    Plane,
    Circle,
    Sphere,
    Box,
    Cylinder,
    Mesh,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn has_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }

    pub fn has_infinity(&self) -> bool {
        self.x.is_infinite() || self.y.is_infinite() || self.z.is_infinite()
    }
}

impl Quaternion {
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    pub fn from_axis_angle(axis: Vector3, angle: f64) -> Self {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        let axis_norm = axis.normalize();
        Self {
            w: half_angle.cos(),
            x: axis_norm.x * s,
            y: axis_norm.y * s,
            z: axis_norm.z * s,
        }
    }

    pub fn length_squared(&self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(&self) -> Self {
        let len = self.length_squared().sqrt();
        if len > 0.0 {
            Self {
                w: self.w / len,
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::IDENTITY
        }
    }

    pub fn has_nan(&self) -> bool {
        self.w.is_nan() || self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }

    pub fn has_infinity(&self) -> bool {
        self.w.is_infinite() || self.x.is_infinite() || self.y.is_infinite() || self.z.is_infinite()
    }
}

impl Default for Vector3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ObjectState {
    pub fn new(id: ObjectId, kind: ObjectKind) -> Self {
        Self {
            id,
            kind,
            position: Vector3::ZERO,
            orientation: Quaternion::IDENTITY,
            scale: Vector3::ONE,
            velocity: None,
            angular_velocity: None,
            derived: HashMap::new(),
            is_static: false,
            visible: true,
        }
    }

    pub fn with_position(mut self, position: Vector3) -> Self {
        self.position = position;
        self
    }

    pub fn with_orientation(mut self, orientation: Quaternion) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn with_scale(mut self, scale: Vector3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_velocity(mut self, velocity: Vector3) -> Self {
        self.velocity = Some(velocity);
        self
    }

    pub fn make_static(mut self) -> Self {
        self.is_static = true;
        self.velocity = None;
        self.angular_velocity = None;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.position.has_nan() {
            return Err("Position contains NaN".to_string());
        }
        if self.position.has_infinity() {
            return Err("Position contains infinity".to_string());
        }
        if self.orientation.has_nan() {
            return Err("Orientation contains NaN".to_string());
        }
        if self.orientation.has_infinity() {
            return Err("Orientation contains infinity".to_string());
        }
        if self.scale.has_nan() {
            return Err("Scale contains NaN".to_string());
        }
        if self.scale.has_infinity() {
            return Err("Scale contains infinity".to_string());
        }

        // Validate orientation is unit quaternion
        let quat_len_sq = self.orientation.length_squared();
        if (quat_len_sq - 1.0).abs() > 1e-6 {
            return Err(format!(
                "Orientation is not normalized (length^2 = {})",
                quat_len_sq
            ));
        }

        Ok(())
    }

    pub fn has_nan(&self) -> bool {
        self.position.has_nan()
            || self.orientation.has_nan()
            || self.scale.has_nan()
            || self.velocity.as_ref().map_or(false, |v| v.has_nan())
            || self
                .angular_velocity
                .as_ref()
                .map_or(false, |v| v.has_nan())
            || self.derived.values().any(|v| v.is_nan())
    }

    pub fn has_infinity(&self) -> bool {
        self.position.has_infinity()
            || self.orientation.has_infinity()
            || self.scale.has_infinity()
            || self.velocity.as_ref().map_or(false, |v| v.has_infinity())
            || self
                .angular_velocity
                .as_ref()
                .map_or(false, |v| v.has_infinity())
            || self.derived.values().any(|v| v.is_infinite())
    }

    pub fn set_derived(&mut self, key: String, value: f64) {
        self.derived.insert(key, value);
    }

    pub fn get_derived(&self, key: &str) -> Option<f64> {
        self.derived.get(key).copied()
    }
}
