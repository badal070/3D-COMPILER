use crate::error::{RuntimeError, RuntimeResult};
use crate::math::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::math::types::MathValue;
use crate::step_trace::StepEmitter;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type VariableMap = HashMap<String, f64>;

#[derive(Debug, Default)]
pub struct RuntimeMathEngine {
    function_cache: HashMap<String, MathValue>,
    derivative_cache: HashMap<(String, String), Expression>,
    integral_cache: HashMap<(String, String, u64, u64), f64>,
    step_emitter: Option<Arc<Mutex<StepEmitter>>>,
}

impl RuntimeMathEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_emitter(emitter: Arc<Mutex<StepEmitter>>) -> Self {
        let mut engine = Self::new();
        engine.step_emitter = Some(emitter);
        engine
    }

    pub fn set_emitter(&mut self, emitter: Arc<Mutex<StepEmitter>>) {
        self.step_emitter = Some(emitter);
    }

    pub fn evaluate_expression(
        &mut self,
        expr: &Expression,
        variables: &VariableMap,
    ) -> RuntimeResult<MathValue> {
        let cache_key = format!("{expr}|{:?}", variables);
        if let Some(cached) = self.function_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let value = MathValue::Real(self.evaluate_scalar(expr, variables)?);
        self.function_cache.insert(cache_key, value.clone());
        Ok(value)
    }

    pub fn evaluate_derivative(
        &mut self,
        expr: &Expression,
        variable: &str,
    ) -> RuntimeResult<Expression> {
        let expr_key = expr.to_string();
        let cache_key = (expr_key.clone(), variable.to_string());
        if let Some(cached) = self.derivative_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let symbolic = self.symbolic_derivative(expr, variable);
        let derivative = symbolic.unwrap_or(Expression::NumericalDerivative {
            expression: Box::new(expr.clone()),
            variable: variable.to_string(),
            step: 1e-5,
        });

        self.derivative_cache.insert(cache_key, derivative.clone());
        Ok(derivative)
    }

    pub fn evaluate_integral(
        &mut self,
        expr: &Expression,
        variable: &str,
        lower: f64,
        upper: f64,
    ) -> RuntimeResult<f64> {
        let cache_key = (
            expr.to_string(),
            variable.to_string(),
            lower.to_bits(),
            upper.to_bits(),
        );
        if let Some(cached) = self.integral_cache.get(&cache_key) {
            return Ok(*cached);
        }

        let integral = if let Some(antiderivative) = self.symbolic_antiderivative(expr, variable) {
            let mut upper_vars = HashMap::new();
            upper_vars.insert(variable.to_string(), upper);
            let mut lower_vars = HashMap::new();
            lower_vars.insert(variable.to_string(), lower);

            let upper_value = self.evaluate_scalar(&antiderivative, &upper_vars)?;
            let lower_value = self.evaluate_scalar(&antiderivative, &lower_vars)?;
            upper_value - lower_value
        } else {
            self.integrate_simpson(expr, variable, lower, upper, 256)?
        };

        self.integral_cache.insert(cache_key, integral);
        Ok(integral)
    }

    pub fn invalidate_caches(&mut self) {
        self.function_cache.clear();
        self.derivative_cache.clear();
        self.integral_cache.clear();
    }

    fn evaluate_scalar(&self, expr: &Expression, variables: &VariableMap) -> RuntimeResult<f64> {
        let (node_id, highlight_token, expression) = match expr {
            Expression::Annotated {
                node_id,
                highlight_token,
                expression,
            } => (node_id.clone(), highlight_token.clone(), expression.as_ref()),
            _ => (None, None, expr),
        };

        match expression {
            Expression::Number(n) => {
                self.emit_step(
                    node_id,
                    highlight_token,
                    "number",
                    None,
                    None,
                    *n,
                    variables,
                    format!("literal {n}"),
                );
                Ok(*n)
            }
            Expression::Variable(name) => {
                let value = variables.get(name).copied().ok_or_else(|| {
                    RuntimeError::Configuration(format!("Missing variable value for '{name}'"))
                })?;
                self.emit_step(
                    node_id,
                    highlight_token,
                    "substitute",
                    None,
                    None,
                    value,
                    variables,
                    format!("substitute {name} = {value}"),
                );
                Ok(value)
            }
            Expression::Unary(UnaryOperator::Negate, inner) => {
                let value = -self.evaluate_scalar(inner, variables)?;
                self.emit_step(
                    node_id,
                    highlight_token,
                    "negate",
                    None,
                    None,
                    value,
                    variables,
                    "apply unary negation".to_string(),
                );
                Ok(value)
            }
            Expression::Binary(lhs, op, rhs) => {
                let l = self.evaluate_scalar(lhs, variables)?;
                let r = self.evaluate_scalar(rhs, variables)?;
                let (op_name, result) = match op {
                    BinaryOperator::Add => ("add", l + r),
                    BinaryOperator::Subtract => ("subtract", l - r),
                    BinaryOperator::Multiply => ("multiply", l * r),
                    BinaryOperator::Divide => {
                        if r == 0.0 {
                            return Err(RuntimeError::Configuration(
                                "Division by zero in math expression".to_string(),
                            ));
                        }
                        ("divide", l / r)
                    }
                    BinaryOperator::Power => ("power", l.powf(r)),
                };
                self.emit_step(
                    node_id,
                    highlight_token,
                    op_name,
                    Some(l),
                    Some(r),
                    result,
                    variables,
                    format!("{op_name} operands"),
                );
                Ok(result)
            }
            Expression::FunctionCall(name, args) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.evaluate_scalar(arg, variables)?);
                }

                let result = match (name.as_str(), values.as_slice()) {
                    ("sin", [x]) => Ok(x.sin()),
                    ("cos", [x]) => Ok(x.cos()),
                    ("tan", [x]) => Ok(x.tan()),
                    ("exp", [x]) => Ok(x.exp()),
                    ("sqrt", [x]) => Ok(x.sqrt()),
                    ("ln", [x]) => Ok(x.ln()),
                    ("log", [x]) => Ok(x.ln()),
                    _ => Err(RuntimeError::Configuration(format!(
                        "Unsupported function call '{name}'"
                    ))),
                }?;
                self.emit_step(
                    node_id,
                    highlight_token,
                    name,
                    None,
                    None,
                    result,
                    variables,
                    format!("evaluate function {name}"),
                );
                Ok(result)
            }
            Expression::NumericalDerivative {
                expression,
                variable,
                step,
            } => {
                let center = variables.get(variable).copied().ok_or_else(|| {
                    RuntimeError::Configuration(format!(
                        "Missing variable value for derivative '{variable}'"
                    ))
                })?;

                let mut left_vars = variables.clone();
                left_vars.insert(variable.clone(), center - step);
                let mut right_vars = variables.clone();
                right_vars.insert(variable.clone(), center + step);

                let left = self.evaluate_scalar(expression, &left_vars)?;
                let right = self.evaluate_scalar(expression, &right_vars)?;
                let result = (right - left) / (2.0 * step);
                self.emit_step(
                    node_id,
                    highlight_token,
                    "numerical_derivative",
                    Some(left),
                    Some(right),
                    result,
                    variables,
                    format!("central difference derivative for {variable}"),
                );
                Ok(result)
            }
            Expression::Annotated { .. } => unreachable!("annotated expression is unwrapped above"),
        }
    }

    fn symbolic_derivative(&self, expr: &Expression, variable: &str) -> Option<Expression> {
        match expr {
            Expression::Annotated {
                node_id,
                highlight_token,
                expression,
            } => Some(
                self.symbolic_derivative(expression, variable)?
                    .with_annotation(node_id.clone(), highlight_token.clone()),
            ),
            Expression::Number(_) => Some(Expression::Number(0.0)),
            Expression::Variable(name) => {
                Some(Expression::Number(if name == variable { 1.0 } else { 0.0 }))
            }
            Expression::Unary(UnaryOperator::Negate, inner) => Some(Expression::Unary(
                UnaryOperator::Negate,
                Box::new(self.symbolic_derivative(inner, variable)?),
            )),
            Expression::Binary(lhs, op, rhs) => match op {
                BinaryOperator::Add => Some(Expression::Binary(
                    Box::new(self.symbolic_derivative(lhs, variable)?),
                    BinaryOperator::Add,
                    Box::new(self.symbolic_derivative(rhs, variable)?),
                )),
                BinaryOperator::Subtract => Some(Expression::Binary(
                    Box::new(self.symbolic_derivative(lhs, variable)?),
                    BinaryOperator::Subtract,
                    Box::new(self.symbolic_derivative(rhs, variable)?),
                )),
                BinaryOperator::Multiply => Some(Expression::Binary(
                    Box::new(Expression::Binary(
                        Box::new(self.symbolic_derivative(lhs, variable)?),
                        BinaryOperator::Multiply,
                        Box::new((**rhs).clone()),
                    )),
                    BinaryOperator::Add,
                    Box::new(Expression::Binary(
                        Box::new((**lhs).clone()),
                        BinaryOperator::Multiply,
                        Box::new(self.symbolic_derivative(rhs, variable)?),
                    )),
                )),
                BinaryOperator::Power => {
                    if let Expression::Number(n) = **rhs {
                        // d/dx (u^n) = n * u^(n-1) * du/dx for constant n.
                        Some(Expression::Binary(
                            Box::new(Expression::Binary(
                                Box::new(Expression::Number(n)),
                                BinaryOperator::Multiply,
                                Box::new(Expression::Binary(
                                    Box::new((**lhs).clone()),
                                    BinaryOperator::Power,
                                    Box::new(Expression::Number(n - 1.0)),
                                )),
                            )),
                            BinaryOperator::Multiply,
                            Box::new(self.symbolic_derivative(lhs, variable)?),
                        ))
                    } else {
                        None
                    }
                }
                BinaryOperator::Divide => None,
            },
            Expression::FunctionCall(name, args) => {
                if args.len() != 1 {
                    return None;
                }
                let x = args[0].clone();
                let dx = self.symbolic_derivative(&x, variable)?;
                match name.as_str() {
                    "sin" => Some(Expression::Binary(
                        Box::new(Expression::FunctionCall("cos".to_string(), vec![x])),
                        BinaryOperator::Multiply,
                        Box::new(dx),
                    )),
                    "cos" => Some(Expression::Binary(
                        Box::new(Expression::Unary(
                            UnaryOperator::Negate,
                            Box::new(Expression::FunctionCall("sin".to_string(), vec![x])),
                        )),
                        BinaryOperator::Multiply,
                        Box::new(dx),
                    )),
                    "exp" => Some(Expression::Binary(
                        Box::new(Expression::FunctionCall("exp".to_string(), vec![x])),
                        BinaryOperator::Multiply,
                        Box::new(dx),
                    )),
                    _ => None,
                }
            }
            Expression::NumericalDerivative { .. } => None,
        }
    }

    fn symbolic_antiderivative(&self, expr: &Expression, variable: &str) -> Option<Expression> {
        match expr {
            Expression::Annotated {
                node_id,
                highlight_token,
                expression,
            } => Some(
                self.symbolic_antiderivative(expression, variable)?
                    .with_annotation(node_id.clone(), highlight_token.clone()),
            ),
            Expression::Number(c) => Some(Expression::Binary(
                Box::new(Expression::Number(*c)),
                BinaryOperator::Multiply,
                Box::new(Expression::Variable(variable.to_string())),
            )),
            Expression::Variable(name) if name == variable => Some(Expression::Binary(
                Box::new(Expression::Binary(
                    Box::new(Expression::Variable(variable.to_string())),
                    BinaryOperator::Power,
                    Box::new(Expression::Number(2.0)),
                )),
                BinaryOperator::Divide,
                Box::new(Expression::Number(2.0)),
            )),
            _ => None,
        }
    }

    fn emit_step(
        &self,
        node_id: Option<String>,
        highlight_token: Option<String>,
        operation_name: &str,
        left_value: Option<f64>,
        right_value: Option<f64>,
        result: f64,
        variables: &VariableMap,
        description: String,
    ) {
        if let Some(emitter) = &self.step_emitter {
            if let Ok(mut guard) = emitter.lock() {
                // Convert operation name string to StepOperation enum
                let operation = match operation_name {
                    "add" => crate::step_trace::StepOperation::Add,
                    "subtract" => crate::step_trace::StepOperation::Subtract,
                    "multiply" => crate::step_trace::StepOperation::Multiply,
                    "divide" => crate::step_trace::StepOperation::Divide,
                    "power" => crate::step_trace::StepOperation::Power,
                    "negate" => crate::step_trace::StepOperation::Negate,
                    "substitute" => {
                        // Extract variable name from description if possible
                        let var_name = description
                            .split('=')
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .unwrap_or("unknown")
                            .to_string();
                        crate::step_trace::StepOperation::Substitute { variable: var_name }
                    }
                    "number" => crate::step_trace::StepOperation::Constant {
                        name: "literal".to_string(),
                    },
                    name if name.contains("derivative") => {
                        crate::step_trace::StepOperation::Derivative {
                            variable: "x".to_string(),
                            order: 1,
                        }
                    }
                    name if name.contains("function") => {
                        let func_name = name
                            .split('_')
                            .last()
                            .unwrap_or("unknown")
                            .to_string();
                        crate::step_trace::StepOperation::FunctionCall {
                            name: func_name,
                        }
                    }
                    _ => crate::step_trace::StepOperation::Constant {
                        name: operation_name.to_string(),
                    },
                };

                let mut event = if let (Some(l), Some(r)) = (left_value, right_value) {
                    crate::step_trace::MathStepEvent::binary(node_id, operation, l, r, result, vec![])
                } else {
                    crate::step_trace::MathStepEvent::new(node_id, operation, result, vec![])
                };

                event.highlight_token = highlight_token;
                event.description = description;
                event.variable_bindings = variables
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();

                guard.emit(event);
            }
        }
    }

    fn integrate_simpson(
        &self,
        expr: &Expression,
        variable: &str,
        lower: f64,
        upper: f64,
        n: usize,
    ) -> RuntimeResult<f64> {
        let n = if n % 2 == 0 { n } else { n + 1 };
        let h = (upper - lower) / n as f64;
        let mut sum = 0.0;

        for i in 0..=n {
            let x = lower + i as f64 * h;
            let mut vars = HashMap::new();
            vars.insert(variable.to_string(), x);

            let fx = self.evaluate_scalar(expr, &vars)?;
            let weight = if i == 0 || i == n {
                1.0
            } else if i % 2 == 0 {
                2.0
            } else {
                4.0
            };
            sum += weight * fx;
        }

        Ok(sum * h / 3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_arithmetic_expression() {
        let mut engine = RuntimeMathEngine::new();
        let expr = Expression::Binary(
            Box::new(Expression::Number(2.0)),
            BinaryOperator::Add,
            Box::new(Expression::Binary(
                Box::new(Expression::Number(3.0)),
                BinaryOperator::Multiply,
                Box::new(Expression::Number(4.0)),
            )),
        );
        let value = engine.evaluate_expression(&expr, &HashMap::new()).unwrap();
        assert_eq!(value, MathValue::Real(14.0));
    }

    #[test]
    fn test_symbolic_derivative_path() {
        let mut engine = RuntimeMathEngine::new();
        let expr = Expression::FunctionCall(
            "sin".to_string(),
            vec![Expression::Variable("x".to_string())],
        );
        let derivative = engine.evaluate_derivative(&expr, "x").unwrap();
        assert!(matches!(derivative, Expression::Binary(_, _, _)));
    }

    #[test]
    fn test_integral_fallback_numeric() {
        let mut engine = RuntimeMathEngine::new();
        let expr = Expression::FunctionCall(
            "sin".to_string(),
            vec![Expression::Variable("x".to_string())],
        );
        let value = engine
            .evaluate_integral(&expr, "x", 0.0, std::f64::consts::PI)
            .unwrap();
        assert!((value - 2.0).abs() < 1e-3);
    }
}
