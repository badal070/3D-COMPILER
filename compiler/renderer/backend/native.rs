//! Native Backend
//!
//! Desktop acceleration, large scenes, debug builds.
//! Uses wgpu for cross-platform GPU rendering.

use crate::renderer::{
    backend::{RenderBackend, RenderGeometry, RenderMaterial, RenderTransform},
    error::{RenderError, RenderResult},
};
use std::collections::HashMap;

/// Native GPU rendering backend using wgpu
///
/// Optimized for:
/// - Desktop performance
/// - Large scene handling
/// - Debug visualization
/// - Offline rendering
pub struct NativeBackend {
    objects: HashMap<u64, NativeObject>,
    debug_mode: bool,
}

struct NativeObject {
    geometry: RenderGeometry,
    transform: RenderTransform,
    material: RenderMaterial,
    visible: bool,
    highlighted: bool,
    // In real implementation, these would be GPU buffers
    vertex_buffer_id: Option<u64>,
    index_buffer_id: Option<u64>,
}

impl NativeBackend {
    /// Create new native backend
    pub fn new() -> RenderResult<Self> {
        Ok(Self {
            objects: HashMap::new(),
            debug_mode: false,
        })
    }

    /// Create with debug mode enabled
    pub fn new_debug() -> RenderResult<Self> {
        Ok(Self {
            objects: HashMap::new(),
            debug_mode: true,
        })
    }

    /// Enable or disable debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    /// Get debug statistics
    pub fn debug_stats(&self) -> NativeBackendStats {
        let total_vertices: usize = self
            .objects
            .values()
            .map(|obj| self.estimate_vertex_count(&obj.geometry))
            .sum();

        let total_triangles: usize = self
            .objects
            .values()
            .map(|obj| self.estimate_triangle_count(&obj.geometry))
            .sum();

        NativeBackendStats {
            object_count: self.objects.len(),
            vertex_count: total_vertices,
            triangle_count: total_triangles,
            buffer_count: self
                .objects
                .values()
                .filter(|o| o.vertex_buffer_id.is_some())
                .count(),
        }
    }

    fn estimate_vertex_count(&self, geometry: &RenderGeometry) -> usize {
        match geometry {
            RenderGeometry::Sphere { segments, .. } => {
                let rings = segments;
                let sectors = segments;
                ((rings + 1) * (sectors + 1)) as usize
            }
            RenderGeometry::Box { .. } => 24, // 6 faces * 4 vertices
            RenderGeometry::Cylinder { segments, .. } => ((segments * 2 + 2) * 2) as usize,
            RenderGeometry::Cone { segments, .. } => (segments + 1 + segments) as usize,
            RenderGeometry::Plane { .. } => 4,
            RenderGeometry::Line { points } => points.len(),
            RenderGeometry::Mesh { vertices, .. } => vertices.len(),
        }
    }

    fn estimate_triangle_count(&self, geometry: &RenderGeometry) -> usize {
        match geometry {
            RenderGeometry::Sphere { segments, .. } => {
                let rings = segments;
                let sectors = segments;
                (rings * sectors * 2) as usize
            }
            RenderGeometry::Box { .. } => 12, // 6 faces * 2 triangles
            RenderGeometry::Cylinder { segments, .. } => (segments * 4) as usize,
            RenderGeometry::Cone { segments, .. } => (segments * 2) as usize,
            RenderGeometry::Plane { .. } => 2,
            RenderGeometry::Line { .. } => 0,
            RenderGeometry::Mesh { indices, .. } => indices.len() / 3,
        }
    }

    fn log_debug(&self, message: &str) {
        if self.debug_mode {
            log::debug!("[Native Backend] {}", message);
        }
    }

    // In real implementation, these would create GPU buffers
    fn create_vertex_buffer(&mut self, _geometry: &RenderGeometry) -> u64 {
        static mut BUFFER_ID: u64 = 1;
        unsafe {
            let id = BUFFER_ID;
            BUFFER_ID += 1;
            id
        }
    }

    fn create_index_buffer(&mut self, _geometry: &RenderGeometry) -> u64 {
        static mut BUFFER_ID: u64 = 1;
        unsafe {
            let id = BUFFER_ID;
            BUFFER_ID += 1;
            id
        }
    }
}

impl Default for NativeBackend {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl RenderBackend for NativeBackend {
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        transform: RenderTransform,
        material: RenderMaterial,
    ) -> RenderResult<u64> {
        let id = self.objects.len() as u64 + 1;

        self.log_debug(&format!(
            "Creating object {} with geometry {:?}",
            id, geometry
        ));

        // Create GPU buffers (simulated)
        let vertex_buffer_id = Some(self.create_vertex_buffer(&geometry));
        let index_buffer_id = Some(self.create_index_buffer(&geometry));

        self.objects.insert(
            id,
            NativeObject {
                geometry,
                transform,
                material,
                visible: true,
                highlighted: false,
                vertex_buffer_id,
                index_buffer_id,
            },
        );

        Ok(id)
    }

    fn update_transform(&mut self, id: u64, transform: RenderTransform) -> RenderResult<()> {
        self.log_debug(&format!("Updating transform for object {}", id));
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.transform = transform;
        Ok(())
    }

    fn update_material(&mut self, id: u64, material: RenderMaterial) -> RenderResult<()> {
        self.log_debug(&format!("Updating material for object {}", id));
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.material = material;
        Ok(())
    }

