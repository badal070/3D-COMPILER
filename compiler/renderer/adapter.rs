//! Semantic → Visual Mapping
//!
//! Deterministic, stateless, reversible conversion.
//! Maps abstract geometry to concrete render primitives.

use crate::renderer::{
    backend::{RenderGeometry, RenderMaterial, RenderTransform},
    error::{RenderError, RenderResult},
    GeometryType, MaterialProperties, Transform,
};

/// Converts semantic objects to render objects
///
/// Rules:
/// - Deterministic: Same input always produces same output
/// - Stateless: No internal state affects conversion
/// - Reversible: Where possible, maintain enough info to reverse
pub struct Adapter {}

impl Adapter {
    pub fn new() -> Self {
        Self {}
    }

    /// Convert semantic geometry to render geometry
    ///
    /// Example: Sphere(radius=r) → Mesh(SphereGeometry(r))
    pub fn convert_geometry(&self, geometry: &GeometryType) -> RenderResult<RenderGeometry> {
        match geometry {
            GeometryType::Sphere { radius } => {
                if *radius <= 0.0 {
                    return Err(RenderError::InvalidGeometry(
                        "Sphere radius must be positive".into(),
                    ));
                }
                Ok(RenderGeometry::Sphere {
                    radius: *radius as f32,
                    segments: self.sphere_segments(*radius),
                })
            }

            GeometryType::Box {
                width,
                height,
                depth,
            } => {
                if *width <= 0.0 || *height <= 0.0 || *depth <= 0.0 {
                    return Err(RenderError::InvalidGeometry(
                        "Box dimensions must be positive".into(),
                    ));
                }
                Ok(RenderGeometry::Box {
                    width: *width as f32,
                    height: *height as f32,
                    depth: *depth as f32,
                })
            }

            GeometryType::Cylinder { radius, height } => {
                if *radius <= 0.0 || *height <= 0.0 {
                    return Err(RenderError::InvalidGeometry(
                        "Cylinder dimensions must be positive".into(),
                    ));
                }
                Ok(RenderGeometry::Cylinder {
                    radius: *radius as f32,
                    height: *height as f32,
                    segments: self.cylinder_segments(*radius),
                })
            }

            GeometryType::Cone { radius, height } => {
                if *radius <= 0.0 || *height <= 0.0 {
                    return Err(RenderError::InvalidGeometry(
                        "Cone dimensions must be positive".into(),
                    ));
                }
                Ok(RenderGeometry::Cone {
                    radius: *radius as f32,
                    height: *height as f32,
                    segments: self.cone_segments(*radius),
                })
            }

            GeometryType::Plane { width, height } => {
                if *width <= 0.0 || *height <= 0.0 {
                    return Err(RenderError::InvalidGeometry(
                        "Plane dimensions must be positive".into(),
                    ));
                }
                Ok(RenderGeometry::Plane {
                    width: *width as f32,
                    height: *height as f32,
                })
            }

            GeometryType::Line { points } => {
                if points.len() < 2 {
                    return Err(RenderError::InvalidGeometry(
                        "Line must have at least 2 points".into(),
                    ));
                }
                let converted: Vec<[f32; 3]> = points
                    .iter()
                    .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
                    .collect();
                Ok(RenderGeometry::Line { points: converted })
            }

            GeometryType::Mesh { vertices, indices } => {
                if vertices.is_empty() {
                    return Err(RenderError::InvalidGeometry("Mesh has no vertices".into()));
                }
                if indices.is_empty() {
                    return Err(RenderError::InvalidGeometry("Mesh has no indices".into()));
                }

                let converted_vertices: Vec<[f32; 3]> = vertices
                    .iter()
                    .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
                    .collect();

                Ok(RenderGeometry::Mesh {
                    vertices: converted_vertices,
                    indices: indices.clone(),
                })
            }
        }
    }

    /// Convert transform to 4x4 matrix
    ///
    /// Maintains numerical precision through quaternion math
    pub fn convert_transform(&self, transform: &Transform) -> RenderTransform {
        let matrix =
            self.compose_matrix(&transform.position, &transform.rotation, &transform.scale);

        RenderTransform { matrix }
    }

    /// Convert material properties to render material
    ///
    /// Direct mapping, no interpretation
    pub fn convert_material(&self, material: &MaterialProperties) -> RenderMaterial {
        RenderMaterial {
            color: material.color,
            metallic: material.metallic.clamp(0.0, 1.0),
            roughness: material.roughness.clamp(0.0, 1.0),
            opacity: material.opacity.clamp(0.0, 1.0),
            emissive: material.emissive,
        }
    }

