//! Visibility Management
//!
//! Handles culling, layering, and educational focus.
//! Still no math decisions - just presentation rules.

use crate::renderer::ObjectState;

/// Manages which objects are visible and how they're presented
///
/// Responsibilities:
/// - Culling (performance optimization)
/// - Layer management (organization)
/// - Educational focus (highlighting relevant objects)
///
/// Not responsible for:
/// - Determining object correctness
/// - Modifying object state
/// - Making semantic decisions
pub struct VisibilityManager {
    culling_enabled: bool,
}

impl VisibilityManager {
    /// Create new visibility manager
    pub fn new(culling_enabled: bool) -> Self {
        Self { culling_enabled }
    }

    /// Filter objects based on visibility rules
    ///
    /// Returns only objects that should be rendered
    pub fn filter(&self, objects: &[ObjectState], focus_ids: &[u64]) -> Vec<ObjectState> {
        objects
            .iter()
            .filter(|obj| self.should_render(obj, focus_ids))
            .cloned()
            .collect()
    }

    /// Determine if an object should be rendered
    fn should_render(&self, obj: &ObjectState, focus_ids: &[u64]) -> bool {
        // Explicit visibility flag takes precedence
        if !obj.visible {
            return false;
        }

        // If culling disabled, render everything visible
        if !self.culling_enabled {
            return true;
        }

        // If there are focused objects, only render those
        if !focus_ids.is_empty() {
            return focus_ids.contains(&obj.id);
        }

        // Otherwise render all visible objects
        true
    }

    /// Apply educational focus highlighting
    ///
    /// Marks focused objects for visual emphasis
    pub fn apply_focus(&self, objects: &mut [ObjectState], focus_ids: &[u64]) {
        for obj in objects.iter_mut() {
            obj.highlighted = focus_ids.contains(&obj.id);
        }
    }

    /// Compute visibility layers for complex scenes
    ///
    /// Returns objects grouped by layer priority
    pub fn compute_layers(&self, objects: &[ObjectState]) -> Vec<Vec<ObjectState>> {
        let mut layers: Vec<Vec<ObjectState>> = Vec::new();

        // Layer 0: Highlighted objects (front)
        let highlighted: Vec<ObjectState> = objects
            .iter()
            .filter(|obj| obj.highlighted)
            .cloned()
            .collect();
        if !highlighted.is_empty() {
            layers.push(highlighted);
        }

        // Layer 1: Normal objects (middle)
        let normal: Vec<ObjectState> = objects
            .iter()
            .filter(|obj| !obj.highlighted)
            .cloned()
            .collect();
        if !normal.is_empty() {
            layers.push(normal);
        }

        layers
    }

    /// Perform frustum culling (spatial optimization)
    ///
    /// Returns objects within view frustum
    /// Note: Requires camera parameters from backend
    pub fn frustum_cull(
        &self,
        objects: &[ObjectState],
        camera_position: [f64; 3],
        view_distance: f64,
    ) -> Vec<ObjectState> {
        if !self.culling_enabled {
            return objects.to_vec();
        }

        objects
            .iter()
            .filter(|obj| {
                let distance = self.distance(&obj.transform.position, &camera_position);
                distance <= view_distance
            })
            .cloned()
            .collect()
    }

    /// Perform occlusion culling
    ///
    /// Returns objects not fully occluded by others
    /// This is a simple implementation - real occlusion is complex
    pub fn occlusion_cull(&self, objects: &[ObjectState]) -> Vec<ObjectState> {
        if !self.culling_enabled {
            return objects.to_vec();
        }

        // Simple heuristic: keep highlighted objects always
        // For non-highlighted, apply basic Z-sorting logic
        objects
            .iter()
            .filter(|obj| {
                obj.highlighted || obj.transform.position[2] >= -100.0 // Simple depth check
            })
            .cloned()
            .collect()
    }

    /// Calculate statistics about visibility
    pub fn stats(&self, objects: &[ObjectState], focus_ids: &[u64]) -> VisibilityStats {
        let total = objects.len();
        let visible = objects.iter().filter(|o| o.visible).count();
        let hidden = total - visible;
        let focused = objects.iter().filter(|o| focus_ids.contains(&o.id)).count();
        let highlighted = objects.iter().filter(|o| o.highlighted).count();

        VisibilityStats {
            total_objects: total,
            visible_objects: visible,
            hidden_objects: hidden,
            focused_objects: focused,
            highlighted_objects: highlighted,
        }
    }

