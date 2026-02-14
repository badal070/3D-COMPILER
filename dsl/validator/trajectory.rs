/// Trajectory validation pass.
/// Validates trajectory paths and targets.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct TrajectoryValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl TrajectoryValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        // Build entity table
        let entity_table = self.build_entity_table(&ast.entities);

        // Validate each trajectory
        for trajectory in &ast.trajectories {
            self.validate_trajectory(trajectory, &entity_table);
        }

        self.errors.into_result(())
    }

    fn build_entity_table<'a>(&self, entities: &'a [AstEntity]) -> HashMap<String, &'a AstEntity> {
        entities.iter().map(|e| (e.name.clone(), e)).collect()
    }

    fn validate_trajectory(
        &mut self,
        trajectory: &AstTrajectory,
        entity_table: &HashMap<String, &AstEntity>,
    ) {
        // Validate type field exists
        if trajectory.path_type().is_none() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!("Trajectory '{}' missing 'type' field", trajectory.name),
                trajectory.span,
                self.file.clone(),
            ));
        }

        // Validate path type is valid
        if let Some(path_type) = trajectory.path_type() {
            let valid_types = ["linear", "bezier", "spline", "circular"];
            if !valid_types.contains(&path_type) {
                if let Some(type_field) = trajectory.get_field("type") {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidFieldType,
                        format!(
                            "Invalid trajectory type: '{}'. Valid types: {}",
                            path_type,
                            valid_types.join(", ")
                        ),
                        type_field.span,
                        self.file.clone(),
                    ));
                }
            }
        }

        // Validate target entity exists
        if let Some(target) = trajectory.target() {
            if !entity_table.contains_key(target) {
                if let Some(target_field) = trajectory.get_field("target") {
                    self.errors.add(DslError::new(
                        ErrorCode::UndefinedEntity,
                        format!(
                            "Trajectory '{}' references undefined entity '{}'",
                            trajectory.name, target
                        ),
                        target_field.span,
                        self.file.clone(),
                    ));
                }
            }
        } else {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!("Trajectory '{}' missing 'target' field", trajectory.name),
                trajectory.span,
                self.file.clone(),
            ));
        }

        // Validate path-specific fields
        if let Some(path_type) = trajectory.path_type() {
            match path_type {
                "linear" => self.validate_linear_trajectory(trajectory),
                "bezier" => self.validate_bezier_trajectory(trajectory),
                "spline" => self.validate_spline_trajectory(trajectory),
                "circular" => self.validate_circular_trajectory(trajectory),
                _ => {}
            }
        }
    }

    fn validate_linear_trajectory(&mut self, trajectory: &AstTrajectory) {
        let required_fields = ["start", "end"];
        for field_name in &required_fields {
            if trajectory.get_field(field_name).is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!(
                        "Linear trajectory '{}' missing required field '{}'",
                        trajectory.name, field_name
                    ),
                    trajectory.span,
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_bezier_trajectory(&mut self, trajectory: &AstTrajectory) {
        let required_fields = ["start", "control1", "control2", "end"];
        for field_name in &required_fields {
            if trajectory.get_field(field_name).is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!(
                        "Bezier trajectory '{}' missing required field '{}'",
                        trajectory.name, field_name
                    ),
                    trajectory.span,
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_spline_trajectory(&mut self, trajectory: &AstTrajectory) {
        if trajectory.get_field("points").is_none() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!(
                    "Spline trajectory '{}' missing required field 'points'",
                    trajectory.name
                ),
                trajectory.span,
                self.file.clone(),
            ));
        }
    }

    fn validate_circular_trajectory(&mut self, trajectory: &AstTrajectory) {
        let required_fields = ["center", "radius"];
        for field_name in &required_fields {
            if trajectory.get_field(field_name).is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!(
                        "Circular trajectory '{}' missing required field '{}'",
                        trajectory.name, field_name
                    ),
                    trajectory.span,
                    self.file.clone(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = TrajectoryValidator::new(PathBuf::from("test.dsl"));
        assert!(!validator.errors.has_errors());
    }
}
