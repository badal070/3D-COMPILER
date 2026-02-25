// runtime/state/parameter_state.rs
// Holds user-controlled variables, derived parameters, live-updatable values
// This is what enables parametric learning

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ParameterId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterState {
    /// All parameters (global and local)
    parameters: HashMap<ParameterId, Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub id: ParameterId,
    pub kind: ParameterKind,
    pub value: f64,
    /// Valid range for parameter
    pub range: Option<ParameterRange>,
    /// Is this parameter user-controllable
    pub user_controllable: bool,
    /// Is this parameter derived from others
    pub derived: bool,
    /// Derivation expression (if derived)
    pub derivation: Option<String>,
    /// Units for display
    pub units: Option<String>,
    /// Display symbol (typically LaTeX)
    pub display_symbol: Option<String>,
    /// Human-readable description
    pub description: Option<String>,
    /// Scene entity that visually encodes this parameter
    pub animation_entity_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterKind {
    Scalar,
    Angle,
    Length,
    Time,
    Mass,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParameterRange {
    pub min: f64,
    pub max: f64,
    /// Should values wrap around (for angles)
    pub wraps: bool,
}

impl ParameterState {
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
        }
    }

    /// Add parameter
    pub fn add(&mut self, param: Parameter) -> Result<(), String> {
        if self.parameters.contains_key(&param.id) {
            return Err(format!("Parameter {} already exists", param.id));
        }
        param.validate()?;
        self.parameters.insert(param.id.clone(), param);
        Ok(())
    }

    /// Remove parameter
    pub fn remove(&mut self, id: &ParameterId) -> Result<Parameter, String> {
        self.parameters
            .remove(id)
            .ok_or_else(|| format!("Parameter {} not found", id))
    }

    /// Get parameter value
    pub fn get(&self, id: &ParameterId) -> Option<f64> {
        self.parameters.get(id).map(|p| p.value)
    }

    /// Set parameter value
    pub fn set(&mut self, id: &ParameterId, value: f64) -> Result<(), String> {
        let param = self
            .parameters
            .get_mut(id)
            .ok_or_else(|| format!("Parameter {} not found", id))?;

        if param.derived {
            return Err(format!("Cannot set derived parameter {}", id));
        }

        param.set_value(value)?;
        Ok(())
    }

    /// Get parameter
    pub fn get_parameter(&self, id: &ParameterId) -> Option<&Parameter> {
        self.parameters.get(id)
    }

    /// Get mutable parameter
    pub fn get_parameter_mut(&mut self, id: &ParameterId) -> Option<&mut Parameter> {
        self.parameters.get_mut(id)
    }

    /// All parameters
    pub fn all(&self) -> &HashMap<ParameterId, Parameter> {
        &self.parameters
    }

    /// User-controllable parameters
    pub fn user_controllable(&self) -> impl Iterator<Item = &Parameter> {
        self.parameters.values().filter(|p| p.user_controllable)
    }

    /// Derived parameters
    pub fn derived(&self) -> impl Iterator<Item = &Parameter> {
        self.parameters.values().filter(|p| p.derived)
    }

    /// Get all parameter values
    pub fn values(&self) -> HashMap<ParameterId, f64> {
        self.parameters
            .iter()
            .map(|(id, p)| (id.clone(), p.value))
            .collect()
    }

    /// Validate all parameters
    pub fn validate(&self) -> Result<(), String> {
        for param in self.parameters.values() {
            param.validate()?;
        }
        Ok(())
    }

    /// Check for NaN
    pub fn has_nan(&self) -> bool {
        self.parameters.values().any(|p| p.value.is_nan())
    }

    /// Check for infinity
    pub fn has_infinity(&self) -> bool {
        self.parameters.values().any(|p| p.value.is_infinite())
    }
}

impl Default for ParameterState {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameter {
    pub fn new(id: ParameterId, value: f64) -> Self {
        Self {
            id,
            kind: ParameterKind::Scalar,
            value,
            range: None,
            user_controllable: true,
            derived: false,
            derivation: None,
            units: None,
            display_symbol: None,
            description: None,
            animation_entity_id: None,
        }
    }

    pub fn with_kind(mut self, kind: ParameterKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_range(mut self, min: f64, max: f64, wraps: bool) -> Self {
        self.range = Some(ParameterRange { min, max, wraps });
        self
    }

    pub fn make_derived(mut self, expression: String) -> Self {
        self.derived = true;
        self.derivation = Some(expression);
        self.user_controllable = false;
        self
    }

    pub fn with_units(mut self, units: String) -> Self {
        self.units = Some(units);
        self
    }

    pub fn with_display_symbol(mut self, symbol: String) -> Self {
        self.display_symbol = Some(symbol);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_animation_entity_id(mut self, entity_id: String) -> Self {
        self.animation_entity_id = Some(entity_id);
        self
    }

    pub fn set_value(&mut self, value: f64) -> Result<(), String> {
        if value.is_nan() {
            return Err("Cannot set parameter to NaN".to_string());
        }
        if value.is_infinite() {
            return Err("Cannot set parameter to infinity".to_string());
        }

        if let Some(range) = &self.range {
            self.value = range.clamp(value);
        } else {
            self.value = value;
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.value.is_nan() {
            return Err(format!("Parameter {} contains NaN", self.id));
        }
        if self.value.is_infinite() {
            return Err(format!("Parameter {} contains infinity", self.id));
        }

        if let Some(range) = &self.range {
            if !range.wraps && (self.value < range.min || self.value > range.max) {
                return Err(format!(
                    "Parameter {} value {} outside valid range [{}, {}]",
                    self.id, self.value, range.min, range.max
                ));
            }
        }

        Ok(())
    }

    pub fn is_in_range(&self) -> bool {
        if let Some(range) = &self.range {
            if range.wraps {
                true
            } else {
                self.value >= range.min && self.value <= range.max
            }
        } else {
            true
        }
    }
}

impl ParameterRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            wraps: false,
        }
    }

    pub fn angle() -> Self {
        Self {
            min: 0.0,
            max: 2.0 * std::f64::consts::PI,
            wraps: true,
        }
    }

    pub fn clamp(&self, value: f64) -> f64 {
        if self.wraps {
            let range = self.max - self.min;
            let mut v = value;
            while v < self.min {
                v += range;
            }
            while v > self.max {
                v -= range;
            }
            v
        } else {
            value.max(self.min).min(self.max)
        }
    }
}