    fn update_geometry(&mut self, id: u64, geometry: RenderGeometry) -> RenderResult<()> {
        self.log_debug(&format!("Updating geometry for object {}", id));
        let vertex_buffer_id = Some(self.create_vertex_buffer(&geometry));
        let index_buffer_id = Some(self.create_index_buffer(&geometry));

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        // Would need to recreate GPU buffers
        obj.vertex_buffer_id = vertex_buffer_id;
        obj.index_buffer_id = index_buffer_id;
        obj.geometry = geometry;

        Ok(())
    }

    fn set_visible(&mut self, id: u64, visible: bool) -> RenderResult<()> {
        self.log_debug(&format!(
            "Setting visibility for object {} to {}",
            id, visible
        ));
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.visible = visible;
        Ok(())
    }

    fn set_highlighted(&mut self, id: u64, highlighted: bool) -> RenderResult<()> {
        self.log_debug(&format!(
            "Setting highlight for object {} to {}",
            id, highlighted
        ));
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.highlighted = highlighted;

        // In real implementation, would update material to show highlight
        if highlighted {
            // Increase emissive color
        }

        Ok(())
    }

    fn set_highlighted_with_color(
        &mut self,
        id: u64,
        highlighted: bool,
        color_index: u8,
    ) -> RenderResult<()> {
        self.log_debug(&format!(
            "Setting colored highlight for object {} to {} with color {}",
            id, highlighted, color_index
        ));
        self.set_highlighted(id, highlighted)
    }

    fn set_annotation(
        &mut self,
        anchor_id: u64,
        label_text: &str,
        _offset: [f64; 3],
        is_active: bool,
    ) -> RenderResult<()> {
        self.log_debug(&format!(
            "Setting annotation on {}: '{}' (active={})",
            anchor_id, label_text, is_active
        ));
        if self.objects.contains_key(&anchor_id) {
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(anchor_id))
        }
    }

    fn remove_object(&mut self, id: u64) -> RenderResult<()> {
        let obj = self
            .objects
            .remove(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        self.log_debug(&format!("Removing object {}", id));

        // Would clean up GPU buffers
        if obj.vertex_buffer_id.is_some() {
            self.log_debug("  - Freed vertex buffer");
        }
        if obj.index_buffer_id.is_some() {
            self.log_debug("  - Freed index buffer");
        }

        Ok(())
    }

    fn clear_scene(&mut self) -> RenderResult<()> {
        self.log_debug(&format!(
            "Clearing scene with {} objects",
            self.objects.len()
        ));

        let count = self.objects.len();
        self.objects.clear();

        self.log_debug(&format!("  - Removed {} objects", count));
        Ok(())
    }

    fn name(&self) -> &str {
        if self.debug_mode {
            "native (debug)"
        } else {
            "native"
        }
    }
}

/// Statistics for native backend
#[derive(Debug, Clone, Copy)]
pub struct NativeBackendStats {
    pub object_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub buffer_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_backend_creation() {
        let backend = NativeBackend::new().unwrap();
        assert_eq!(backend.name(), "native");
    }

    #[test]
    fn test_debug_mode() {
        let mut backend = NativeBackend::new_debug().unwrap();
        assert_eq!(backend.name(), "native (debug)");

        backend.set_debug_mode(false);
        assert_eq!(backend.name(), "native");
    }

    #[test]
    fn test_object_lifecycle() {
        let mut backend = NativeBackend::new().unwrap();

        let geometry = RenderGeometry::Sphere {
            radius: 1.0,
            segments: 16,
        };
        let transform = RenderTransform::default();
        let material = RenderMaterial::default();

        let id = backend
            .create_object(geometry, transform, material)
            .unwrap();

        assert!(backend.objects.contains_key(&id));

        backend.remove_object(id).unwrap();
        assert!(!backend.objects.contains_key(&id));
    }

    #[test]
    fn test_debug_stats() {
        let mut backend = NativeBackend::new().unwrap();

        backend
            .create_object(
                RenderGeometry::Sphere {
                    radius: 1.0,
                    segments: 16,
                },
                RenderTransform::default(),
                RenderMaterial::default(),
            )
            .unwrap();

        backend
            .create_object(
                RenderGeometry::Box {
                    width: 1.0,
                    height: 1.0,
                    depth: 1.0,
                },
                RenderTransform::default(),
                RenderMaterial::default(),
            )
            .unwrap();

        let stats = backend.debug_stats();
        assert_eq!(stats.object_count, 2);
        assert!(stats.vertex_count > 0);
        assert!(stats.triangle_count > 0);
    }

    #[test]
    fn test_vertex_estimation() {
        let backend = NativeBackend::new().unwrap();

        let sphere = RenderGeometry::Sphere {
            radius: 1.0,
            segments: 16,
        };
        let vertices = backend.estimate_vertex_count(&sphere);
        assert!(vertices > 0);

        let box_geo = RenderGeometry::Box {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        };
        let vertices = backend.estimate_vertex_count(&box_geo);
        assert_eq!(vertices, 24);
    }

    #[test]
    fn test_triangle_estimation() {
        let backend = NativeBackend::new().unwrap();

        let box_geo = RenderGeometry::Box {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        };
        let triangles = backend.estimate_triangle_count(&box_geo);
        assert_eq!(triangles, 12);
    }
}
