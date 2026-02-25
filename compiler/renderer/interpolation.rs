//! Visual Interpolation
//!
//! Smooths motion between states. Hides low sampling rates.
//! Never alters true positions.
//!
//! Important: Interpolation is cosmetic. State remains authoritative.

use crate::renderer::ObjectState;

/// Handles smooth visual transitions between frames
///
/// Purpose: Make animation appear smooth even when runtime updates slowly
///
/// Rules:
/// - Never modify authoritative state
/// - Only affects visual presentation
/// - Must be reversible/disableable
pub struct Interpolator {
    enabled: bool,
}

impl Interpolator {
    /// Create new interpolator
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Interpolate between two object state snapshots
    ///
    /// Alpha: 0.0 = previous state, 1.0 = current state
    ///
    /// Returns visually interpolated objects for rendering only
    pub fn interpolate(
        &self,
        previous: &[ObjectState],
        current: &[ObjectState],
        alpha: f64,
    ) -> Vec<ObjectState> {
        if !self.enabled || alpha >= 1.0 {
            return current.to_vec();
        }

        if alpha <= 0.0 {
            return previous.to_vec();
        }

        // Build lookup map for previous states
        let prev_map: std::collections::HashMap<u64, &ObjectState> =
            previous.iter().map(|obj| (obj.id, obj)).collect();

        // Interpolate each current object
        current
            .iter()
            .map(|curr_obj| {
                if let Some(prev_obj) = prev_map.get(&curr_obj.id) {
                    self.interpolate_object(prev_obj, curr_obj, alpha)
                } else {
                    // New object - no interpolation
                    curr_obj.clone()
                }
            })
            .collect()
    }

    /// Interpolate a single object
    fn interpolate_object(
        &self,
        previous: &ObjectState,
        current: &ObjectState,
        alpha: f64,
    ) -> ObjectState {
        ObjectState {
            id: current.id,
            geometry: current.geometry.clone(), // Geometry doesn't interpolate
            transform: self.interpolate_transform(&previous.transform, &current.transform, alpha),
            material: self.interpolate_material(&previous.material, &current.material, alpha),
            visible: current.visible, // Boolean states don't interpolate
            highlighted: current.highlighted,
            highlight_token: current.highlight_token.clone(),
        }
    }

    /// Interpolate transform (position, rotation, scale)
    fn interpolate_transform(
        &self,
        prev: &crate::renderer::Transform,
        curr: &crate::renderer::Transform,
        alpha: f64,
    ) -> crate::renderer::Transform {
        crate::renderer::Transform {
            position: self.lerp_vec3(&prev.position, &curr.position, alpha),
            rotation: self.slerp_quat(&prev.rotation, &curr.rotation, alpha),
            scale: self.lerp_vec3(&prev.scale, &curr.scale, alpha),
        }
    }

    /// Interpolate material properties
    fn interpolate_material(
        &self,
        prev: &crate::renderer::MaterialProperties,
        curr: &crate::renderer::MaterialProperties,
        alpha: f64,
    ) -> crate::renderer::MaterialProperties {
        crate::renderer::MaterialProperties {
            color: self.lerp_color(&prev.color, &curr.color, alpha),
            metallic: self.lerp_f32(prev.metallic, curr.metallic, alpha),
            roughness: self.lerp_f32(prev.roughness, curr.roughness, alpha),
            opacity: self.lerp_f32(prev.opacity, curr.opacity, alpha),
            emissive: self.lerp_color3(&prev.emissive, &curr.emissive, alpha),
        }
    }

    /// Linear interpolation for 3D vectors
    fn lerp_vec3(&self, a: &[f64; 3], b: &[f64; 3], t: f64) -> [f64; 3] {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    }

