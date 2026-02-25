/// Lowering pass from validated AST to IR.
/// This is where DSL constructs become IR constructs.
/// 1:1 mapping, no semantic interpretation.
/// Pure function - validated AST in, IR out.
use crate::ast::*;
use crate::errors::{DslError, DslResult, ErrorCode};
use serde::Serialize;
use std::collections::HashMap;

/// Intermediate Representation - this would typically be defined in a separate IR module
/// For now, we define a minimal IR structure to demonstrate the lowering process

#[derive(Serialize)]
pub struct IrScene {
    pub metadata: IrMetadata,
    pub entities: Vec<IrEntity>,
    pub constraints: Vec<IrConstraint>,
    pub motions: Vec<IrMotion>,
    pub math_entities: Vec<IrMathEntity>,
    pub compound_motions: Vec<IrCompoundMotion>,
    pub timelines: Vec<IrTimeline>,
    pub annotations: Vec<IrAnnotation>,
    pub highlight_schedule: Vec<IrHighlightEntry>,
    pub concept_ref: Option<IrConceptRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMetadata {
    pub name: String,
    pub version: i64,
    pub ir_version: String,
    pub unit_system: String,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEntity {
    pub id: String,
    pub kind: String,
    pub components: HashMap<String, IrComponent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrComponent {
    pub component_type: String,
    pub properties: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub enum IrValue {
    Number(f64),
    String(String),
    Identifier(String),
    Vector3([f64; 3]),
    Boolean(bool),
    Matrix3([[f64; 3]; 3]),
    List(Vec<IrValue>),
    MathExpression(IrMathExpression),
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMathExpression {
    pub expression_type: String,
    pub source: String,
    pub complexity: usize,
    pub node_id: String,
    pub highlight_token: Option<String>,
    pub children: Vec<IrMathExpression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrAnnotation {
    pub label_text: String,
    pub anchor_entity_id: String,
    pub position_offset: [f64; 3],
    pub equation_node_id: Option<String>,
    pub highlight_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrHighlightEntry {
    pub at_time: f64,
    pub highlight_token: String,
    pub entity_id: String,
    pub color_index: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrConceptRef {
    pub concept_id: String,
    pub section_id: String,
    pub step_index: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrConstraint {
    pub id: String,
    pub constraint_type: String,
    pub parameters: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMotion {
    pub id: String,
    pub motion_type: String,
    pub target_entity: String,
    pub parameters: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrCompoundMotion {
    /// Identifier of this compound motion
    pub id: String,
    /// Composition type: sequential, parallel, conditional, etc.
    pub compound_type: String,
    /// Ordered list of referenced motion ids
    pub motions: Vec<String>,
    /// Additional parameters specific to the composition strategy
    pub parameters: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTimeline {
    pub id: String,
    pub events: Vec<IrEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEvent {
    pub motion_id: String,
    pub start_time: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize)]
pub enum IrMathEntity {
    Function(IrFunctionNode),
    Curve(IrCurveNode),
    Surface(IrSurfaceNode),
    Field(IrFieldNode),
    Ode(IrOdeNode),
    Transformation(IrTransformationNode),
    Matrix(IrMatrixNode),
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMathDomain {
    pub space: String,
    pub variables: Vec<String>,
    pub constraints: Vec<IrMathExpression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFunctionNode {
    pub id: String,
    pub parameters: Vec<String>,
    pub body: IrMathExpression,
    pub domain: IrMathDomain,
    pub range_constraints: Vec<IrMathExpression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrCurveNode {
    pub id: String,
    pub curve_type: String,
    pub definition: IrMathExpression,
    pub parameter: Option<String>,
    pub domain: IrMathDomain,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrSurfaceNode {
    pub id: String,
    pub surface_type: String,
    pub definition: IrMathExpression,
    pub parameters: Option<(String, String)>,
    pub domain: IrMathDomain,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrFieldNode {
    pub id: String,
    pub field_kind: String,
    pub dimension: usize,
    pub components: Vec<IrMathExpression>,
    pub domain: IrMathDomain,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrOdeNode {
    pub id: String,
    pub ode_type: String,
    pub order: usize,
    pub equation: IrMathExpression,
    pub initial_conditions: Vec<IrMathExpression>,
    pub boundary_conditions: Vec<IrMathExpression>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTransformationNode {
    pub id: String,
    pub transform_type: String,
    pub expression: IrMathExpression,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMatrixNode {
    pub id: String,
    pub rows: usize,
    pub cols: usize,
    pub elements: Vec<Vec<IrMathExpression>>,
}

/// Lowers a validated AST to IR
pub struct IrLowering;

impl IrLowering {
    pub fn lower(ast: AstFile) -> DslResult<IrScene> {
        let metadata = Self::lower_metadata(&ast.scene, &ast.library_imports);
        let entities = Self::lower_entities(ast.entities)?;
        let constraints = Self::lower_constraints(ast.constraints)?;
        let motions = Self::lower_motions(ast.motions)?;
        let math_entities = Self::lower_math_objects(ast.math_objects);
        let compound_motions = Self::lower_compound_motions(ast.compound_motions)?;
        let timelines = Self::lower_timelines(ast.timelines)?;
        let annotations = Self::lower_annotations(ast.annotations);
        let highlight_schedule = Self::lower_highlight_schedule(ast.highlight_schedule);
        let concept_ref = Self::lower_concept_ref(ast.concept_ref);

        Ok(IrScene {
            metadata,
            entities,
            constraints,
            motions,
            math_entities,
            compound_motions,
            timelines,
            annotations,
            highlight_schedule,
            concept_ref,
        })
    }

    fn lower_metadata(scene: &AstScene, imports: &AstLibraryImports) -> IrMetadata {
        IrMetadata {
            name: scene.name.clone(),
            version: scene.version,
            ir_version: scene.ir_version.clone(),
            unit_system: scene.unit_system.clone(),
            libraries: imports
                .imports
                .iter()
                .map(|i| i.library_name.clone())
                .collect(),
        }
    }

    fn lower_entities(entities: Vec<AstEntity>) -> DslResult<Vec<IrEntity>> {
        entities
            .into_iter()
            .map(|entity| {
                let mut components_map = HashMap::new();

                for component in entity.components {
                    let mut properties = HashMap::new();

                    for field in component.fields {
                        let value = Self::lower_value(field.value)?;
                        properties.insert(field.name, value);
                    }

                    let ir_component = IrComponent {
                        component_type: component.name.clone(),
                        properties,
                    };

                    components_map.insert(component.name, ir_component);
                }

                Ok(IrEntity {
                    id: entity.name,
                    kind: entity.kind,
                    components: components_map,
                })
            })
            .collect()
    }

    fn lower_constraints(constraints: Vec<AstConstraint>) -> DslResult<Vec<IrConstraint>> {
        constraints
            .into_iter()
            .map(|constraint| {
                let mut parameters = HashMap::new();
                let mut constraint_type = String::new();

                for field in constraint.fields {
                    if field.name == "type" {
                        if let AstValue::Identifier(type_name, _) = field.value {
                            constraint_type = type_name;
                        }
                    } else {
                        let value = Self::lower_value(field.value)?;
                        parameters.insert(field.name, value);
                    }
                }

                Ok(IrConstraint {
                    id: constraint.name,
                    constraint_type,
                    parameters,
                })
            })
            .collect()
    }

    fn lower_motions(motions: Vec<AstMotion>) -> DslResult<Vec<IrMotion>> {
        motions
            .into_iter()
            .map(|motion| {
                let mut parameters = HashMap::new();
                let mut motion_type = String::new();
                let mut target_entity = String::new();

                for field in motion.fields {
                    match field.name.as_str() {
                        "type" => {
                            if let AstValue::Identifier(type_name, _) = field.value {
                                motion_type = type_name;
                            }
                        }
                        "target" => {
                            if let AstValue::Identifier(target, _) = field.value {
                                target_entity = target;
                            }
                        }
                        _ => {
                            let value = Self::lower_value(field.value)?;
                            parameters.insert(field.name, value);
                        }
                    }
                }

                Ok(IrMotion {
                    id: motion.name,
                    motion_type,
                    target_entity,
                    parameters,
                })
            })
            .collect()
    }

    fn lower_compound_motions(
        compound_motions: Vec<AstCompoundMotion>,
    ) -> DslResult<Vec<IrCompoundMotion>> {
        compound_motions
            .into_iter()
            .map(|compound| {
                let mut parameters = HashMap::new();
                let mut compound_type = String::new();
                let mut motions: Vec<String> = Vec::new();

                for field in compound.fields {
                    match field.name.as_str() {
                        "type" => {
                            if let AstValue::Identifier(type_name, _) = field.value {
                                compound_type = type_name;
                            }
                        }
                        "motions" => match field.value {
                            AstValue::String(list, _) => {
                                motions = list
                                    .split(',')
                                    .map(|m| m.trim().to_string())
                                    .filter(|m| !m.is_empty())
                                    .collect();
                            }
                            AstValue::List(values, _) => {
                                motions = values
                                    .into_iter()
                                    .filter_map(|value| match value {
                                        AstValue::Identifier(id, _) => Some(id),
                                        AstValue::String(s, _) => Some(s),
                                        _ => None,
                                    })
                                    .collect();
                            }
                            _ => {}
                        },
                        _ => {
                            let value = Self::lower_value(field.value)?;
                            parameters.insert(field.name, value);
                        }
                    }
                }

                Ok(IrCompoundMotion {
                    id: compound.name,
                    compound_type,
                    motions,
                    parameters,
                })
            })
            .collect()
    }

    fn lower_timelines(timelines: Vec<AstTimeline>) -> DslResult<Vec<IrTimeline>> {
        timelines
            .into_iter()
            .map(|timeline| {
                let events = timeline
                    .events
                    .into_iter()
                    .filter_map(|event| {
                        let motion_id = event.motion()?.to_string();
                        let start_time = event.start()?;
                        let duration = event.duration()?;

                        Some(IrEvent {
                            motion_id,
                            start_time,
                            duration,
                        })
                    })
                    .collect();

                Ok(IrTimeline {
                    id: timeline.name,
                    events,
                })
            })
            .collect()
    }

    fn lower_annotations(annotations: Vec<AnnotationNode>) -> Vec<IrAnnotation> {
        annotations
            .into_iter()
            .map(|annotation| IrAnnotation {
                label_text: annotation.label_text,
                anchor_entity_id: annotation.anchor_entity_id,
                position_offset: annotation.position_offset,
                equation_node_id: annotation.equation_node_id,
                highlight_token: annotation.highlight_token,
            })
            .collect()
    }

    fn lower_highlight_schedule(
        schedule: Vec<HighlightScheduleEntry>,
    ) -> Vec<IrHighlightEntry> {
        schedule
            .into_iter()
            .map(|entry| IrHighlightEntry {
                at_time: entry.at_time,
                highlight_token: entry.highlight_token,
                entity_id: entry.entity_id,
                color_index: entry.color_index,
            })
            .collect()
    }

    fn lower_concept_ref(concept_ref: Option<ConceptAnnotation>) -> Option<IrConceptRef> {
        concept_ref.map(|concept| IrConceptRef {
            concept_id: concept.concept_id,
            section_id: concept.section_id,
            step_index: concept.step_index,
        })
    }

    fn lower_math_objects(math_objects: Vec<MathObjectNode>) -> Vec<IrMathEntity> {
        math_objects
            .into_iter()
            .map(|node| match node {
                MathObjectNode::Function(function) => IrMathEntity::Function(IrFunctionNode {
                    id: function.name,
                    parameters: function.parameters,
                    body: Self::lower_math_expression(&function.body),
                    domain: Self::lower_math_domain(function.domain),
                    range_constraints: function
                        .range
                        .map(|range| {
                            range
                                .constraints
                                .into_iter()
                                .map(|constraint| {
                                    Self::lower_math_expression(&constraint.expression)
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }),
                MathObjectNode::Curve(curve) => IrMathEntity::Curve(IrCurveNode {
                    id: curve.name,
                    curve_type: Self::curve_type_name(curve.curve_type).to_string(),
                    definition: Self::lower_math_expression(&curve.definition),
                    parameter: curve.parameter,
                    domain: Self::lower_math_domain(curve.domain),
                }),
                MathObjectNode::Surface(surface) => IrMathEntity::Surface(IrSurfaceNode {
                    id: surface.name,
                    surface_type: Self::surface_type_name(surface.surface_type).to_string(),
                    definition: Self::lower_math_expression(&surface.definition),
                    parameters: surface.parameters,
                    domain: Self::lower_math_domain(surface.domain),
                }),
                MathObjectNode::VectorField(field) => IrMathEntity::Field(IrFieldNode {
                    id: field.name,
                    field_kind: "vector".to_string(),
                    dimension: field.dimension,
                    components: field
                        .components
                        .iter()
                        .map(Self::lower_math_expression)
                        .collect(),
                    domain: Self::lower_math_domain(field.domain),
                }),
                MathObjectNode::ScalarField(field) => IrMathEntity::Field(IrFieldNode {
                    id: field.name,
                    field_kind: "scalar".to_string(),
                    dimension: 1,
                    components: vec![Self::lower_math_expression(&field.expression)],
                    domain: Self::lower_math_domain(field.domain),
                }),
                MathObjectNode::Transformation(transform) => {
                    IrMathEntity::Transformation(IrTransformationNode {
                        id: transform.name,
                        transform_type: Self::transformation_type_name(transform.transform_type)
                            .to_string(),
                        expression: Self::lower_math_expression(&transform.expression),
                    })
                }
                MathObjectNode::DifferentialEquation(ode) => IrMathEntity::Ode(IrOdeNode {
                    id: ode.name,
                    ode_type: Self::differential_equation_type_name(ode.equation_type).to_string(),
                    order: ode.order,
                    equation: Self::lower_math_expression(&ode.equation),
                    initial_conditions: ode
                        .initial_conditions
                        .iter()
                        .map(|condition| Self::lower_math_expression(&condition.expression))
                        .collect(),
                    boundary_conditions: ode
                        .boundary_conditions
                        .iter()
                        .map(|condition| Self::lower_math_expression(&condition.expression))
                        .collect(),
                }),
                MathObjectNode::MatrixDefinition(matrix) => IrMathEntity::Matrix(IrMatrixNode {
                    id: matrix.name,
                    rows: matrix.rows,
                    cols: matrix.cols,
                    elements: matrix
                        .elements
                        .iter()
                        .map(|row| row.iter().map(Self::lower_math_expression).collect())
                        .collect(),
                }),
            })
            .collect()
    }

    fn lower_math_domain(domain: DomainConstraint) -> IrMathDomain {
        IrMathDomain {
            space: Self::math_space_name(domain.space).to_string(),
            variables: domain.variables,
            constraints: domain
                .constraints
                .iter()
                .map(|constraint| Self::lower_math_expression(&constraint.expression))
                .collect(),
        }
    }

    fn curve_type_name(curve_type: CurveType) -> &'static str {
        match curve_type {
            CurveType::Explicit => "explicit",
            CurveType::Implicit => "implicit",
            CurveType::Parametric => "parametric",
            CurveType::Polar => "polar",
        }
    }

    fn surface_type_name(surface_type: SurfaceType) -> &'static str {
        match surface_type {
            SurfaceType::Explicit => "explicit",
            SurfaceType::Implicit => "implicit",
            SurfaceType::Parametric => "parametric",
        }
    }

    fn transformation_type_name(transform_type: TransformationType) -> &'static str {
        match transform_type {
            TransformationType::Linear => "linear",
            TransformationType::Affine => "affine",
            TransformationType::NonLinear => "non_linear",
        }
    }

    fn differential_equation_type_name(equation_type: DifferentialEquationType) -> &'static str {
        match equation_type {
            DifferentialEquationType::Ode => "ode",
            DifferentialEquationType::Pde => "pde",
        }
    }

    fn math_space_name(space: MathSpace) -> &'static str {
        match space {
            MathSpace::Real => "real",
            MathSpace::Real2 => "real2",
            MathSpace::Real3 => "real3",
            MathSpace::Complex => "complex",
        }
    }

    fn lower_value(value: AstValue) -> DslResult<IrValue> {
        match value {
            AstValue::Number(n, _) => Ok(IrValue::Number(n)),
            AstValue::String(s, _) => Ok(IrValue::String(s)),
            AstValue::Boolean(b, _) => Ok(IrValue::Boolean(b)),
            AstValue::Identifier(id, _) => {
                // Handle boolean identifiers
                match id.as_str() {
                    "true" => Ok(IrValue::Boolean(true)),
                    "false" => Ok(IrValue::Boolean(false)),
                    _ => Ok(IrValue::Identifier(id)),
                }
            }
            AstValue::Vector(vec, span) => {
                if vec.len() != 3 {
                    return Err(DslError::new(
                        ErrorCode::InvalidVectorLength,
                        format!("Expected 3D vector, found {} components", vec.len()),
                        span,
                        std::path::PathBuf::from("lowering"),
                    ));
                }
                Ok(IrValue::Vector3([vec[0], vec[1], vec[2]]))
            }
            AstValue::Matrix(matrix, span) => {
                if matrix.len() != 3 || matrix.iter().any(|row| row.len() != 3) {
                    return Err(DslError::new(
                        ErrorCode::InvalidFieldType,
                        format!(
                            "Expected 3x3 matrix, found {}x{}",
                            matrix.len(),
                            matrix.get(0).map(|row| row.len()).unwrap_or(0)
                        ),
                        span,
                        std::path::PathBuf::from("lowering"),
                    ));
                }
                Ok(IrValue::Matrix3([
                    [matrix[0][0], matrix[0][1], matrix[0][2]],
                    [matrix[1][0], matrix[1][1], matrix[1][2]],
                    [matrix[2][0], matrix[2][1], matrix[2][2]],
                ]))
            }
            AstValue::List(values, _) => {
                let mut lowered = Vec::with_capacity(values.len());
                for value in values {
                    lowered.push(Self::lower_value(value)?);
                }
                Ok(IrValue::List(lowered))
            }
            AstValue::MathExpression(expr, _) => {
                Ok(IrValue::MathExpression(Self::lower_math_expression(&expr)))
            }
        }
    }

    fn lower_math_expression(expr: &AnnotatedExpr) -> IrMathExpression {
        IrMathExpression {
            expression_type: Self::math_expr_kind(&expr.expr).to_string(),
            source: Self::annotated_expr_to_string(expr),
            complexity: Self::annotated_expr_complexity(expr),
            node_id: expr.node_id.clone(),
            highlight_token: expr.highlight_token.clone(),
            children: Self::lower_math_children(&expr.expr),
        }
    }

    fn lower_math_children(expr: &MathExpression) -> Vec<IrMathExpression> {
        match expr {
            MathExpression::BinaryOp(lhs, _, rhs) => vec![
                Self::lower_math_expression(lhs),
                Self::lower_math_expression(rhs),
            ],
            MathExpression::UnaryOp(_, value)
            | MathExpression::Derivative {
                expression: value, ..
            }
            | MathExpression::Limit {
                expression: value, ..
            }
            | MathExpression::Summation {
                expression: value, ..
            }
            | MathExpression::Product {
                expression: value, ..
            } => vec![Self::lower_math_expression(value)],
            MathExpression::Integral {
                expression,
                bounds,
                ..
            } => {
                let mut children = vec![Self::lower_math_expression(expression)];
                if let Some(interval) = bounds {
                    children.push(Self::lower_math_expression(&interval.lower));
                    children.push(Self::lower_math_expression(&interval.upper));
                }
                children
            }
            MathExpression::FunctionCall(_, args) => {
                args.iter().map(Self::lower_math_expression).collect()
            }
            MathExpression::Piecewise(cases) => cases
                .iter()
                .map(|(_, expr)| Self::lower_math_expression(expr))
                .collect(),
            MathExpression::MatrixExpr(rows) => rows
                .iter()
                .flat_map(|row| row.iter())
                .map(Self::lower_math_expression)
                .collect(),
            MathExpression::VectorExpr(values) => {
                values.iter().map(Self::lower_math_expression).collect()
            }
            MathExpression::Variable(_)
            | MathExpression::Constant(_)
            | MathExpression::Number(_)
            | MathExpression::ComplexNumber { .. } => Vec::new(),
        }
    }

    fn math_expr_kind(expr: &MathExpression) -> &'static str {
        match expr {
            MathExpression::Variable(_) => "variable",
            MathExpression::Constant(_) => "constant",
            MathExpression::Number(_) => "number",
            MathExpression::ComplexNumber { .. } => "complex_number",
            MathExpression::BinaryOp(..) => "binary",
            MathExpression::UnaryOp(..) => "unary",
            MathExpression::FunctionCall(..) => "function_call",
            MathExpression::Derivative { .. } => "derivative",
            MathExpression::Integral { .. } => "integral",
            MathExpression::Limit { .. } => "limit",
            MathExpression::Summation { .. } => "summation",
            MathExpression::Product { .. } => "product",
            MathExpression::Piecewise(_) => "piecewise",
            MathExpression::MatrixExpr(_) => "matrix",
            MathExpression::VectorExpr(_) => "vector",
        }
    }

    fn annotated_expr_to_string(expr: &AnnotatedExpr) -> String {
        Self::math_expr_to_string(&expr.expr)
    }

    fn math_expr_to_string(expr: &MathExpression) -> String {
        match expr {
            MathExpression::Variable(v) => v.clone(),
            MathExpression::Constant(c) => match c {
                crate::ast::MathConstant::Pi => "pi".to_string(),
                crate::ast::MathConstant::Euler => "e".to_string(),
                crate::ast::MathConstant::ImaginaryUnit => "i".to_string(),
                crate::ast::MathConstant::Infinity => "inf".to_string(),
            },
            MathExpression::Number(n) => n.to_string(),
            MathExpression::ComplexNumber { real, imag } => format!("{}+{}i", real, imag),
            MathExpression::BinaryOp(lhs, op, rhs) => format!(
                "({} {:?} {})",
                Self::annotated_expr_to_string(lhs),
                op,
                Self::annotated_expr_to_string(rhs)
            ),
            MathExpression::UnaryOp(op, value) => {
                format!("({:?} {})", op, Self::annotated_expr_to_string(value))
            }
            MathExpression::FunctionCall(name, args) => format!(
                "{}({})",
                name,
                args.iter()
                    .map(Self::annotated_expr_to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            MathExpression::Derivative {
                expression,
                variable,
                order,
            } => format!(
                "derivative({}, {}, {})",
                Self::annotated_expr_to_string(expression),
                variable,
                order
            ),
            MathExpression::Integral {
                expression,
                variable,
                bounds,
            } => {
                if let Some(b) = bounds {
                    format!(
                        "integral({}, {}, {}, {})",
                        Self::annotated_expr_to_string(expression),
                        variable,
                        Self::annotated_expr_to_string(&b.lower),
                        Self::annotated_expr_to_string(&b.upper)
                    )
                } else {
                    format!(
                        "integral({}, {})",
                        Self::annotated_expr_to_string(expression),
                        variable
                    )
                }
            }
            MathExpression::Limit {
                expression,
                variable,
                approach,
            } => format!(
                "limit({}, {}, {})",
                Self::annotated_expr_to_string(expression),
                variable,
                approach
            ),
            MathExpression::Summation {
                expression,
                variable,
                bounds,
            } => format!(
                "sum({}, {}, {}, {})",
                Self::annotated_expr_to_string(expression),
                variable,
                Self::annotated_expr_to_string(&bounds.lower),
                Self::annotated_expr_to_string(&bounds.upper)
            ),
            MathExpression::Product {
                expression,
                variable,
                bounds,
            } => format!(
                "product({}, {}, {}, {})",
                Self::annotated_expr_to_string(expression),
                variable,
                Self::annotated_expr_to_string(&bounds.lower),
                Self::annotated_expr_to_string(&bounds.upper)
            ),
            MathExpression::Piecewise(cases) => format!("piecewise[{} cases]", cases.len()),
            MathExpression::MatrixExpr(rows) => format!(
                "matrix[{}x{}]",
                rows.len(),
                rows.first().map_or(0, |r| r.len())
            ),
            MathExpression::VectorExpr(values) => format!("vector[{}]", values.len()),
        }
    }

    fn annotated_expr_complexity(expr: &AnnotatedExpr) -> usize {
        Self::math_expr_complexity(&expr.expr)
    }

    fn math_expr_complexity(expr: &MathExpression) -> usize {
        match expr {
            MathExpression::Variable(_)
            | MathExpression::Constant(_)
            | MathExpression::Number(_)
            | MathExpression::ComplexNumber { .. } => 1,
            MathExpression::UnaryOp(_, v) => 1 + Self::annotated_expr_complexity(v),
            MathExpression::BinaryOp(l, _, r) => {
                1 + Self::annotated_expr_complexity(l) + Self::annotated_expr_complexity(r)
            }
            MathExpression::FunctionCall(_, args) => {
                1 + args.iter().map(Self::annotated_expr_complexity).sum::<usize>()
            }
            MathExpression::Derivative { expression, .. }
            | MathExpression::Integral { expression, .. }
            | MathExpression::Limit { expression, .. }
            | MathExpression::Summation { expression, .. }
            | MathExpression::Product { expression, .. } => {
                1 + Self::annotated_expr_complexity(expression)
            }
            MathExpression::Piecewise(cases) => {
                1 + cases
                    .iter()
                    .map(|(_, expr)| Self::annotated_expr_complexity(expr))
                    .sum::<usize>()
            }
            MathExpression::MatrixExpr(rows) => {
                1 + rows
                    .iter()
                    .flat_map(|row| row.iter())
                    .map(Self::annotated_expr_complexity)
                    .sum::<usize>()
            }
            MathExpression::VectorExpr(values) => {
                1 + values.iter().map(Self::annotated_expr_complexity).sum::<usize>()
            }
        }
    }
}

/// IR serialization helper (for debugging/output)
impl IrScene {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "metadata": {
                "name": self.metadata.name,
                "version": self.metadata.version,
                "ir_version": self.metadata.ir_version,
                "unit_system": self.metadata.unit_system,
                "libraries": self.metadata.libraries,
            },
            "entities": self.entities.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "kind": e.kind,
                    "components": e.components.iter().map(|(name, comp)| {
                        (name.clone(), serde_json::json!({
                            "type": comp.component_type,
                            "properties": comp.properties.iter().map(|(k, v)| {
                                (k.clone(), Self::value_to_json(v))
                            }).collect::<serde_json::Map<_, _>>(),
                        }))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "constraints": self.constraints.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "type": c.constraint_type,
                    "parameters": c.parameters.iter().map(|(k, v)| {
                        (k.clone(), Self::value_to_json(v))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "motions": self.motions.iter().map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "type": m.motion_type,
                    "target": m.target_entity,
                    "parameters": m.parameters.iter().map(|(k, v)| {
                        (k.clone(), Self::value_to_json(v))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "math_entities": self.math_entities.iter().map(Self::math_entity_to_json).collect::<Vec<_>>(),
            "compound_motions": self.compound_motions.iter().map(|cm| {
                serde_json::json!({
                    "id": cm.id,
                    "type": cm.compound_type,
                    "motions": cm.motions,
                    "parameters": cm.parameters.iter().map(|(k, v)| {
                        (k.clone(), Self::value_to_json(v))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "timelines": self.timelines.iter().map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "events": t.events.iter().map(|e| {
                        serde_json::json!({
                            "motion": e.motion_id,
                            "start": e.start_time,
                            "duration": e.duration,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "annotations": self.annotations.iter().map(|a| {
                serde_json::json!({
                    "label_text": a.label_text,
                    "anchor_entity_id": a.anchor_entity_id,
                    "position_offset": a.position_offset,
                    "equation_node_id": a.equation_node_id,
                    "highlight_token": a.highlight_token,
                })
            }).collect::<Vec<_>>(),
            "highlight_schedule": self.highlight_schedule.iter().map(|e| {
                serde_json::json!({
                    "at_time": e.at_time,
                    "highlight_token": e.highlight_token,
                    "entity_id": e.entity_id,
                    "color_index": e.color_index,
                })
            }).collect::<Vec<_>>(),
            "concept_ref": self.concept_ref.as_ref().map(|c| serde_json::json!({
                "concept_id": c.concept_id,
                "section_id": c.section_id,
                "step_index": c.step_index,
            })),
        })
    }

    fn value_to_json(value: &IrValue) -> serde_json::Value {
        match value {
            IrValue::Number(n) => serde_json::json!(n),
            IrValue::String(s) => serde_json::json!(s),
            IrValue::Identifier(id) => serde_json::json!(id),
            IrValue::Vector3(v) => serde_json::json!(v),
            IrValue::Boolean(b) => serde_json::json!(b),
            IrValue::Matrix3(m) => serde_json::json!(m),
            IrValue::List(values) => {
                serde_json::json!(values.iter().map(Self::value_to_json).collect::<Vec<_>>())
            }
            IrValue::MathExpression(expr) => serde_json::json!({
                "type": expr.expression_type,
                "source": expr.source,
                "complexity": expr.complexity,
                "node_id": expr.node_id,
                "highlight_token": expr.highlight_token,
                "children": expr.children.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
            }),
        }
    }

    fn math_entity_to_json(entity: &IrMathEntity) -> serde_json::Value {
        match entity {
            IrMathEntity::Function(node) => serde_json::json!({
                "kind": "function",
                "id": node.id,
                "parameters": node.parameters,
                "body": {
                    "type": node.body.expression_type,
                    "source": node.body.source,
                    "complexity": node.body.complexity
                },
                "domain": Self::math_domain_to_json(&node.domain),
                "range_constraints": node.range_constraints.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
            }),
            IrMathEntity::Curve(node) => serde_json::json!({
                "kind": "curve",
                "id": node.id,
                "curve_type": node.curve_type,
                "definition": Self::math_expression_to_json(&node.definition),
                "parameter": node.parameter,
                "domain": Self::math_domain_to_json(&node.domain),
            }),
            IrMathEntity::Surface(node) => serde_json::json!({
                "kind": "surface",
                "id": node.id,
                "surface_type": node.surface_type,
                "definition": Self::math_expression_to_json(&node.definition),
                "parameters": node.parameters,
                "domain": Self::math_domain_to_json(&node.domain),
            }),
            IrMathEntity::Field(node) => serde_json::json!({
                "kind": "field",
                "id": node.id,
                "field_kind": node.field_kind,
                "dimension": node.dimension,
                "components": node.components.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
                "domain": Self::math_domain_to_json(&node.domain),
            }),
            IrMathEntity::Ode(node) => serde_json::json!({
                "kind": "ode",
                "id": node.id,
                "ode_type": node.ode_type,
                "order": node.order,
                "equation": Self::math_expression_to_json(&node.equation),
                "initial_conditions": node.initial_conditions.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
                "boundary_conditions": node.boundary_conditions.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
            }),
            IrMathEntity::Transformation(node) => serde_json::json!({
                "kind": "transformation",
                "id": node.id,
                "transform_type": node.transform_type,
                "expression": Self::math_expression_to_json(&node.expression),
            }),
            IrMathEntity::Matrix(node) => serde_json::json!({
                "kind": "matrix",
                "id": node.id,
                "rows": node.rows,
                "cols": node.cols,
                "elements": node.elements.iter().map(|row| row.iter().map(Self::math_expression_to_json).collect::<Vec<_>>()).collect::<Vec<_>>(),
            }),
        }
    }

    fn math_domain_to_json(domain: &IrMathDomain) -> serde_json::Value {
        serde_json::json!({
            "space": domain.space,
            "variables": domain.variables,
            "constraints": domain.constraints.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
        })
    }

    fn math_expression_to_json(expr: &IrMathExpression) -> serde_json::Value {
        serde_json::json!({
            "type": expr.expression_type,
            "source": expr.source,
            "complexity": expr.complexity,
            "node_id": expr.node_id,
            "highlight_token": expr.highlight_token,
            "children": expr.children.iter().map(Self::math_expression_to_json).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::SourceSpan;

    fn mk_expr(expr: MathExpression) -> AnnotatedExpr {
        AnnotatedExpr {
            node_id: "test_node".to_string(),
            highlight_token: None,
            expr,
        }
    }

    #[test]
    fn test_value_lowering() {
        let span = SourceSpan::single_point(1, 1, 0);

        let num = AstValue::Number(42.0, span);
        let ir_num = IrLowering::lower_value(num).unwrap();
        assert!(matches!(ir_num, IrValue::Number(42.0)));

        let vec = AstValue::Vector(vec![1.0, 2.0, 3.0], span);
        let ir_vec = IrLowering::lower_value(vec).unwrap();
        assert!(matches!(ir_vec, IrValue::Vector3([1.0, 2.0, 3.0])));

        let bool_true = AstValue::Boolean(true, span);
        let ir_bool = IrLowering::lower_value(bool_true).unwrap();
        assert!(matches!(ir_bool, IrValue::Boolean(true)));
    }

    #[test]
    fn test_invalid_vector_length() {
        let span = SourceSpan::single_point(1, 1, 0);
        let vec = AstValue::Vector(vec![1.0, 2.0], span);
        let result = IrLowering::lower_value(vec);
        assert!(result.is_err());
    }

    #[test]
    fn test_lower_math_objects_to_explicit_ir_entities() {
        let s = SourceSpan::single_point(1, 1, 0);
        let domain = DomainConstraint {
            variables: vec!["x".to_string()],
            constraints: vec![],
            space: MathSpace::Real,
        };

        let ast = AstFile {
            scene: AstScene {
                name: "math_scene".to_string(),
                version: 1,
                ir_version: "0.1.0".to_string(),
                unit_system: "SI".to_string(),
                domain: Some("math".to_string()),
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
            motions: vec![],
            math_objects: vec![
                MathObjectNode::Function(FunctionNode {
                    name: "f".to_string(),
                    parameters: vec!["x".to_string()],
                    body: mk_expr(MathExpression::Variable("x".to_string())),
                    domain: domain.clone(),
                    range: None,
                    properties: FunctionProperties {
                        continuous: true,
                        differentiable_order: None,
                        periodic: None,
                        symmetric: None,
                        monotonic: None,
                    },
                    span: s,
                }),
                MathObjectNode::ScalarField(ScalarFieldNode {
                    name: "phi".to_string(),
                    expression: mk_expr(MathExpression::Number(3.0)),
                    domain: domain.clone(),
                    span: s,
                }),
                MathObjectNode::DifferentialEquation(DifferentialEquationNode {
                    name: "ode1".to_string(),
                    equation_type: DifferentialEquationType::Ode,
                    order: 1,
                    equation: mk_expr(MathExpression::Variable("x".to_string())),
                    initial_conditions: vec![],
                    boundary_conditions: vec![],
                    span: s,
                }),
            ],
            compound_motions: vec![],
            trajectories: vec![],
            timelines: vec![],
            concept_ref: None,
            annotations: vec![],
            highlight_schedule: vec![],
            span: s,
        };

        let ir = IrLowering::lower(ast).expect("lowering should succeed");
        assert_eq!(ir.math_entities.len(), 3);
        assert!(matches!(ir.math_entities[0], IrMathEntity::Function(_)));
        assert!(matches!(ir.math_entities[1], IrMathEntity::Field(_)));
        assert!(matches!(ir.math_entities[2], IrMathEntity::Ode(_)));
    }
}
