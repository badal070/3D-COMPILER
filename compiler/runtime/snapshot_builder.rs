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
        let active_highlight_token = state.current_highlight_token.clone();
        let highlight_schedule = state
            .highlight_schedule
            .iter()
            .map(|entry| SnapshotHighlightEntry {
                at_time: entry.at_time,
                highlight_token: entry.highlight_token.clone(),
                entity_id_hash: self.object_id_hash(&entry.entity_id),
                color_index: entry.color_index,
            })
            .collect::<Vec<_>>();
        let annotations = state
            .annotations
            .iter()
            .map(|annotation| SnapshotAnnotation {
                label_text: annotation.label_text.clone(),
                anchor_object_id: self.object_id_hash(&annotation.anchor_entity_id),
                position_offset: annotation.position_offset,
                equation_node_id: annotation.equation_node_id.clone(),
                highlight_token: annotation.highlight_token.clone(),
                is_active: annotation
                    .highlight_token
                    .as_ref()
                    .zip(active_highlight_token.as_ref())
                    .map(|(a, b)| a == b)
                    .unwrap_or(false),
            })
            .collect::<Vec<_>>();

        RendererSnapshot {
            tick,
            timestamp,
            objects,
            math_values: self.extract_math_values(state),
            math_preview: None,
            math_renderables: self.build_math_renderables(state),
            focus_ids: vec![], // TODO: implement focus tracking
            active_highlight_token,
            highlight_schedule,
            annotations,
        }
    }

    fn build_math_renderables(&self, state: &RuntimeState) -> Vec<SnapshotMathRenderable> {
        state
            .math_renderables
            .iter()
            .map(|entry| match entry.kind {
                crate::state::MathRenderableKind::Function => SnapshotMathRenderable::Function {
                    id: entry.id,
                    domain: entry.domain_x,
                    resolution: entry.resolution[0],
                    amplitude: entry.amplitude,
                    frequency: entry.frequency.max(0.1),
                    phase: entry.phase,
                },
                crate::state::MathRenderableKind::Surface => SnapshotMathRenderable::Surface {
                    id: entry.id,
                    domain_x: entry.domain_x,
                    domain_y: entry.domain_y.unwrap_or([-1.0, 1.0]),
                    resolution: entry.resolution,
                    amplitude: entry.amplitude,
                    phase: entry.phase,
                },
                crate::state::MathRenderableKind::Field => SnapshotMathRenderable::Field {
                    id: entry.id,
                    domain_x: entry.domain_x,
                    domain_y: entry.domain_y.unwrap_or([-1.0, 1.0]),
                    resolution: entry.resolution,
                    scale: entry.scale,
                    phase: entry.phase,
                },
            })
            .collect()
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
    pub active_highlight_token: Option<String>,
    pub highlight_schedule: Vec<SnapshotHighlightEntry>,
    pub annotations: Vec<SnapshotAnnotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHighlightEntry {
    pub at_time: f64,
    pub highlight_token: String,
    pub entity_id_hash: u64,
    pub color_index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotAnnotation {
    pub label_text: String,
    pub anchor_object_id: u64,
    pub position_offset: [f64; 3],
    pub equation_node_id: Option<String>,
    pub highlight_token: Option<String>,
    pub is_active: bool,
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
