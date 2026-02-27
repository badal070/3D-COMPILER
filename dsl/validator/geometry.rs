use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct GeometryValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl GeometryValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        let entities: HashMap<&str, &AstEntity> =
            ast.entities.iter().map(|entity| (entity.name.as_str(), entity)).collect();

        self.validate_boolean_operations(ast, &entities);
        self.validate_sketch_based_features(&entities);
        self.validate_lofts(&entities);
        self.validate_edge_features(&entities);

        self.errors.into_result(())
    }

    fn validate_boolean_operations(
        &mut self,
        ast: &AstFile,
        entities: &HashMap<&str, &AstEntity>,
    ) {
        for constraint in &ast.constraints {
            let Some(kind) = constraint.constraint_type() else {
                continue;
            };

            if !matches!(kind, "boolean_subtract" | "boolean_union" | "boolean_intersect") {
                continue;
            }

            let target = constraint
                .get_field("target")
                .and_then(|field| field.value.as_identifier());
            let tool = constraint
                .get_field("tool")
                .and_then(|field| field.value.as_identifier());

            self.validate_boolean_pair(target, tool, entities, constraint.span, kind);
        }

        for entity in entities.values() {
            for component in &entity.components {
                if component.name != "boolean_op" {
                    continue;
                }

                let operation = component
                    .get_field("operation")
                    .and_then(|field| field.value.as_identifier())
                    .unwrap_or("boolean_op");
                let target = component
                    .get_field("target")
                    .and_then(|field| field.value.as_identifier());
                let tool = component
                    .get_field("tool")
                    .and_then(|field| field.value.as_identifier());

                self.validate_boolean_pair(target, tool, entities, component.span, operation);
            }
        }
    }

    fn validate_boolean_pair(
        &mut self,
        target: Option<&str>,
        tool: Option<&str>,
        entities: &HashMap<&str, &AstEntity>,
        span: crate::errors::SourceSpan,
        op_kind: &str,
    ) {
        let (Some(target), Some(tool)) = (target, tool) else {
            self.errors.add(
                DslError::new(
                    ErrorCode::InvalidBooleanOperation,
                    format!(
                        "Invalid {} operation: both 'target' and 'tool' references are required",
                        op_kind
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help("Specify valid target/tool entity ids for boolean operations".to_string()),
            );
            return;
        };

        if target == tool {
            self.errors.add(
                DslError::new(
                    ErrorCode::ConstraintConflictDetected,
                    format!(
                        "Invalid {} operation: tool and target cannot be the same entity ('{}')",
                        op_kind, target
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help("Use two distinct closed solids for boolean operations".to_string()),
            );
            return;
        }

        let target_entity = entities.get(target).copied();
        let tool_entity = entities.get(tool).copied();
        if target_entity.is_none() || tool_entity.is_none() {
            self.errors.add(
                DslError::new(
                    ErrorCode::FeatureDependsOnDeletedGeometry,
                    format!(
                        "Boolean operation references missing geometry (target='{}', tool='{}')",
                        target, tool
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help("Ensure boolean target and tool entities are defined before use".to_string()),
            );
            return;
        }

        let target_entity = target_entity.unwrap();
        let tool_entity = tool_entity.unwrap();

        if !Self::is_closed_solid(target_entity) || !Self::is_closed_solid(tool_entity) {
            self.errors.add(
                DslError::new(
                    ErrorCode::InvalidBooleanOperation,
                    format!(
                        "Invalid {} operation: tool and target must both be closed solids",
                        op_kind
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help("Use box/sphere/cylinder/cone/torus-like closed primitives for booleans".to_string()),
            );
        }
    }

    fn validate_sketch_based_features(&mut self, entities: &HashMap<&str, &AstEntity>) {
        for entity in entities.values() {
            for component in &entity.components {
                if !matches!(component.name.as_str(), "extrude" | "revolve") {
                    continue;
                }

                let sketch_ref = component
                    .get_field("sketch_ref")
                    .and_then(|field| field.value.as_identifier());

                let Some(sketch_ref) = sketch_ref else {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::SketchNotClosed,
                            format!(
                                "{} feature '{}' is missing required sketch_ref",
                                component.name, entity.name
                            ),
                            component.span,
                            self.file.clone(),
                        )
                        .with_help("Reference a closed sketch entity using sketch_ref".to_string()),
                    );
                    continue;
                };

                let Some(sketch_entity) = entities.get(sketch_ref) else {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::FeatureDependsOnDeletedGeometry,
                            format!(
                                "{} feature '{}' references missing sketch '{}'",
                                component.name, entity.name, sketch_ref
                            ),
                            component.span,
                            self.file.clone(),
                        )
                        .with_help("Define the referenced sketch entity before this feature".to_string()),
                    );
                    continue;
                };

                if !Self::is_closed_sketch(sketch_entity) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::SketchNotClosed,
                            format!(
                                "Sketch '{}' must be closed before it can be used in {} '{}'",
                                sketch_ref, component.name, entity.name
                            ),
                            component.span,
                            self.file.clone(),
                        )
                        .with_help("Set sketch.closed to true and provide a non-self-intersecting loop".to_string()),
                    );
                }
            }
        }
    }

    fn validate_lofts(&mut self, entities: &HashMap<&str, &AstEntity>) {
        for entity in entities.values() {
            let Some(loft) = entity.components.iter().find(|component| component.name == "loft") else {
                continue;
            };

            let Some(profiles) = loft.get_field("profiles").and_then(|field| field.value.as_list()) else {
                continue;
            };

            for profile in profiles {
                let Some(profile_id) = profile.as_identifier() else {
                    continue;
                };

                let Some(profile_entity) = entities.get(profile_id).copied() else {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::FeatureDependsOnDeletedGeometry,
                            format!(
                                "Loft '{}' references missing profile sketch '{}'",
                                entity.name, profile_id
                            ),
                            loft.span,
                            self.file.clone(),
                        )
                        .with_help("Ensure every loft profile id points to an existing sketch".to_string()),
                    );
                    continue;
                };

                if !Self::is_closed_sketch(profile_entity) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::SketchNotClosed,
                            format!(
                                "Loft '{}' references non-closed profile sketch '{}'",
                                entity.name, profile_id
                            ),
                            loft.span,
                            self.file.clone(),
                        )
                        .with_help("Only closed sketch profiles can be lofted".to_string()),
                    );
                    continue;
                }

                if Self::sketch_has_self_intersection(profile_entity) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::ConstraintConflictDetected,
                            format!(
                                "Loft profile '{}' for '{}' appears self-intersecting",
                                profile_id, entity.name
                            ),
                            loft.span,
                            self.file.clone(),
                        )
                        .with_help("Provide simple, non-self-intersecting profile loops for loft".to_string()),
                    );
                }
            }
        }
    }

    fn validate_edge_features(&mut self, entities: &HashMap<&str, &AstEntity>) {
        for entity in entities.values() {
            for component in &entity.components {
                let (field_name, feature_label) = match component.name.as_str() {
                    "fillet" => ("radius", "fillet"),
                    "chamfer" => ("distance", "chamfer"),
                    _ => continue,
                };

                let Some(value) = component
                    .get_field(field_name)
                    .and_then(|field| field.value.as_number())
                else {
                    continue;
                };

                let target = component
                    .get_field("target")
                    .and_then(|field| field.value.as_identifier())
                    .unwrap_or(entity.name.as_str());

                let Some(target_entity) = entities.get(target).copied() else {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::FeatureDependsOnDeletedGeometry,
                            format!(
                                "{} '{}' references missing target entity '{}'",
                                feature_label, entity.name, target
                            ),
                            component.span,
                            self.file.clone(),
                        )
                        .with_help("Create the target entity before applying edge features".to_string()),
                    );
                    continue;
                };

                if let Some(min_dimension) = Self::min_dimension(target_entity) {
                    if value > min_dimension {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::ConstraintConflictDetected,
                                format!(
                                    "{} '{}' value {} exceeds available edge length {} on target '{}'",
                                    feature_label, entity.name, value, min_dimension, target
                                ),
                                component.span,
                                self.file.clone(),
                            )
                            .with_help("Reduce the feature size or increase base geometry dimensions".to_string()),
                        );
                    }
                }
            }
        }
    }

    fn is_closed_solid(entity: &AstEntity) -> bool {
        if entity.kind != "solid" {
            return false;
        }

        let primitive = entity
            .components
            .iter()
            .find(|component| component.name == "geometry" || component.name == "solid")
            .and_then(|component| component.get_field("primitive"))
            .and_then(|field| field.value.as_identifier());

        matches!(
            primitive,
            Some("box" | "sphere" | "cylinder" | "cone" | "torus" | "capsule")
        )
    }

    fn is_closed_sketch(entity: &AstEntity) -> bool {
        entity
            .components
            .iter()
            .find(|component| component.name == "sketch")
            .and_then(|component| component.get_field("closed"))
            .and_then(|field| field.value.as_boolean())
            .unwrap_or(false)
    }

    fn sketch_has_self_intersection(entity: &AstEntity) -> bool {
        let Some(points_field) = entity
            .components
            .iter()
            .find(|component| component.name == "sketch")
            .and_then(|component| component.get_field("points"))
        else {
            return false;
        };

        let Some(points) = points_field.value.as_list() else {
            return false;
        };

        let mut seen: Vec<[f64; 3]> = Vec::new();
        for point in points {
            let Some(vector) = point.as_vector() else {
                continue;
            };
            if vector.len() != 3 {
                continue;
            }
            let current = [vector[0], vector[1], vector[2]];
            if seen.iter().any(|prior| Self::same_point(*prior, current)) {
                return true;
            }
            seen.push(current);
        }

        false
    }

    fn same_point(a: [f64; 3], b: [f64; 3]) -> bool {
        (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6 && (a[2] - b[2]).abs() < 1e-6
    }

    fn min_dimension(entity: &AstEntity) -> Option<f64> {
        let dims = entity
            .components
            .iter()
            .find_map(|component| {
                if component.name == "solid" || component.name == "geometry" {
                    component
                        .get_field("dimensions")
                        .and_then(|field| field.value.as_vector())
                } else {
                    None
                }
            })?;

        dims.iter().copied().filter(|value| value.is_finite() && *value > 0.0).fold(
            None,
            |current, next| match current {
                Some(existing) => Some(existing.min(next)),
                None => Some(next),
            },
        )
    }
}
