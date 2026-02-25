use super::{TimeState, WorldState};
use crate::math::{Expression, MathValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightScheduleEntry {
    pub at_time: f64,
    pub highlight_token: String,
    pub entity_id: String,
    pub color_index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationEntry {
    pub label_text: String,
    pub anchor_entity_id: String,
    pub position_offset: [f64; 3],
    pub equation_node_id: Option<String>,
    pub highlight_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MathRenderableKind {
    Function,
    Surface,
    Field,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathRenderableEntry {
    pub id: u64,
    pub kind: MathRenderableKind,
    pub expression: Expression,
    pub domain_x: [f64; 2],
    pub domain_y: Option<[f64; 2]>,
    pub resolution: [usize; 2],
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    pub world: WorldState,
    pub time: TimeState,
    pub math_values: HashMap<String, MathValue>,
    pub current_highlight_token: Option<String>,
    pub highlight_schedule: Vec<HighlightScheduleEntry>,
    pub annotations: Vec<AnnotationEntry>,
    pub math_renderables: Vec<MathRenderableEntry>,
}

impl RuntimeState {
    pub fn new(world: WorldState, time: TimeState) -> Self {
        Self {
            world,
            time,
            math_values: HashMap::new(),
            current_highlight_token: None,
            highlight_schedule: Vec::new(),
            annotations: Vec::new(),
            math_renderables: Vec::new(),
        }
    }

    pub fn set_highlight_token(&mut self, token: Option<String>) {
        self.current_highlight_token = token;
    }

    pub fn validate(&self) -> Result<(), String> {
        self.world.validate()?;
        self.time.validate()?;
        if self.has_nan() {
            return Err("Runtime state contains NaN values".to_string());
        }
        if self.has_infinity() {
            return Err("Runtime state contains infinite values".to_string());
        }
        Ok(())
    }

    pub fn has_nan(&self) -> bool {
        self.world.has_nan()
            || self.math_values.values().any(|value| match value {
                MathValue::Real(v) => v.is_nan(),
                MathValue::Complex(c) => c.real.is_nan() || c.imag.is_nan(),
                _ => false,
            })
    }

    pub fn has_infinity(&self) -> bool {
        self.world.has_infinity()
            || self.math_values.values().any(|value| match value {
                MathValue::Real(v) => v.is_infinite(),
                MathValue::Complex(c) => c.real.is_infinite() || c.imag.is_infinite(),
                _ => false,
            })
    }

    pub fn summary(&self) -> RuntimeStateSummary {
        RuntimeStateSummary {
            object_count: self.world.objects.len(),
            parameter_count: self.world.parameters.values().len(),
            constraint_count: self.world.constraints.len(),
            math_value_count: self.math_values.len(),
            annotation_count: self.annotations.len(),
            highlight_schedule_count: self.highlight_schedule.len(),
            has_nan: self.has_nan(),
            has_infinity: self.has_infinity(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeStateSummary {
    pub object_count: usize,
    pub parameter_count: usize,
    pub constraint_count: usize,
    pub math_value_count: usize,
    pub annotation_count: usize,
    pub highlight_schedule_count: usize,
    pub has_nan: bool,
    pub has_infinity: bool,
}
