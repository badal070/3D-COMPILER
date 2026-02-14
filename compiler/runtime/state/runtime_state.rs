use super::{TimeState, WorldState};
use crate::math::MathValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    pub world: WorldState,
    pub time: TimeState,
    pub math_values: HashMap<String, MathValue>,
}

impl RuntimeState {
    pub fn new(world: WorldState, time: TimeState) -> Self {
        Self {
            world,
            time,
            math_values: HashMap::new(),
        }
    }
}
