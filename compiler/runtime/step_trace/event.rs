// compiler/runtime/step_trace/event.rs
// Defines MathStepEvent — one emitted record per sub-expression evaluation

use serde::{Deserialize, Serialize};

/// Describes the type of mathematical operation that produced this step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepOperation {
    /// Binary addition
    Add,
    /// Binary subtraction
    Subtract,
    /// Binary multiplication
    Multiply,
    /// Binary division
    Divide,
    /// Binary exponentiation
    Power,
    /// Unary negation
    Negate,
    /// Function call (e.g., sin, cos, exp, ln, sqrt)
    FunctionCall { name: String },
    /// Variable substitution with its current value
    Substitute { variable: String },
    /// Derivative application
    Derivative { variable: String, order: usize },
    /// Integral evaluation
    Integral { variable: String },
    /// Limit evaluation
    Limit { variable: String, approach: f64 },
    /// Named constant (π, e, etc.)
    Constant { name: String },
}

impl StepOperation {
    /// Generate a human-readable description of this operation
    fn describe(
        &self,
        left_value: Option<f64>,
        right_value: Option<f64>,
        result: f64,
        bindings: &[(String, f64)],
    ) -> String {
        match self {
            StepOperation::Add => {
                match (left_value, right_value) {
                    (Some(l), Some(r)) => {
                        format!("Adding {} and {} to get {}", l, r, result)
                    }
                    _ => "Adding values".to_string(),
                }
            }
            StepOperation::Subtract => {
                match (left_value, right_value) {
                    (Some(l), Some(r)) => {
                        format!("Subtracting {} from {} to get {}", r, l, result)
                    }
                    _ => "Subtracting values".to_string(),
                }
            }
            StepOperation::Multiply => {
                match (left_value, right_value) {
                    (Some(l), Some(r)) => {
                        format!("Multiplying {} by {} to get {}", l, r, result)
                    }
                    _ => "Multiplying values".to_string(),
                }
            }
            StepOperation::Divide => {
                match (left_value, right_value) {
                    (Some(l), Some(r)) => {
                        format!("Dividing {} by {} to get {}", l, r, result)
                    }
                    _ => "Dividing values".to_string(),
                }
            }
            StepOperation::Power => {
                match (left_value, right_value) {
                    (Some(l), Some(r)) => {
                        format!("Raising {} to the power of {} to get {}", l, r, result)
                    }
                    _ => "Raising to a power".to_string(),
                }
            }
            StepOperation::Negate => {
                format!("Negating the value to get {}", result)
            }
            StepOperation::FunctionCall { name } => {
                match left_value {
                    Some(val) => format!("Evaluating {}({}) to get {}", name, val, result),
                    None => format!("Evaluating {}", name),
                }
            }
            StepOperation::Substitute { variable } => {
                if let Some((_, val)) = bindings.iter().find(|(k, _)| k == variable) {
                    format!("Substituting {} = {} into the expression", variable, val)
                } else {
                    format!("Substituting {} into the expression", variable)
                }
            }
            StepOperation::Derivative { variable, order } => {
                if *order == 1 {
                    format!("Taking the derivative with respect to {}", variable)
                } else {
                    format!("Taking the {}-order derivative with respect to {}", order, variable)
                }
            }
            StepOperation::Integral { variable } => {
                format!("Integrating with respect to {}", variable)
            }
            StepOperation::Limit { variable, approach } => {
                format!("Taking the limit as {} approaches {}", variable, approach)
            }
            StepOperation::Constant { name } => {
                format!("Evaluating constant {} to get {}", name, result)
            }
        }
    }
}

/// One emitted record per sub-expression evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathStepEvent {
    /// The AnnotatedExpr.node_id from the AST
    /// Stable identifier across AST, IR, runtime, and frontend
    pub node_id: Option<String>,

    /// Pedagogically assigned token if this node is a teaching focus
    /// Comes from AnnotatedExpr.highlight_token
    /// Most nodes have None; only LLM-marked nodes carry a token
    pub highlight_token: Option<String>,

    /// Description of the mathematical operation
    pub operation: StepOperation,

    /// Left operand if binary; None for unary and function calls
    pub left_value: Option<f64>,

    /// Right operand if binary; None otherwise
    pub right_value: Option<f64>,

    /// Computed numerical result of this node
    pub result_value: f64,

    /// Snapshot of all variable values active at evaluation time
    /// Used by Equation Panel to show substitution values
    pub variable_bindings: Vec<(String, f64)>,

    /// Plain-English sentence describing this step
    /// Auto-generated from operation data
    pub description: String,

    /// Name of the mathematical rule if one was applied
    /// Examples: "Chain Rule", "Product Rule", "L'Hôpital's Rule"
    /// None for arithmetic operations
    pub rule_applied: Option<String>,

    /// False if result is NaN or infinite (triggers warning display)
    pub is_finite: bool,

    /// Global index within the current MathStepTrace
    /// Set by StepEmitter when event is registered
    pub step_index: usize,
}

impl MathStepEvent {
    /// Create a new MathStepEvent with auto-generated description
    pub fn new(
        node_id: Option<String>,
        operation: StepOperation,
        result: f64,
        bindings: Vec<(String, f64)>,
    ) -> Self {
        let is_finite = result.is_finite();
        let description = operation.describe(None, None, result, &bindings);

        Self {
            node_id,
            highlight_token: None,
            operation,
            left_value: None,
            right_value: None,
            result_value: result,
            variable_bindings: bindings,
            description,
            rule_applied: None,
            is_finite,
            step_index: 0,
        }
    }

    /// Create a new event with both operands
    pub fn binary(
        node_id: Option<String>,
        operation: StepOperation,
        left: f64,
        right: f64,
        result: f64,
        bindings: Vec<(String, f64)>,
    ) -> Self {
        let is_finite = result.is_finite();
        let description = operation.describe(Some(left), Some(right), result, &bindings);

        Self {
            node_id,
            highlight_token: None,
            operation,
            left_value: Some(left),
            right_value: Some(right),
            result_value: result,
            variable_bindings: bindings,
            description,
            rule_applied: None,
            is_finite,
            step_index: 0,
        }
    }

    /// Attach a highlight token to this event (builder pattern)
    pub fn with_highlight(mut self, token: String) -> Self {
        self.highlight_token = Some(token);
        self
    }

    /// Attach a rule name to this event (builder pattern)
    pub fn with_rule(mut self, rule: String) -> Self {
        self.rule_applied = Some(rule);
        self
    }

    /// Manually set the step index
    pub fn with_step_index(mut self, index: usize) -> Self {
        self.step_index = index;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_operation() {
        let event = MathStepEvent::binary(
            Some("node_1".to_string()),
            StepOperation::Add,
            3.0,
            4.0,
            7.0,
            vec![],
        );
        assert!(event.description.contains("Adding"));
        assert_eq!(event.result_value, 7.0);
    }

    #[test]
    fn test_highlight_token() {
        let event = MathStepEvent::new(
            Some("node_1".to_string()),
            StepOperation::Add,
            5.0,
            vec![],
        )
        .with_highlight("hk_01".to_string());

        assert_eq!(event.highlight_token, Some("hk_01".to_string()));
    }

    #[test]
    fn test_infinity_check() {
        let event = MathStepEvent::new(
            None,
            StepOperation::Divide,
            f64::INFINITY,
            vec![],
        );
        assert!(!event.is_finite);
    }
}
