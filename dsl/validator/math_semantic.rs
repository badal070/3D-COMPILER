/// Math semantic validation pass.
/// Performs baseline type inference for math expressions and checks
/// domain constraints, dimension consistency, and obvious singularities.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector, SourceSpan};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredMathType {
    Real,
    Complex,
    Vector(usize),
    Matrix(usize, usize),
    Unknown,
}

pub struct MathSemanticValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl MathSemanticValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        for material in &ast.materials {
            self.validate_fields(&material.fields);
        }

        for field_def in &ast.fields {
            self.validate_fields(&field_def.fields);
        }

        for entity in &ast.entities {
            for component in &entity.components {
                self.validate_fields(&component.fields);
            }
        }

        for constraint in &ast.constraints {
            self.validate_fields(&constraint.fields);
        }

        for motion in &ast.motions {
            self.validate_fields(&motion.fields);
        }

        for compound in &ast.compound_motions {
            self.validate_fields(&compound.fields);
        }

        for trajectory in &ast.trajectories {
            self.validate_fields(&trajectory.fields);
        }

        for timeline in &ast.timelines {
            for event in &timeline.events {
                self.validate_fields(&event.fields);
            }
        }

        self.errors.into_result(())
    }

    fn validate_fields(&mut self, fields: &[AstField]) {
        for field in fields {
            self.validate_value(&field.value, field.span);
        }
    }

    fn validate_value(&mut self, value: &AstValue, span: SourceSpan) {
        match value {
            AstValue::List(values, _) => {
                for inner in values {
                    self.validate_value(inner, inner.span());
                }
            }
            AstValue::MathExpression(expr, _) => {
                let _ = self.infer_type(expr, span);
            }
            _ => {}
        }
    }

    fn infer_type(&mut self, expr: &MathExpression, span: SourceSpan) -> InferredMathType {
        match expr {
            MathExpression::Variable(_) => InferredMathType::Unknown,
            MathExpression::Constant(c) => match c {
                MathConstant::ImaginaryUnit => InferredMathType::Complex,
                _ => InferredMathType::Real,
            },
            MathExpression::Number(_) => InferredMathType::Real,
            MathExpression::ComplexNumber { .. } => InferredMathType::Complex,
            MathExpression::UnaryOp(_, inner) => self.infer_type(inner, span),
            MathExpression::FunctionCall(name, args) => {
                self.check_function_domain(name, args, span);
                if name == "conjugate" || name == "arg" || name == "abs_complex" {
                    InferredMathType::Complex
                } else {
                    InferredMathType::Real
                }
            }
            MathExpression::BinaryOp(lhs, op, rhs) => {
                let lhs_t = self.infer_type(lhs, span);
                let rhs_t = self.infer_type(rhs, span);
                self.check_binary_constraints(*op, lhs_t, rhs_t, span);
                self.result_type_for_binary(*op, lhs_t, rhs_t)
            }
            MathExpression::Derivative { expression, .. } => self.infer_type(expression, span),
            MathExpression::Integral {
                expression, bounds, ..
            } => {
                if let Some(bounds) = bounds {
                    self.validate_interval(bounds, span);
                }
                self.infer_type(expression, span)
            }
            MathExpression::Limit { expression, .. } => self.infer_type(expression, span),
            MathExpression::Summation {
                expression, bounds, ..
            }
            | MathExpression::Product {
                expression, bounds, ..
            } => {
                self.validate_interval(bounds, span);
                self.infer_type(expression, span)
            }
            MathExpression::Piecewise(cases) => {
                let mut t = InferredMathType::Unknown;
                for (_cond, branch) in cases {
                    let branch_t = self.infer_type(branch, span);
                    if t == InferredMathType::Unknown {
                        t = branch_t;
                    } else if branch_t != InferredMathType::Unknown && t != branch_t {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::DimensionMismatch,
                                "Piecewise branches must have compatible result types".to_string(),
                                span,
                                self.file.clone(),
                            )
                            .with_help(
                                "Ensure all branches evaluate to the same scalar/vector/matrix shape"
                                    .to_string(),
                            ),
                        );
                    }
                }
                t
            }
            MathExpression::MatrixExpr(rows) => {
                let row_count = rows.len();
                let col_count = rows.first().map_or(0, |r| r.len());
                if row_count > 0 && rows.iter().any(|r| r.len() != col_count) {
                    self.errors.add(DslError::new(
                        ErrorCode::DimensionMismatch,
                        "Matrix rows must have identical length".to_string(),
                        span,
                        self.file.clone(),
                    ));
                }
                InferredMathType::Matrix(row_count, col_count)
            }
            MathExpression::VectorExpr(values) => {
                for value in values {
                    let t = self.infer_type(value, span);
                    if t != InferredMathType::Real && t != InferredMathType::Unknown {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::DimensionMismatch,
                                "Vector elements must be scalar values".to_string(),
                                span,
                                self.file.clone(),
                            )
                            .with_help("Use only scalar terms inside vector literals".to_string()),
                        );
                    }
                }
                InferredMathType::Vector(values.len())
            }
        }
    }

    fn check_function_domain(&mut self, name: &str, args: &[MathExpression], span: SourceSpan) {
        if args.is_empty() {
            return;
        }

        match name {
            "sqrt" => {
                if let Some(value) = self.eval_constant(&args[0]) {
                    if value < 0.0 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::InvalidMathDomain,
                                "sqrt(x) is undefined for x < 0 in the real domain".to_string(),
                                span,
                                self.file.clone(),
                            )
                            .with_help("Restrict the input domain to x >= 0".to_string()),
                        );
                    }
                }
            }
            "log" | "ln" => {
                if let Some(value) = self.eval_constant(&args[0]) {
                    if value <= 0.0 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::InvalidMathDomain,
                                "log(x) is undefined for x <= 0 in the real domain".to_string(),
                                span,
                                self.file.clone(),
                            )
                            .with_help("Restrict the input domain to x > 0".to_string()),
                        );
                    }
                }
            }
            "tan" => {
                if let Some(value) = self.eval_constant(&args[0]) {
                    let k = (value / std::f64::consts::FRAC_PI_2).round();
                    let near = (value - k * std::f64::consts::FRAC_PI_2).abs() < 1e-9;
                    if near && (k as i64) % 2 != 0 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::PotentialSingularity,
                                "tan(x) has a singularity at odd multiples of π/2".to_string(),
                                span,
                                self.file.clone(),
                            )
                            .with_help("Avoid x = (2k+1)π/2 or split the domain".to_string()),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn validate_interval(&mut self, interval: &IntervalConstraint, span: SourceSpan) {
        let lower = self.eval_constant(&interval.lower);
        let upper = self.eval_constant(&interval.upper);

        if let (Some(l), Some(u)) = (lower, upper) {
            if l > u {
                self.errors.add(
                    DslError::new(
                        ErrorCode::InvalidMathDomain,
                        "Interval lower bound must not exceed upper bound".to_string(),
                        span,
                        self.file.clone(),
                    )
                    .with_help("Swap bounds or fix the expression ordering".to_string()),
                );
            }
        }
    }

    fn check_binary_constraints(
        &mut self,
        op: MathBinaryOperator,
        lhs: InferredMathType,
        rhs: InferredMathType,
        span: SourceSpan,
    ) {
        match op {
            MathBinaryOperator::Add | MathBinaryOperator::Subtract => {
                if !self.are_compatible(lhs, rhs) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::DimensionMismatch,
                            format!(
                                "Incompatible types for {:?}: left={:?}, right={:?}",
                                op, lhs, rhs
                            ),
                            span,
                            self.file.clone(),
                        )
                        .with_help(
                            "Addition/subtraction require matching scalar/vector/matrix shapes"
                                .to_string(),
                        ),
                    );
                }
            }
            MathBinaryOperator::Divide => {
                if let InferredMathType::Vector(_) | InferredMathType::Matrix(_, _) = rhs {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::DimensionMismatch,
                            "Division by vector/matrix is not supported in baseline semantics"
                                .to_string(),
                            span,
                            self.file.clone(),
                        )
                        .with_help("Use scalar denominators for division".to_string()),
                    );
                }
            }
            MathBinaryOperator::Dot => match (lhs, rhs) {
                (InferredMathType::Vector(a), InferredMathType::Vector(b)) if a == b => {}
                _ => self.errors.add(
                    DslError::new(
                        ErrorCode::DimensionMismatch,
                        "Dot product requires vectors of equal dimension".to_string(),
                        span,
                        self.file.clone(),
                    )
                    .with_help("Use vectors with matching dimensions for dot product".to_string()),
                ),
            },
            MathBinaryOperator::Cross => match (lhs, rhs) {
                (InferredMathType::Vector(3), InferredMathType::Vector(3)) => {}
                _ => self.errors.add(
                    DslError::new(
                        ErrorCode::DimensionMismatch,
                        "Cross product is defined only for 3D vectors".to_string(),
                        span,
                        self.file.clone(),
                    )
                    .with_help("Use 3D vectors for cross product".to_string()),
                ),
            },
            _ => {}
        }
    }

    fn are_compatible(&self, a: InferredMathType, b: InferredMathType) -> bool {
        if a == InferredMathType::Unknown || b == InferredMathType::Unknown {
            return true;
        }
        a == b
            || (a == InferredMathType::Real && b == InferredMathType::Complex)
            || (a == InferredMathType::Complex && b == InferredMathType::Real)
    }

    fn result_type_for_binary(
        &self,
        op: MathBinaryOperator,
        lhs: InferredMathType,
        rhs: InferredMathType,
    ) -> InferredMathType {
        match op {
            MathBinaryOperator::Dot => InferredMathType::Real,
            MathBinaryOperator::Cross => InferredMathType::Vector(3),
            MathBinaryOperator::Add | MathBinaryOperator::Subtract => {
                if lhs == InferredMathType::Complex || rhs == InferredMathType::Complex {
                    InferredMathType::Complex
                } else if lhs != InferredMathType::Unknown {
                    lhs
                } else {
                    rhs
                }
            }
            MathBinaryOperator::Multiply => match (lhs, rhs) {
                (InferredMathType::Real, t) | (t, InferredMathType::Real) => t,
                (InferredMathType::Complex, _) | (_, InferredMathType::Complex) => {
                    InferredMathType::Complex
                }
                _ => InferredMathType::Unknown,
            },
            MathBinaryOperator::Divide => match lhs {
                InferredMathType::Unknown => InferredMathType::Real,
                _ => lhs,
            },
            MathBinaryOperator::Power => lhs,
        }
    }

    fn eval_constant(&self, expr: &MathExpression) -> Option<f64> {
        match expr {
            MathExpression::Number(n) => Some(*n),
            MathExpression::Constant(c) => match c {
                MathConstant::Pi => Some(std::f64::consts::PI),
                MathConstant::Euler => Some(std::f64::consts::E),
                MathConstant::Infinity => Some(f64::INFINITY),
                MathConstant::ImaginaryUnit => None,
            },
            MathExpression::UnaryOp(MathUnaryOperator::Negate, inner) => {
                self.eval_constant(inner).map(|v| -v)
            }
            MathExpression::BinaryOp(lhs, op, rhs) => {
                let l = self.eval_constant(lhs)?;
                let r = self.eval_constant(rhs)?;
                match op {
                    MathBinaryOperator::Add => Some(l + r),
                    MathBinaryOperator::Subtract => Some(l - r),
                    MathBinaryOperator::Multiply => Some(l * r),
                    MathBinaryOperator::Divide => {
                        if r == 0.0 {
                            None
                        } else {
                            Some(l / r)
                        }
                    }
                    MathBinaryOperator::Power => Some(l.powf(r)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::SourceSpan;

    #[test]
    fn test_constant_eval() {
        let validator = MathSemanticValidator::new(PathBuf::from("test.dsl"));
        let expr = MathExpression::BinaryOp(
            Box::new(MathExpression::Number(2.0)),
            MathBinaryOperator::Multiply,
            Box::new(MathExpression::Number(4.0)),
        );
        assert_eq!(validator.eval_constant(&expr), Some(8.0));
    }

    #[test]
    fn test_interval_validation() {
        let mut validator = MathSemanticValidator::new(PathBuf::from("test.dsl"));
        let bad = IntervalConstraint {
            lower: Box::new(MathExpression::Number(5.0)),
            upper: Box::new(MathExpression::Number(1.0)),
            lower_inclusive: true,
            upper_inclusive: true,
        };
        validator.validate_interval(&bad, SourceSpan::single_point(1, 1, 0));
        assert!(validator.errors.has_errors());
    }
}