    // Private helpers - deterministic calculations only

    /// Calculate sphere segment count based on radius
    /// Larger spheres get more detail
    fn sphere_segments(&self, radius: f64) -> u32 {
        let base = 16;
        let scale = (radius.log10().max(0.0) * 8.0) as u32;
        (base + scale).min(64)
    }

    /// Calculate cylinder segment count
    fn cylinder_segments(&self, radius: f64) -> u32 {
        let base = 16;
        let scale = (radius.log10().max(0.0) * 8.0) as u32;
        (base + scale).min(64)
    }

    /// Calculate cone segment count
    fn cone_segments(&self, radius: f64) -> u32 {
        let base = 16;
        let scale = (radius.log10().max(0.0) * 8.0) as u32;
        (base + scale).min(64)
    }

    /// Compose 4x4 transformation matrix from TRS components
    ///
    /// Order: Scale -> Rotate -> Translate
    fn compose_matrix(
        &self,
        position: &[f64; 3],
        rotation: &[f64; 4],
        scale: &[f64; 3],
    ) -> [f32; 16] {
        // Normalize quaternion
        let (x, y, z, w) = (rotation[0], rotation[1], rotation[2], rotation[3]);
        let mag = (x * x + y * y + z * z + w * w).sqrt();
        let (x, y, z, w) = if mag > 0.0 {
            (x / mag, y / mag, z / mag, w / mag)
        } else {
            (0.0, 0.0, 0.0, 1.0)
        };

        // Compute rotation matrix from quaternion
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        let sx = scale[0];
        let sy = scale[1];
        let sz = scale[2];

        // Column-major 4x4 matrix
        [
            ((1.0 - (yy + zz)) * sx) as f32,
            ((xy + wz) * sx) as f32,
            ((xz - wy) * sx) as f32,
            0.0,
            ((xy - wz) * sy) as f32,
            ((1.0 - (xx + zz)) * sy) as f32,
            ((yz + wx) * sy) as f32,
            0.0,
            ((xz + wy) * sz) as f32,
            ((yz - wx) * sz) as f32,
            ((1.0 - (xx + yy)) * sz) as f32,
            0.0,
            position[0] as f32,
            position[1] as f32,
            position[2] as f32,
            1.0,
        ]
    }
}

impl Default for Adapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_conversion() {
        let adapter = Adapter::new();
        let geometry = GeometryType::Sphere { radius: 1.0 };

        let result1 = adapter.convert_geometry(&geometry).unwrap();
        let result2 = adapter.convert_geometry(&geometry).unwrap();

        // Same input produces same output
        assert_eq!(format!("{:?}", result1), format!("{:?}", result2));
    }

    #[test]
    fn test_invalid_geometry_rejected() {
        let adapter = Adapter::new();

        // Negative radius
        let geometry = GeometryType::Sphere { radius: -1.0 };
        assert!(adapter.convert_geometry(&geometry).is_err());

        // Zero dimensions
        let geometry = GeometryType::Box {
            width: 0.0,
            height: 1.0,
            depth: 1.0,
        };
        assert!(adapter.convert_geometry(&geometry).is_err());

        // Too few points
        let geometry = GeometryType::Line {
            points: vec![[0.0, 0.0, 0.0]],
        };
        assert!(adapter.convert_geometry(&geometry).is_err());
    }

    #[test]
    fn test_transform_identity() {
        let adapter = Adapter::new();
        let transform = Transform::default();
        let result = adapter.convert_transform(&transform);

        // Identity matrix
        let expected = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        for (a, b) in result.matrix.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_material_clamping() {
        let adapter = Adapter::new();

        let material = MaterialProperties {
            color: [1.0, 1.0, 1.0, 1.0],
            metallic: 2.0,   // Over range
            roughness: -1.0, // Under range
            opacity: 1.5,    // Over range
            emissive: [0.0, 0.0, 0.0],
        };

        let result = adapter.convert_material(&material);

        assert_eq!(result.metallic, 1.0);
        assert_eq!(result.roughness, 0.0);
        assert_eq!(result.opacity, 1.0);
    }

    #[test]
    fn test_stateless_operation() {
        let adapter1 = Adapter::new();
        let adapter2 = Adapter::new();

        let geometry = GeometryType::Sphere { radius: 2.0 };

        let result1 = adapter1.convert_geometry(&geometry).unwrap();
        let result2 = adapter2.convert_geometry(&geometry).unwrap();

        // Different adapter instances produce same result
        assert_eq!(format!("{:?}", result1), format!("{:?}", result2));
    }
}
