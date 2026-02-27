/// Schema validation pass.
/// Validates entity components, constraint types, and field schemas.
/// Ensures conformance to known component and constraint schemas.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector, SourceSpan};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct SchemaValidator {
    file: PathBuf,
    errors: ErrorCollector,
    component_schemas: HashMap<String, ComponentSchema>,
    constraint_schemas: HashMap<String, ConstraintSchema>,
    motion_schemas: HashMap<String, MotionSchema>,
}

/// Schema definition for a component
#[derive(Debug, Clone)]
pub struct ComponentSchema {
    pub name: String,
    pub required_fields: Vec<FieldSchema>,
    pub optional_fields: Vec<FieldSchema>,
}

/// Schema definition for a field
#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: FieldType,
}

/// Field type definitions
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Number,
    String,
    Identifier,
    Vector,
    Vector3,
    List,
    Boolean,
    MathExpression,
}

/// Schema definition for a constraint
#[derive(Debug, Clone)]
pub struct ConstraintSchema {
    pub name: String,
    pub required_fields: Vec<FieldSchema>,
    pub optional_fields: Vec<FieldSchema>,
}

/// Schema definition for a motion
#[derive(Debug, Clone)]
pub struct MotionSchema {
    pub name: String,
    pub required_fields: Vec<FieldSchema>,
    pub optional_fields: Vec<FieldSchema>,
}

impl SchemaValidator {
    pub fn new(file: PathBuf) -> Self {
        let mut validator = Self {
            file,
            errors: ErrorCollector::new(),
            component_schemas: HashMap::new(),
            constraint_schemas: HashMap::new(),
            motion_schemas: HashMap::new(),
        };

        validator.init_default_schemas();
        validator
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        self.validate_entities(&ast.entities);
        self.validate_constraints(&ast.constraints);
        self.validate_motions(&ast.motions);
        self.validate_math_objects(&ast.math_objects);
        self.validate_concept_ref(ast.concept_ref.as_ref());
        self.validate_annotations(&ast.annotations);
        self.validate_highlight_schedule(&ast.highlight_schedule);

        self.errors.into_result(())
    }

