/// Field validation pass.
/// Validates field definitions and types.

use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::path::PathBuf;
use std::collections::HashSet;

pub struct FieldValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl FieldValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        // Check for duplicate field names
        let mut seen_names = HashSet::new();

        for field_def in &ast.fields {
            if !seen_names.insert(&field_def.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateField,
                    format!("Duplicate field definition: '{}'", field_def.name),
                    field_def.span,
                    self.file.clone(),
                ));
            }

            self.validate_field_def(field_def);
        }

        self.errors.into_result(())
    }

    fn validate_field_def(&mut self, field_def: &AstFieldDef) {
        // Validate field type exists
        if field_def.field_type().is_none() {
            self.errors.add(DslError::new(
                ErrorCode::MissingRequiredField,
                format!("Field definition '{}' missing 'type' field", field_def.name),
                field_def.span,
                self.file.clone(),
            ));
        }

        // Validate field type is valid
        if let Some(field_type) = field_def.field_type() {
            let valid_types = ["scalar", "vector", "string", "identifier"];
            if !valid_types.contains(&field_type) {
                if let Some(type_field) = field_def.get_field("type") {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidFieldType,
                        format!("Invalid field type: '{}'. Valid types: {}", 
                            field_type, valid_types.join(", ")),
                        type_field.span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = FieldValidator::new(PathBuf::from("test.dsl"));
        assert!(!validator.errors.has_errors());
    }
}