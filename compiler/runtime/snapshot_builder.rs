// runtime/snapshot_builder.rs
// Builds immutable snapshots from runtime state for renderer consumption

use crate::state::{ObjectState as StateObjectState, RuntimeState};
use serde::{Deserialize, Serialize};

/// Snapshot builder - converts runtime state to renderer-friendly format
pub struct SnapshotBuilder {
    #[allow(dead_code)]
    next_id: u64,
}

impl SnapshotBuilder {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    /// Build a snapshot from current runtime state
    pub fn build_snapshot(&mut self, state: &RuntimeState) -> RendererSnapshot {
        let tick = state.time.step_count;
        let timestamp = state.time.current_time;

        let objects: Vec<SnapshotObject> = state
            .world
            .objects
            .iter()
            .map(|(id, obj)| self.convert_object(id, obj))
            .collect();

        RendererSnapshot {
            tick,
            timestamp,
            objects,
            math_values: self.extract_math_values(state),
            math_preview: None,
            math_renderables: self.build_math_renderables(state),
            focus_ids: vec![], // TODO: implement focus tracking
        }
    }

    fn build_math_renderables(&self, state: &RuntimeState) -> Vec<SnapshotMathRenderable> {
        let wave = read_real(state, "wave").or_else(|| read_real(state, "engine.wave"));
        let decay = read_real(state, "decay").unwrap_or(1.0);
        let time = read_real(state, "time")
            .or_else(|| read_real(state, "engine.time"))
            .unwrap_or(0.0);
        let integral = read_real(state, "engine.integral_x2_0_1").unwrap_or(0.3333333333);

        let mut out = Vec::new();
        if let Some(amplitude) = wave {
            out.push(SnapshotMathRenderable::Function {
                id: 9_000_001,
                domain: [-3.0, 3.0],
                resolution: 64,
                amplitude,
                frequency: decay.max(0.1),
                phase: time,
            });
            out.push(SnapshotMathRenderable::Surface {
                id: 9_000_002,
                domain_x: [-2.0, 2.0],
                domain_y: [-2.0, 2.0],
                resolution: [24, 24],
                amplitude: amplitude.abs() + 0.25,
                phase: time,
            });
            out.push(SnapshotMathRenderable::Field {
                id: 9_000_003,
                domain_x: [-2.0, 2.0],
                domain_y: [-2.0, 2.0],
                resolution: [20, 20],
                scale: integral,
                phase: time,
            });
        }
        out
    }

    fn extract_math_values(&self, state: &RuntimeState) -> Vec<SnapshotMathValue> {
        let mut entries: Vec<SnapshotMathValue> = state
            .math_values
            .iter()
            .filter_map(|(name, value)| match value {
                crate::math::MathValue::Real(v) => Some(SnapshotMathValue {
                    name: name.clone(),
                    value: *v,
                }),
                crate::math::MathValue::Integer(v) => Some(SnapshotMathValue {
                    name: name.clone(),
                    value: *v as f64,
                }),
                crate::math::MathValue::Rational(num, den) if *den != 0 => {
                    Some(SnapshotMathValue {
                        name: name.clone(),
                        value: *num as f64 / *den as f64,
                    })
                }
                _ => None,
            })
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    fn convert_object(&self, id: &str, obj: &StateObjectState) -> SnapshotObject {
        SnapshotObject {
            id: self.object_id_hash(id),
            geometry: self.convert_geometry(obj),
            transform: SnapshotTransform {
                position: [obj.position.x, obj.position.y, obj.position.z],
                rotation: [
                    obj.orientation.x,
                    obj.orientation.y,
                    obj.orientation.z,
                    obj.orientation.w,
                ],
                scale: [obj.scale.x, obj.scale.y, obj.scale.z],
            },
            material: SnapshotMaterial {
                color: [0.5, 0.7, 1.0, 1.0],
                metallic: 0.3,
                roughness: 0.7,
                opacity: 1.0,
                emissive: [0.0, 0.0, 0.0],
            },
            visible: obj.visible,
            highlighted: false,
        }
    }

    fn convert_geometry(&self, obj: &StateObjectState) -> SnapshotGeometry {
        match obj.kind {
            crate::state::ObjectKind::Sphere => SnapshotGeometry::Sphere { radius: 1.0 },
            crate::state::ObjectKind::Box => SnapshotGeometry::Box {
                width: obj.scale.x,
                height: obj.scale.y,
                depth: obj.scale.z,
            },
            crate::state::ObjectKind::Cylinder => SnapshotGeometry::Cylinder {
                radius: obj.scale.x * 0.5,
                height: obj.scale.y,
            },
            crate::state::ObjectKind::Plane => SnapshotGeometry::Plane {
                width: obj.scale.x,
                height: obj.scale.z,
            },
            _ => SnapshotGeometry::Box {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
        }
    }

    fn object_id_hash(&self, id: &str) -> u64 {
        // Simple hash for demo - would use proper hash in production
        id.bytes().map(|b| b as u64).sum()
    }
}

impl Default for SnapshotBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot sent to renderer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererSnapshot {
    pub tick: u64,
    pub timestamp: f64,
    pub objects: Vec<SnapshotObject>,
    pub math_values: Vec<SnapshotMathValue>,
    pub math_preview: Option<SnapshotMathPreview>,
    pub math_renderables: Vec<SnapshotMathRenderable>,
    pub focus_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMathValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMathPreview {
    pub points: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SnapshotMathRenderable {
    Function {
        id: u64,
        domain: [f64; 2],
        resolution: usize,
        amplitude: f64,
        frequency: f64,
        phase: f64,
    },
    Surface {
        id: u64,
        domain_x: [f64; 2],
        domain_y: [f64; 2],
        resolution: [usize; 2],
        amplitude: f64,
        phase: f64,
    },
    Field {
        id: u64,
        domain_x: [f64; 2],
        domain_y: [f64; 2],
        resolution: [usize; 2],
        scale: f64,
        phase: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotObject {
    pub id: u64,
    pub geometry: SnapshotGeometry,
    pub transform: SnapshotTransform,
    pub material: SnapshotMaterial,
    pub visible: bool,
    pub highlighted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SnapshotGeometry {
    Sphere { radius: f64 },
    Box { width: f64, height: f64, depth: f64 },
    Cylinder { radius: f64, height: f64 },
    Plane { width: f64, height: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotTransform {
    pub position: [f64; 3],
    pub rotation: [f64; 4], // quaternion [x, y, z, w]
    pub scale: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMaterial {
    pub color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub opacity: f32,
    pub emissive: [f32; 3],
}

fn read_real(state: &RuntimeState, key: &str) -> Option<f64> {
    state.math_values.get(key).and_then(|value| match value {
        crate::math::MathValue::Real(v) => Some(*v),
        crate::math::MathValue::Integer(v) => Some(*v as f64),
        crate::math::MathValue::Rational(num, den) if *den != 0 => Some(*num as f64 / *den as f64),
        _ => None,
    })
}
