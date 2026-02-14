use runtime::math::{BinaryOperator, Expression, MathValue, RuntimeMathEngine};
use std::collections::HashMap;

#[test]
fn runtime_math_loop_updates_and_reacts_to_parameter_changes() {
    let mut engine = RuntimeMathEngine::new();
    let expr = Expression::Binary(
        Box::new(Expression::Variable("gain".to_string())),
        BinaryOperator::Multiply,
        Box::new(Expression::FunctionCall(
            "sin".to_string(),
            vec![Expression::Variable("t".to_string())],
        )),
    );

    let mut vars = HashMap::new();
    vars.insert("t".to_string(), 0.0);
    vars.insert("gain".to_string(), 1.0);

    let first = engine
        .evaluate_expression(&expr, &vars)
        .expect("first evaluation");
    let first_value = match first {
        MathValue::Real(v) => v,
        other => panic!("expected real value, got {:?}", other),
    };

    vars.insert("t".to_string(), std::f64::consts::FRAC_PI_2);
    let second = engine
        .evaluate_expression(&expr, &vars)
        .expect("second evaluation");
    let second_value = match second {
        MathValue::Real(v) => v,
        other => panic!("expected real value, got {:?}", other),
    };

    vars.insert("gain".to_string(), 2.0);
    engine.invalidate_caches();
    let third = engine
        .evaluate_expression(&expr, &vars)
        .expect("third evaluation");
    let third_value = match third {
        MathValue::Real(v) => v,
        other => panic!("expected real value, got {:?}", other),
    };

    assert!(second_value > first_value);
    assert!(third_value > second_value);
}
