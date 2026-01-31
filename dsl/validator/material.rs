/// Material validation pass.
/// Validates material properties based on domain.

use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::path::PathBuf;

pub struct MaterialValidator {
    file: PathBuf,
    errors: ErrorCollector,
    domain: String,
}

impl MaterialValidator {
    pub fn new(file: PathBuf, domain: String) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
            domain,
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        for material in &ast.materials {
            self.validate_material(material);
        }
        self.errors.into_result(())
    }

    fn validate_material(&mut self, material: &AstMaterial) {
        match self.domain.as_str() {
            "physics" => self.validate_physics_material(material),
            "chemistry" => self.validate_chemistry_material(material),
            _ => {}
        }
    }

    fn validate_physics_material(&mut self, material: &AstMaterial) {
        // Validate required physics properties
        let required_fields = ["density", "elasticity", "friction"];
        
        for field_name in &required_fields {
            if !material.fields.iter().any(|f| f.name == *field_name) {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!("Physics material '{}' missing required field '{}'", material.name, field_name),
                    material.span,
                    self.file.clone(),
                ));
            }
        }

        // Validate field values are positive
        for field in &material.fields {
            if let AstValue::Number(val, span) = field.value {
                if val < 0.0 {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidNumber,
                        format!("Material property '{}' cannot be negative", field.name),
                        span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }

    fn validate_chemistry_material(&mut self, material: &AstMaterial) {
        // Validate required chemistry properties
        let required_fields = ["molecular_weight", "state", "reactivity"];
        
        for field_name in &required_fields {
            if !material.fields.iter().any(|f| f.name == *field_name) {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!("Chemistry material '{}' missing required field '{}'", material.name, field_name),
                    material.span,
                    self.file.clone(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::SourceSpan;

    #[test]
    fn test_validator_creation() {
        let validator = MaterialValidator::new(
            PathBuf::from("test.dsl"),
            "physics".to_string()
        );
        assert_eq!(validator.domain, "physics");
    }
}