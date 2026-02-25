use dsl_compiler::ast::{
    AnnotatedExpr, AstFile, AstLibraryImports, AstMotion, AstScene, AstValue, MathBinaryOperator,
    MathExpression,
};
use dsl_compiler::errors::SourceSpan;
use dsl_compiler::lower_to_ir::{IrLowering, IrValue};

fn span() -> SourceSpan {
    SourceSpan::single_point(1, 1, 0)
}

fn mk_expr(expr: MathExpression) -> AnnotatedExpr {
    AnnotatedExpr {
        node_id: "test_expr".to_string(),
        highlight_token: None,
        expr,
    }
}

#[test]
fn lowers_math_expression_into_ir_payload() {
    let s = span();
    let expr = MathExpression::BinaryOp(
        Box::new(mk_expr(MathExpression::Number(2.0))),
        MathBinaryOperator::Add,
        Box::new(mk_expr(MathExpression::Number(3.0))),
    );

    let ast = AstFile {
        scene: AstScene {
            name: "test".to_string(),
            version: 1,
            ir_version: "0.1.0".to_string(),
            unit_system: "SI".to_string(),
            domain: None,
            span: s,
        },
        library_imports: AstLibraryImports {
            imports: vec![],
            span: s,
        },
        materials: vec![],
        fields: vec![],
        entities: vec![],
        constraints: vec![],
        motions: vec![AstMotion {
            name: "m1".to_string(),
            fields: vec![
                dsl_compiler::ast::AstField {
                    name: "type".to_string(),
                    value: AstValue::Identifier("rotation".to_string(), s),
                    span: s,
                },
                dsl_compiler::ast::AstField {
                    name: "target".to_string(),
                    value: AstValue::Identifier("obj1".to_string(), s),
                    span: s,
                },
                dsl_compiler::ast::AstField {
                    name: "expr".to_string(),
                    value: AstValue::MathExpression(mk_expr(expr), s),
                    span: s,
                },
            ],
            span: s,
        }],
        math_objects: vec![],
        compound_motions: vec![],
        trajectories: vec![],
        timelines: vec![],
        concept_ref: None,
        annotations: vec![],
        highlight_schedule: vec![],
        span: s,
    };

    let ir = IrLowering::lower(ast).expect("lowering should succeed");
    let motion = &ir.motions[0];
    let value = motion
        .parameters
        .get("expr")
        .expect("expr parameter exists");

    match value {
        IrValue::MathExpression(payload) => {
            assert_eq!(payload.expression_type, "binary");
            assert!(payload.complexity >= 3);
            assert!(payload.source.contains("2"));
        }
        other => panic!("expected math expression payload, found {other:?}"),
    }
}

#[test]
fn lowers_list_values() {
    let s = span();
    let ast = AstFile {
        scene: AstScene {
            name: "test".to_string(),
            version: 1,
            ir_version: "0.1.0".to_string(),
            unit_system: "SI".to_string(),
            domain: None,
            span: s,
        },
        library_imports: AstLibraryImports {
            imports: vec![],
            span: s,
        },
        materials: vec![],
        fields: vec![],
        entities: vec![],
        constraints: vec![],
        motions: vec![AstMotion {
            name: "m1".to_string(),
            fields: vec![
                dsl_compiler::ast::AstField {
                    name: "type".to_string(),
                    value: AstValue::Identifier("rotation".to_string(), s),
                    span: s,
                },
                dsl_compiler::ast::AstField {
                    name: "target".to_string(),
                    value: AstValue::Identifier("obj1".to_string(), s),
                    span: s,
                },
                dsl_compiler::ast::AstField {
                    name: "vals".to_string(),
                    value: AstValue::List(
                        vec![
                            AstValue::Number(1.0, s),
                            AstValue::Identifier("true".to_string(), s),
                        ],
                        s,
                    ),
                    span: s,
                },
            ],
            span: s,
        }],
        math_objects: vec![],
        compound_motions: vec![],
        trajectories: vec![],
        timelines: vec![],
        concept_ref: None,
        annotations: vec![],
        highlight_schedule: vec![],
        span: s,
    };

    let ir = IrLowering::lower(ast).expect("lowering should succeed");
    match ir.motions[0].parameters.get("vals").unwrap() {
        IrValue::List(values) => assert_eq!(values.len(), 2),
        other => panic!("expected list payload, found {other:?}"),
    }
}