    fn init_default_schemas(&mut self) {
        // Register default component schemas
        self.register_component_schema(ComponentSchema {
            name: "transform".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "position".to_string(),
                    field_type: FieldType::Vector3,
                },
                FieldSchema {
                    name: "rotation".to_string(),
                    field_type: FieldType::Vector3,
                },
                FieldSchema {
                    name: "scale".to_string(),
                    field_type: FieldType::Vector3,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "geometry".to_string(),
            required_fields: vec![FieldSchema {
                name: "primitive".to_string(),
                field_type: FieldType::Identifier,
            }],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "physical".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "mass".to_string(),
                    field_type: FieldType::Number,
                },
                FieldSchema {
                    name: "rigid".to_string(),
                    field_type: FieldType::Boolean,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "solid".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "primitive".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "dimensions".to_string(),
                    field_type: FieldType::Vector,
                },
            ],
            optional_fields: vec![FieldSchema {
                name: "fillet_radius".to_string(),
                field_type: FieldType::Number,
            }],
        });

        self.register_component_schema(ComponentSchema {
            name: "boolean_op".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "operation".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "target".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "tool".to_string(),
                    field_type: FieldType::Identifier,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "sketch".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "plane".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "closed".to_string(),
                    field_type: FieldType::Boolean,
                },
                FieldSchema {
                    name: "points".to_string(),
                    field_type: FieldType::List,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "extrude".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "sketch_ref".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "depth".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![FieldSchema {
                name: "taper_angle".to_string(),
                field_type: FieldType::Number,
            }],
        });

        self.register_component_schema(ComponentSchema {
            name: "revolve".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "sketch_ref".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "axis".to_string(),
                    field_type: FieldType::Vector3,
                },
                FieldSchema {
                    name: "angle".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "loft".to_string(),
            required_fields: vec![FieldSchema {
                name: "profiles".to_string(),
                field_type: FieldType::List,
            }],
            optional_fields: vec![FieldSchema {
                name: "guide_curves".to_string(),
                field_type: FieldType::List,
            }],
        });

        self.register_component_schema(ComponentSchema {
            name: "sweep".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "profile".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "path".to_string(),
                    field_type: FieldType::Identifier,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "shell".to_string(),
            required_fields: vec![FieldSchema {
                name: "thickness".to_string(),
                field_type: FieldType::Number,
            }],
            optional_fields: vec![FieldSchema {
                name: "open_faces".to_string(),
                field_type: FieldType::List,
            }],
        });

        self.register_component_schema(ComponentSchema {
            name: "chamfer".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "edges".to_string(),
                    field_type: FieldType::List,
                },
                FieldSchema {
                    name: "distance".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "fillet".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "edges".to_string(),
                    field_type: FieldType::List,
                },
                FieldSchema {
                    name: "radius".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "thread".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "face".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "pitch".to_string(),
                    field_type: FieldType::Number,
                },
                FieldSchema {
                    name: "handedness".to_string(),
                    field_type: FieldType::Identifier,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "annotation".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "target".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "value".to_string(),
                    field_type: FieldType::String,
                },
            ],
            optional_fields: vec![],
        });

        self.register_component_schema(ComponentSchema {
            name: "material_ref".to_string(),
            required_fields: vec![FieldSchema {
                name: "name".to_string(),
                field_type: FieldType::Identifier,
            }],
            optional_fields: vec![],
        });

        // Register default constraint schemas
        self.register_constraint_schema(ConstraintSchema {
            name: "gear_relation".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "driver".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "driven".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "ratio".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_constraint_schema(ConstraintSchema {
            name: "fixed_joint".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "parent".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "child".to_string(),
                    field_type: FieldType::Identifier,
                },
            ],
            optional_fields: vec![],
        });

        for constraint_name in ["coincident", "parallel", "perpendicular", "tangent"] {
            self.register_constraint_schema(ConstraintSchema {
                name: constraint_name.to_string(),
                required_fields: vec![
                    FieldSchema {
                        name: "type".to_string(),
                        field_type: FieldType::Identifier,
                    },
                    FieldSchema {
                        name: "entity_a".to_string(),
                        field_type: FieldType::Identifier,
                    },
                    FieldSchema {
                        name: "entity_b".to_string(),
                        field_type: FieldType::Identifier,
                    },
                ],
                optional_fields: vec![],
            });
        }

        self.register_constraint_schema(ConstraintSchema {
            name: "distance".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_a".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_b".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "value".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![FieldSchema {
                name: "unit".to_string(),
                field_type: FieldType::Identifier,
            }],
        });

        self.register_constraint_schema(ConstraintSchema {
            name: "angle".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_a".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_b".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "value".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_constraint_schema(ConstraintSchema {
            name: "symmetric".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_a".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "entity_b".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "axis".to_string(),
                    field_type: FieldType::Vector3,
                },
            ],
            optional_fields: vec![],
        });

        for constraint_name in ["boolean_subtract", "boolean_union", "boolean_intersect"] {
            self.register_constraint_schema(ConstraintSchema {
                name: constraint_name.to_string(),
                required_fields: vec![
                    FieldSchema {
                        name: "type".to_string(),
                        field_type: FieldType::Identifier,
                    },
                    FieldSchema {
                        name: "target".to_string(),
                        field_type: FieldType::Identifier,
                    },
                    FieldSchema {
                        name: "tool".to_string(),
                        field_type: FieldType::Identifier,
                    },
                ],
                optional_fields: vec![],
            });
        }

        // Register default motion schemas
        self.register_motion_schema(MotionSchema {
            name: "rotation".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "target".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "axis".to_string(),
                    field_type: FieldType::Vector3,
                },
                FieldSchema {
                    name: "speed".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_motion_schema(MotionSchema {
            name: "translation".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "target".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "direction".to_string(),
                    field_type: FieldType::Vector3,
                },
                FieldSchema {
                    name: "speed".to_string(),
                    field_type: FieldType::Number,
                },
            ],
            optional_fields: vec![],
        });

        self.register_motion_schema(MotionSchema {
            name: "math".to_string(),
            required_fields: vec![
                FieldSchema {
                    name: "target".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "type".to_string(),
                    field_type: FieldType::Identifier,
                },
                FieldSchema {
                    name: "expr".to_string(),
                    field_type: FieldType::MathExpression,
                },
            ],
            optional_fields: vec![],
        });
    }

    fn register_component_schema(&mut self, schema: ComponentSchema) {
        self.component_schemas.insert(schema.name.clone(), schema);
    }

    fn register_constraint_schema(&mut self, schema: ConstraintSchema) {
        self.constraint_schemas.insert(schema.name.clone(), schema);
    }

    fn register_motion_schema(&mut self, schema: MotionSchema) {
        self.motion_schemas.insert(schema.name.clone(), schema);
    }

    fn validate_entities(&mut self, entities: &[AstEntity]) {
        for entity in entities {
            // Validate entity kind
            let valid_kinds = [
                "solid",
                "light",
                "camera",
                "particle_system",
                "sketch",
                "feature",
                "annotation",
            ];
            if !valid_kinds.contains(&entity.kind.as_str()) {
                self.errors.add(
                    DslError::new(
                        ErrorCode::InvalidKind,
                        format!("Unknown entity kind: '{}'", entity.kind),
                        entity.span,
                        self.file.clone(),
                    )
                    .with_help(format!("Valid kinds: {}", valid_kinds.join(", "))),
                );
            }

            // Validate components
            for component in &entity.components {
                self.validate_component(component, &entity.name);
            }
        }
    }

    fn validate_component(&mut self, component: &AstComponent, entity_name: &str) {
        let schema = self.component_schemas.get(&component.name).cloned();

        if schema.is_none() {
            self.errors.add(DslError::new(
                ErrorCode::UnknownComponentType,
                format!("Unknown component type: '{}'", component.name),
                component.span,
                self.file.clone(),
            ));
            return;
        }

        let schema = schema.unwrap();

        // Check required fields
        for required in &schema.required_fields {
            if !component.fields.iter().any(|f| f.name == required.name) {
                self.errors.add(
                    DslError::new(
                        ErrorCode::MissingRequiredField,
                        format!(
                            "Missing required field '{}' in component '{}' of entity '{}'",
                            required.name, component.name, entity_name
                        ),
                        component.span,
                        self.file.clone(),
                    )
                    .with_help(format!("Add field '{}' to the component", required.name)),
                );
            }
        }

        // Validate field types
        for field in &component.fields {
            let field_schema = schema
                .required_fields
                .iter()
                .chain(schema.optional_fields.iter())
                .find(|f| f.name == field.name);

            if let Some(field_schema) = field_schema {
                self.validate_field_type(
                    &field.value,
                    field_schema.field_type.clone(),
                    &field.name,
                    component.span,
                );
            }
        }
    }

    fn validate_constraints(&mut self, constraints: &[AstConstraint]) {
        for constraint in constraints {
            let constraint_type = constraint.constraint_type();

            if constraint_type.is_none() {
                continue; // Already caught by syntax validator
            }

            let constraint_type = constraint_type.unwrap();
            let schema = self.constraint_schemas.get(constraint_type).cloned();

            if schema.is_none() {
                let type_field = constraint.get_field("type").unwrap();
                self.errors.add(DslError::new(
                    ErrorCode::UndefinedConstraintType,
                    format!("Unknown constraint type: '{}'", constraint_type),
                    type_field.span,
                    self.file.clone(),
                ));
                continue;
            }

            let schema = schema.unwrap();

            // Check required fields
            for required in &schema.required_fields {
                if !constraint.fields.iter().any(|f| f.name == required.name) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::MissingRequiredField,
                            format!(
                                "Missing required field '{}' in constraint '{}' of type '{}'",
                                required.name, constraint.name, constraint_type
                            ),
                            constraint.span,
                            self.file.clone(),
                        )
                        .with_help(format!("Add field '{}' to the constraint", required.name)),
                    );
                }
            }

            // Validate field types
            for field in &constraint.fields {
                let field_schema = schema
                    .required_fields
                    .iter()
                    .chain(schema.optional_fields.iter())
                    .find(|f| f.name == field.name);

                if let Some(field_schema) = field_schema {
                    self.validate_field_type(
                        &field.value,
                        field_schema.field_type.clone(),
                        &field.name,
                        constraint.span,
                    );
                }
            }
        }
    }

    fn validate_motions(&mut self, motions: &[AstMotion]) {
        for motion in motions {
            let motion_type = motion.motion_type();

            if motion_type.is_none() {
                continue; // Already caught by syntax validator
            }

            let motion_type = motion_type.unwrap();
            let schema = self.motion_schemas.get(motion_type).cloned();

            if schema.is_none() {
                let type_field = motion.get_field("type").unwrap();
                self.errors.add(DslError::new(
                    ErrorCode::UnknownLibraryConstruct,
                    format!("Unknown motion type: '{}'", motion_type),
                    type_field.span,
                    self.file.clone(),
                ));
                continue;
            }

            let schema = schema.unwrap();

            // Check required fields
            for required in &schema.required_fields {
                if !motion.fields.iter().any(|f| f.name == required.name) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::MissingRequiredField,
                            format!(
                                "Missing required field '{}' in motion '{}' of type '{}'",
                                required.name, motion.name, motion_type
                            ),
                            motion.span,
                            self.file.clone(),
                        )
                        .with_help(format!("Add field '{}' to the motion", required.name)),
                    );
                }
            }

            // Validate field types
            for field in &motion.fields {
                let field_schema = schema
                    .required_fields
                    .iter()
                    .chain(schema.optional_fields.iter())
                    .find(|f| f.name == field.name);

                if let Some(field_schema) = field_schema {
                    self.validate_field_type(
                        &field.value,
                        field_schema.field_type.clone(),
                        &field.name,
                        motion.span,
                    );
                }
            }
        }
    }

    fn validate_field_type(
        &mut self,
        value: &AstValue,
        expected_type: FieldType,
        field_name: &str,
        _context_span: SourceSpan,
    ) {
        let actual_type = match value {
            AstValue::Number(_, _) => FieldType::Number,
            AstValue::String(_, _) => FieldType::String,
            AstValue::Boolean(_, _) => FieldType::Boolean,
            AstValue::Identifier(id, _) => {
                if id == "true" || id == "false" {
                    FieldType::Boolean
                } else {
                    FieldType::Identifier
                }
            }
            AstValue::Vector(vec, _) => {
                if vec.len() == 3 {
                    FieldType::Vector3
                } else {
                    FieldType::Vector
                }
            }
            AstValue::Matrix(_, _) => {
                self.errors.add(
                    DslError::new(
                        ErrorCode::InvalidFieldType,
                        "Matrix values are not supported in schema validation".to_string(),
                        value.span(),
                        self.file.clone(),
                    )
                    .with_help(format!(
                        "Field '{}' must be of type {:?}",
                        field_name, expected_type
                    )),
                );
                return;
            }
            AstValue::List(_, _) => {
                FieldType::List
            }
            AstValue::MathExpression(_, _) => {
                if matches!(expected_type, FieldType::MathExpression | FieldType::Number) {
                    FieldType::MathExpression
                } else {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::InvalidFieldType,
                            "Math expression is not valid for this field".to_string(),
                            value.span(),
                            self.file.clone(),
                        )
                        .with_help(format!(
                            "Field '{}' must be of type {:?}",
                            field_name, expected_type
                        )),
                    );
                    return;
                }
            }
        };

        let vector_compatible = matches!(
            (actual_type.clone(), expected_type.clone()),
            (FieldType::Vector3, FieldType::Vector) | (FieldType::Vector3, FieldType::List)
        );

        if actual_type != expected_type
            && !vector_compatible
            && !(actual_type == FieldType::MathExpression && expected_type == FieldType::Number)
        {
            self.errors.add(
                DslError::new(
                    ErrorCode::InvalidFieldType,
                    format!(
                        "Invalid type for field '{}': expected {:?}, found {:?}",
                        field_name, expected_type, actual_type
                    ),
                    value.span(),
                    self.file.clone(),
                )
                .with_help(format!(
                    "Field '{}' must be of type {:?}",
                    field_name, expected_type
                )),
            );
        }

        // Additional validations
        if expected_type == FieldType::Number {
            if let AstValue::Number(n, span) = value {
                if !n.is_finite() {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidNumber,
                        format!("Field '{}' must be a finite number", field_name),
                        *span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }

    fn validate_math_objects(&mut self, math_objects: &[MathObjectNode]) {
        for math_object in math_objects {
            match math_object {
                MathObjectNode::Function(node) => {
                    self.validate_domain(&node.domain, node.span, &node.name);
                    self.validate_math_expr(&node.body, node.span, &node.name);
                }
                MathObjectNode::Curve(node) => {
                    self.validate_domain(&node.domain, node.span, &node.name);
                    self.validate_math_expr(&node.definition, node.span, &node.name);
                }
                MathObjectNode::Surface(node) => {
                    self.validate_domain(&node.domain, node.span, &node.name);
                    self.validate_math_expr(&node.definition, node.span, &node.name);
                }
                MathObjectNode::VectorField(node) => {
                    self.validate_domain(&node.domain, node.span, &node.name);
                    if node.dimension == 0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidFieldType,
                            format!("Vector field '{}' must have dimension >= 1", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    for component in &node.components {
                        self.validate_math_expr(component, node.span, &node.name);
                    }
                }
                MathObjectNode::ScalarField(node) => {
                    self.validate_domain(&node.domain, node.span, &node.name);
                    self.validate_math_expr(&node.expression, node.span, &node.name);
                }
                MathObjectNode::Transformation(node) => {
                    self.validate_math_expr(&node.expression, node.span, &node.name);
                }
                MathObjectNode::DifferentialEquation(node) => {
                    if node.order == 0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidFieldType,
                            format!("Differential equation '{}' must have order >= 1", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expr(&node.equation, node.span, &node.name);
                }
                MathObjectNode::MatrixDefinition(node) => {
                    if node.rows == 0 || node.cols == 0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidFieldType,
                            format!("Matrix '{}' must have non-zero dimensions", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    if node.elements.len() != node.rows
                        || node.elements.iter().any(|row| row.len() != node.cols)
                    {
                        self.errors.add(DslError::new(
                            ErrorCode::DimensionMismatch,
                            format!(
                                "Matrix '{}' elements do not match declared dimensions {}x{}",
                                node.name, node.rows, node.cols
                            ),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    for row in &node.elements {
                        for expr in row {
                            self.validate_math_expr(expr, node.span, &node.name);
                        }
                    }
                }
            }
        }
    }

    fn validate_concept_ref(&mut self, concept_ref: Option<&ConceptAnnotation>) {
        if let Some(concept_ref) = concept_ref {
            if concept_ref.concept_id.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "concept_ref.concept_id must be non-empty".to_string(),
                    SourceSpan::single_point(0, 0, 0),
                    self.file.clone(),
                ));
            }
            if concept_ref.section_id.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "concept_ref.section_id must be non-empty".to_string(),
                    SourceSpan::single_point(0, 0, 0),
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_annotations(&mut self, annotations: &[AnnotationNode]) {
        for annotation in annotations {
            if annotation.anchor_entity_id.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "annotation.anchor is required".to_string(),
                    annotation.span,
                    self.file.clone(),
                ));
            }
            if annotation.label_text.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "annotation.label must be non-empty".to_string(),
                    annotation.span,
                    self.file.clone(),
                ));
            }
            if annotation.position_offset.iter().any(|v| !v.is_finite()) {
                self.errors.add(DslError::new(
                    ErrorCode::InvalidFieldType,
                    "annotation.offset must contain finite numeric values".to_string(),
                    annotation.span,
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_highlight_schedule(&mut self, entries: &[HighlightScheduleEntry]) {
        for entry in entries {
            if !entry.at_time.is_finite() || entry.at_time < 0.0 {
                self.errors.add(DslError::new(
                    ErrorCode::InvalidTimeValue,
                    "highlight_schedule.at must be a finite time >= 0".to_string(),
                    entry.span,
                    self.file.clone(),
                ));
            }
            if entry.highlight_token.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "highlight_schedule.token is required".to_string(),
                    entry.span,
                    self.file.clone(),
                ));
            }
            if entry.entity_id.trim().is_empty() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    "highlight_schedule.entity is required".to_string(),
                    entry.span,
                    self.file.clone(),
                ));
            }
            if entry.color_index > 15 {
                self.errors.add(DslError::new(
                    ErrorCode::InvalidFieldType,
                    "highlight_schedule.color_index must be in [0, 15]".to_string(),
                    entry.span,
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_domain(&mut self, domain: &DomainConstraint, span: SourceSpan, object_name: &str) {
        if domain.variables.is_empty() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!(
                    "Math object '{}' must declare at least one domain variable",
                    object_name
                ),
                span,
                self.file.clone(),
            ));
        }
    }

    fn validate_math_expr(&mut self, expr: &AnnotatedExpr, span: SourceSpan, object_name: &str) {
        match &expr.expr {
            MathExpression::FunctionCall(name, args) => {
                if name.is_empty() {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidFieldType,
                        format!(
                            "Math object '{}' has an empty function call name",
                            object_name
                        ),
                        span,
                        self.file.clone(),
                    ));
                }
                for arg in args {
                    self.validate_math_expr(arg, span, object_name);
                }
            }
            MathExpression::UnaryOp(_, inner) => self.validate_math_expr(inner, span, object_name),
            MathExpression::BinaryOp(lhs, _, rhs) => {
                self.validate_math_expr(lhs, span, object_name);
                self.validate_math_expr(rhs, span, object_name);
            }
            MathExpression::Derivative { expression, .. }
            | MathExpression::Integral { expression, .. }
            | MathExpression::Limit { expression, .. }
            | MathExpression::Summation { expression, .. }
            | MathExpression::Product { expression, .. } => {
                self.validate_math_expr(expression, span, object_name);
            }
            MathExpression::Piecewise(cases) => {
                if cases.is_empty() {
                    self.errors.add(DslError::new(
                        ErrorCode::MissingRequiredField,
                        format!(
                            "Math object '{}' has an empty piecewise expression",
                            object_name
                        ),
                        span,
                        self.file.clone(),
                    ));
                }
            }
            MathExpression::MatrixExpr(rows) => {
                if rows.is_empty() || rows.first().is_some_and(|row| row.is_empty()) {
                    self.errors.add(DslError::new(
                        ErrorCode::DimensionMismatch,
                        format!(
                            "Math object '{}' has an empty matrix expression",
                            object_name
                        ),
                        span,
                        self.file.clone(),
                    ));
                }
            }
            MathExpression::VectorExpr(values) => {
                if values.is_empty() {
                    self.errors.add(DslError::new(
                        ErrorCode::DimensionMismatch,
                        format!(
                            "Math object '{}' has an empty vector expression",
                            object_name
                        ),
                        span,
                        self.file.clone(),
                    ));
                }
            }
            MathExpression::Number(_)
            | MathExpression::Variable(_)
            | MathExpression::Constant(_)
            | MathExpression::ComplexNumber { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_validation() {
        assert_eq!(FieldType::Number, FieldType::Number);
        assert_ne!(FieldType::Number, FieldType::String);
    }

    #[test]
    fn test_schema_registration() {
        let validator = SchemaValidator::new(PathBuf::from("test.dsl"));

        assert!(validator.component_schemas.contains_key("transform"));
        assert!(validator.component_schemas.contains_key("geometry"));
        assert!(validator.component_schemas.contains_key("physical"));

        assert!(validator.constraint_schemas.contains_key("gear_relation"));
        assert!(validator.motion_schemas.contains_key("rotation"));
    }
}