    /// Enable or disable culling
    pub fn set_culling_enabled(&mut self, enabled: bool) {
        self.culling_enabled = enabled;
    }

    /// Check if culling is enabled
    pub fn is_culling_enabled(&self) -> bool {
        self.culling_enabled
    }

    // Private helpers

    /// Calculate Euclidean distance between two points
    fn distance(&self, a: &[f64; 3], b: &[f64; 3]) -> f64 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Visibility statistics
#[derive(Debug, Clone, Copy)]
pub struct VisibilityStats {
    pub total_objects: usize,
    pub visible_objects: usize,
    pub hidden_objects: usize,
    pub focused_objects: usize,
    pub highlighted_objects: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{GeometryType, MaterialProperties, Transform};

    fn make_test_object(id: u64, visible: bool) -> ObjectState {
        ObjectState {
            id,
            geometry: GeometryType::Sphere { radius: 1.0 },
            transform: Transform::default(),
            material: MaterialProperties::default(),
            visible,
            highlighted: false,
        }
    }

    #[test]
    fn test_visibility_filtering() {
        let manager = VisibilityManager::new(false);

        let objects = vec![
            make_test_object(1, true),
            make_test_object(2, false),
            make_test_object(3, true),
        ];

        let filtered = manager.filter(&objects, &[]);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|o| o.visible));
    }

    #[test]
    fn test_focus_filtering() {
        let manager = VisibilityManager::new(true);

        let objects = vec![
            make_test_object(1, true),
            make_test_object(2, true),
            make_test_object(3, true),
        ];

        let filtered = manager.filter(&objects, &[1, 3]);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|o| o.id == 1));
        assert!(filtered.iter().any(|o| o.id == 3));
    }

    #[test]
    fn test_apply_focus() {
        let manager = VisibilityManager::new(false);

        let mut objects = vec![
            make_test_object(1, true),
            make_test_object(2, true),
            make_test_object(3, true),
        ];

        manager.apply_focus(&mut objects, &[2]);

        assert!(!objects[0].highlighted);
        assert!(objects[1].highlighted);
        assert!(!objects[2].highlighted);
    }

    #[test]
    fn test_layer_computation() {
        let manager = VisibilityManager::new(false);

        let mut objects = vec![
            make_test_object(1, true),
            make_test_object(2, true),
            make_test_object(3, true),
        ];

        objects[1].highlighted = true;

        let layers = manager.compute_layers(&objects);

        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].len(), 1); // Highlighted layer
        assert_eq!(layers[1].len(), 2); // Normal layer
    }

    #[test]
    fn test_frustum_culling() {
        let manager = VisibilityManager::new(true);

        let mut obj1 = make_test_object(1, true);
        obj1.transform.position = [0.0, 0.0, 0.0];

        let mut obj2 = make_test_object(2, true);
        obj2.transform.position = [100.0, 0.0, 0.0];

        let objects = vec![obj1, obj2];

        let camera = [0.0, 0.0, 0.0];
        let view_distance = 50.0;

        let culled = manager.frustum_cull(&objects, camera, view_distance);

        assert_eq!(culled.len(), 1);
        assert_eq!(culled[0].id, 1);
    }

    #[test]
    fn test_culling_disabled() {
        let manager = VisibilityManager::new(false);

        let mut obj1 = make_test_object(1, true);
        obj1.transform.position = [0.0, 0.0, 0.0];

        let mut obj2 = make_test_object(2, true);
        obj2.transform.position = [100.0, 0.0, 0.0];

        let objects = vec![obj1, obj2];

        let camera = [0.0, 0.0, 0.0];
        let view_distance = 50.0;

        let culled = manager.frustum_cull(&objects, camera, view_distance);

        // With culling disabled, all objects returned
        assert_eq!(culled.len(), 2);
    }

    #[test]
    fn test_stats() {
        let manager = VisibilityManager::new(false);

        let mut objects = vec![
            make_test_object(1, true),
            make_test_object(2, false),
            make_test_object(3, true),
        ];

        objects[0].highlighted = true;

        let stats = manager.stats(&objects, &[1, 3]);

        assert_eq!(stats.total_objects, 3);
        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.hidden_objects, 1);
        assert_eq!(stats.focused_objects, 2);
        assert_eq!(stats.highlighted_objects, 1);
    }

    #[test]
    fn test_distance_calculation() {
        let manager = VisibilityManager::new(true);

        let a = [0.0, 0.0, 0.0];
        let b = [3.0, 4.0, 0.0];

        let distance = manager.distance(&a, &b);

        assert!((distance - 5.0).abs() < 0.001);
    }
}
