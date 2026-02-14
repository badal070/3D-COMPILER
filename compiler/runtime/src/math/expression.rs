use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
