//! Backend Trait Definition
//!
//! The contract between renderer and rendering engine.
//! Renderer issues commands. Backend executes them.

use crate::renderer::error::RenderResult;
#[cfg(test)]
use crate::renderer::RenderError;

/// Render backend trait
///
/// Any rendering engine must implement this interface.
/// Renderer never calls into the backend's internal logic.
pub trait RenderBackend: Send {
    /// Create a new renderable object
    ///
    /// Returns unique render ID for this object
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        transform: RenderTransform,
        material: RenderMaterial,
    ) -> RenderResult<u64>;

    /// Update object's transformation
    fn update_transform(&mut self, id: u64, transform: RenderTransform) -> RenderResult<()>;

    /// Update object's material properties
    fn update_material(&mut self, id: u64, material: RenderMaterial) -> RenderResult<()>;

    /// Update object's geometry
    fn update_geometry(&mut self, id: u64, geometry: RenderGeometry) -> RenderResult<()>;

    /// Set object visibility
    fn set_visible(&mut self, id: u64, visible: bool) -> RenderResult<()>;

    /// Set object highlight state
    fn set_highlighted(&mut self, id: u64, highlighted: bool) -> RenderResult<()>;

    /// Set object highlight state with a color table index.
    fn set_highlighted_with_color(
        &mut self,
        id: u64,
        highlighted: bool,
        _color_index: u8,
    ) -> RenderResult<()> {
        self.set_highlighted(id, highlighted)
    }

    /// Create a standalone annotation overlay.
    fn create_annotation(&mut self, _annotation: RenderAnnotation) -> RenderResult<u64> {
        Ok(0)
    }

    /// Update an existing annotation overlay.
    fn update_annotation(&mut self, _id: u64, _annotation: RenderAnnotation) -> RenderResult<()> {
        Ok(())
    }

    /// Set or update a label annotation anchored to an object.
    fn set_annotation(
        &mut self,
        _anchor_id: u64,
        _label_text: &str,
        _offset: [f64; 3],
        _is_active: bool,
    ) -> RenderResult<()> {
        Ok(())
    }

    /// Remove object from scene
    fn remove_object(&mut self, id: u64) -> RenderResult<()>;

    /// Clear entire scene
    fn clear_scene(&mut self) -> RenderResult<()>;

    /// Get backend name for debugging
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub enum RenderAnnotation {
    LinearDimension {
        start: [f32; 3],
        end: [f32; 3],
        label: String,
    },
    AngleDimension {
        vertex: [f32; 3],
        ray1: [f32; 3],
        ray2: [f32; 3],
        label: String,
    },
    Note {
        position: [f32; 3],
        text: String,
    },
}

/// Geometry data for rendering
#[derive(Debug, Clone)]
pub enum RenderGeometry {
    Sphere {
        radius: f32,
        segments: u32,
    },
    Box {
        width: f32,
        height: f32,
        depth: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        segments: u32,
    },
    Cone {
        radius: f32,
        height: f32,
        segments: u32,
    },
    Plane {
        width: f32,
        height: f32,
    },
    Line {
        points: Vec<[f32; 3]>,
    },
    Mesh {
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
    },
}

/// Transform as 4x4 matrix (column-major)
#[derive(Debug, Clone, Copy)]
pub struct RenderTransform {
    pub matrix: [f32; 16],
}

impl Default for RenderTransform {
    fn default() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

/// Material rendering properties
#[derive(Debug, Clone, Copy)]
pub struct RenderMaterial {
    pub color: [f32; 4],    // RGBA
    pub metallic: f32,      // 0-1
    pub roughness: f32,     // 0-1
    pub opacity: f32,       // 0-1
    pub emissive: [f32; 3], // RGB
}

impl Default for RenderMaterial {
    fn default() -> Self {
        Self {
            color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            opacity: 1.0,
            emissive: [0.0, 0.0, 0.0],
        }
    }
}

/// Mock backend for testing
#[cfg(test)]
pub struct MockBackend {
    objects: std::collections::HashMap<u64, MockObject>,
    next_id: u64,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct MockObject {
    geometry: RenderGeometry,
    transform: RenderTransform,
    material: RenderMaterial,
    visible: bool,
    highlighted: bool,
}

#[cfg(test)]
impl MockBackend {
    pub fn new() -> Self {
        Self {
            objects: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
impl RenderBackend for MockBackend {
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        transform: RenderTransform,
        material: RenderMaterial,
    ) -> RenderResult<u64> {
        let id = self.next_id;
        self.next_id += 1;

        self.objects.insert(
            id,
            MockObject {
                geometry,
                transform,
                material,
                visible: true,
                highlighted: false,
            },
        );

        Ok(id)
    }

    fn update_transform(&mut self, id: u64, transform: RenderTransform) -> RenderResult<()> {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.transform = transform;
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn update_material(&mut self, id: u64, material: RenderMaterial) -> RenderResult<()> {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.material = material;
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn update_geometry(&mut self, id: u64, geometry: RenderGeometry) -> RenderResult<()> {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.geometry = geometry;
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn set_visible(&mut self, id: u64, visible: bool) -> RenderResult<()> {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.visible = visible;
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn set_highlighted(&mut self, id: u64, highlighted: bool) -> RenderResult<()> {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.highlighted = highlighted;
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn remove_object(&mut self, id: u64) -> RenderResult<()> {
        if self.objects.remove(&id).is_some() {
            Ok(())
        } else {
            Err(RenderError::ObjectNotFound(id))
        }
    }

    fn clear_scene(&mut self) -> RenderResult<()> {
        self.objects.clear();
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_backend() {
        let mut backend = MockBackend::new();

        let geometry = RenderGeometry::Sphere {
            radius: 1.0,
            segments: 16,
        };
        let transform = RenderTransform::default();
        let material = RenderMaterial::default();

        let id = backend
            .create_object(geometry, transform, material)
            .unwrap();
        assert_eq!(backend.object_count(), 1);

        backend.remove_object(id).unwrap();
        assert_eq!(backend.object_count(), 0);
    }

    #[test]
    fn test_object_not_found() {
        let mut backend = MockBackend::new();

        let result = backend.update_transform(999, RenderTransform::default());
        assert!(matches!(result, Err(RenderError::ObjectNotFound(999))));
    }

    #[test]
    fn test_clear_scene() {
        let mut backend = MockBackend::new();

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

        assert_eq!(backend.object_count(), 2);

        backend.clear_scene().unwrap();
        assert_eq!(backend.object_count(), 0);
    }
}
