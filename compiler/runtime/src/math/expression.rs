use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Annotated {
        node_id: Option<String>,
        highlight_token: Option<String>,
        expression: Box<Expression>,
    },
    Number(f64),
    Variable(String),
    Unary(UnaryOperator, Box<Expression>),
    Binary(Box<Expression>, BinaryOperator, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
    NumericalDerivative {
        expression: Box<Expression>,
        variable: String,
        step: f64,
    },
}

impl Expression {
    pub fn with_annotation(
        self,
        node_id: Option<String>,
        highlight_token: Option<String>,
    ) -> Self {
        Self::Annotated {
            node_id,
            highlight_token,
            expression: Box::new(self),
        }
    }

    pub fn node_id(&self) -> Option<&str> {
        match self {
            Expression::Annotated { node_id, .. } => node_id.as_deref(),
            _ => None,
        }
    }

    pub fn highlight_token(&self) -> Option<&str> {
        match self {
            Expression::Annotated {
                highlight_token, ..
            } => highlight_token.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Annotated { expression, .. } => write!(f, "{expression}"),
            Expression::Number(n) => write!(f, "{n}"),
            Expression::Variable(name) => write!(f, "{name}"),
            Expression::Unary(UnaryOperator::Negate, expr) => write!(f, "-({expr})"),
            Expression::Binary(lhs, op, rhs) => {
                let symbol = match op {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Subtract => "-",
                    BinaryOperator::Multiply => "*",
                    BinaryOperator::Divide => "/",
                    BinaryOperator::Power => "^",
                };
                write!(f, "({lhs} {symbol} {rhs})")
            }
            Expression::FunctionCall(name, args) => {
                let args = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{name}({args})")
            }
            Expression::NumericalDerivative {
                expression,
                variable,
                step,
            } => write!(f, "num_diff({expression}, {variable}, {step})"),
        }
    }
}
