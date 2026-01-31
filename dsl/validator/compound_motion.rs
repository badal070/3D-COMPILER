/// Compound motion validation pass.
/// Validates compound motions reference existing motions.

use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::path::PathBuf;
use std::collections::HashMap;

pub struct CompoundMotionValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl CompoundMotionValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        // Build motion table
        let motion_table = self.build_motion_table(&ast.motions);

        // Validate each compound motion
        for compound_motion in &ast.compound_motions {
            self.validate_compound_motion(compound_motion, &motion_table);
        }

        self.errors.into_result(())
    }

    fn build_motion_table<'a>(&self, motions: &'a [AstMotion]) -> HashMap<String, &'a AstMotion> {
        motions.iter()
            .map(|m| (m.name.clone(), m))
            .collect()
    }

    fn validate_compound_motion(&mut self, compound_motion: &AstCompoundMotion, motion_table: &HashMap<String, &AstMotion>) {
        // Validate type field exists
        if compound_motion.motion_type().is_none() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!("Compound motion '{}' missing 'type' field", compound_motion.name),
                compound_motion.span,
                self.file.clone(),
            ));
        }

        // Validate motion type is valid
        if let Some(motion_type) = compound_motion.motion_type() {
            let valid_types = ["sequential", "parallel", "conditional"];
            if !valid_types.contains(&motion_type) {
                if let Some(type_field) = compound_motion.get_field("type") {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidFieldType,
                        format!("Invalid compound motion type: '{}'. Valid types: {}", 
                            motion_type, valid_types.join(", ")),
                        type_field.span,
                        self.file.clone(),
                    ));
                }
            }
        }

        // Validate referenced motions exist
        let motion_list = compound_motion.motion_list();
        for motion_name in &motion_list {
            if !motion_table.contains_key(motion_name) {
                self.errors.add(DslError::new(
                    ErrorCode::UndefinedMotion,
                    format!("Compound motion '{}' references undefined motion '{}'", 
                        compound_motion.name, motion_name),
                    compound_motion.span,
                    self.file.clone(),
                ));
            }
        }

        // Validate at least one motion is specified
        if motion_list.is_empty() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!("Compound motion '{}' must specify at least one motion", compound_motion.name),
                compound_motion.span,
                self.file.clone(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = CompoundMotionValidator::new(PathBuf::from("test.dsl"));
        assert!(!validator.errors.has_errors());
    }
}