    /// Linear interpolation for RGBA colors
    fn lerp_color(&self, a: &[f32; 4], b: &[f32; 4], t: f64) -> [f32; 4] {
        let t = t as f32;
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ]
    }

    /// Linear interpolation for RGB colors
    fn lerp_color3(&self, a: &[f32; 3], b: &[f32; 3], t: f64) -> [f32; 3] {
        let t = t as f32;
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    }

    /// Linear interpolation for floats
    fn lerp_f32(&self, a: f32, b: f32, t: f64) -> f32 {
        a + (b - a) * t as f32
    }

    /// Spherical linear interpolation for quaternions
    ///
    /// Provides smooth rotation interpolation
    fn slerp_quat(&self, a: &[f64; 4], b: &[f64; 4], t: f64) -> [f64; 4] {
        // Compute dot product
        let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

        // If negative dot, negate one quaternion to take shorter path
        let (b0, b1, b2, b3) = if dot < 0.0 {
            dot = -dot;
            (-b[0], -b[1], -b[2], -b[3])
        } else {
            (b[0], b[1], b[2], b[3])
        };

        // If very close, use linear interpolation
        if dot > 0.9995 {
            let result = [
                a[0] + (b0 - a[0]) * t,
                a[1] + (b1 - a[1]) * t,
                a[2] + (b2 - a[2]) * t,
                a[3] + (b3 - a[3]) * t,
            ];
            return self.normalize_quat(&result);
        }

        // Slerp
        let theta = dot.acos();
        let sin_theta = theta.sin();

        let scale_a = ((1.0 - t) * theta).sin() / sin_theta;
        let scale_b = (t * theta).sin() / sin_theta;

        [
            a[0] * scale_a + b0 * scale_b,
            a[1] * scale_a + b1 * scale_b,
            a[2] * scale_a + b2 * scale_b,
            a[3] * scale_a + b3 * scale_b,
        ]
    }

    /// Normalize quaternion
    fn normalize_quat(&self, q: &[f64; 4]) -> [f64; 4] {
        let mag = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        if mag > 0.0 {
            [q[0] / mag, q[1] / mag, q[2] / mag, q[3] / mag]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        }
    }

    /// Enable or disable interpolation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if interpolation is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{GeometryType, MaterialProperties, Transform};

    fn make_test_object(id: u64, position: [f64; 3]) -> ObjectState {
        ObjectState {
            id,
            geometry: GeometryType::Sphere { radius: 1.0 },
            transform: Transform {
                position,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            material: MaterialProperties::default(),
            visible: true,
            highlighted: false,
            highlight_token: None,
        }
    }

    #[test]
    fn test_disabled_interpolation() {
        let interp = Interpolator::new(false);

        let prev = vec![make_test_object(1, [0.0, 0.0, 0.0])];
        let curr = vec![make_test_object(1, [10.0, 0.0, 0.0])];

        let result = interp.interpolate(&prev, &curr, 0.5);

        // Should return current state when disabled
        assert_eq!(result[0].transform.position[0], 10.0);
    }

    #[test]
    fn test_position_interpolation() {
        let interp = Interpolator::new(true);

        let prev = vec![make_test_object(1, [0.0, 0.0, 0.0])];
        let curr = vec![make_test_object(1, [10.0, 0.0, 0.0])];

        let result = interp.interpolate(&prev, &curr, 0.5);

        // Should be halfway between
        assert!((result[0].transform.position[0] - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_alpha_bounds() {
        let interp = Interpolator::new(true);

        let prev = vec![make_test_object(1, [0.0, 0.0, 0.0])];
        let curr = vec![make_test_object(1, [10.0, 0.0, 0.0])];

        // Alpha = 0 should return previous
        let result = interp.interpolate(&prev, &curr, 0.0);
        assert_eq!(result[0].transform.position[0], 0.0);

        // Alpha = 1 should return current
        let result = interp.interpolate(&prev, &curr, 1.0);
        assert_eq!(result[0].transform.position[0], 10.0);
    }

    #[test]
    fn test_new_objects_no_interpolation() {
        let interp = Interpolator::new(true);

        let prev = vec![];
        let curr = vec![make_test_object(1, [10.0, 0.0, 0.0])];

        let result = interp.interpolate(&prev, &curr, 0.5);

        // New object should use current state
        assert_eq!(result[0].transform.position[0], 10.0);
    }

    #[test]
    fn test_material_interpolation() {
        let interp = Interpolator::new(true);

        let mut prev_obj = make_test_object(1, [0.0, 0.0, 0.0]);
        prev_obj.material.metallic = 0.0;

        let mut curr_obj = make_test_object(1, [0.0, 0.0, 0.0]);
        curr_obj.material.metallic = 1.0;

        let result = interp.interpolate(&[prev_obj], &[curr_obj], 0.5);

        assert!((result[0].material.metallic - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_quaternion_normalization() {
        let interp = Interpolator::new(true);

        let q = [1.0, 2.0, 3.0, 4.0];
        let normalized = interp.normalize_quat(&q);

        // Check unit length
        let mag = (normalized[0] * normalized[0]
            + normalized[1] * normalized[1]
            + normalized[2] * normalized[2]
            + normalized[3] * normalized[3])
            .sqrt();

        assert!((mag - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_lerp_vec3() {
        let interp = Interpolator::new(true);

        let a = [0.0, 0.0, 0.0];
        let b = [10.0, 20.0, 30.0];

        let result = interp.lerp_vec3(&a, &b, 0.5);

        assert_eq!(result, [5.0, 10.0, 15.0]);
    }

    #[test]
    fn test_state_independence() {
        let interp = Interpolator::new(true);

        let prev = vec![make_test_object(1, [0.0, 0.0, 0.0])];
        let curr = vec![make_test_object(1, [10.0, 0.0, 0.0])];

        let result1 = interp.interpolate(&prev, &curr, 0.5);
        let result2 = interp.interpolate(&prev, &curr, 0.5);

        // Multiple calls with same input produce same output
        assert_eq!(
            result1[0].transform.position[0],
            result2[0].transform.position[0]
        );
    }
}
