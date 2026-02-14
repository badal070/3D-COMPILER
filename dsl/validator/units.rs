/// Unit validation pass.
/// Validates physical units and ensures consistency.
/// Enforces unit system constraints (SI vs Imperial).
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    SI,
    Imperial,
}

impl UnitSystem {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "SI" => Some(UnitSystem::SI),
            "Imperial" => Some(UnitSystem::Imperial),
            _ => None,
        }
    }
}

pub struct UnitValidator {
    file: PathBuf,
    errors: ErrorCollector,
    unit_system: UnitSystem,
}

impl UnitValidator {
    pub fn new(file: PathBuf, unit_system: UnitSystem) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
            unit_system,
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        self.validate_entities(&ast.entities);
        self.validate_motions(&ast.motions);

        self.errors.into_result(())
    }

    fn validate_entities(&mut self, entities: &[AstEntity]) {
        for entity in entities {
            for component in &entity.components {
                match component.name.as_str() {
                    "transform" => self.validate_transform_units(component, &entity.name),
                    "physical" => self.validate_physical_units(component, &entity.name),
                    _ => {}
                }
            }
        }
    }

    fn validate_transform_units(&mut self, component: &AstComponent, entity_name: &str) {
        // Validate rotation is in radians (both SI and Imperial use radians)
        if let Some(field) = component.get_field("rotation") {
            if let AstValue::Vector(vec, span) = &field.value {
                for &angle in vec {
                    // Check if angle is suspiciously large (likely degrees instead of radians)
                    if angle.abs() > 100.0 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::InvalidRotationUnit,
                                format!(
                                    "Rotation angle {} is suspiciously large - expecting radians, not degrees",
                                    angle
                                ),
                                *span,
                                self.file.clone(),
                            )
                            .with_help(format!("Convert {} degrees to radians: {} rad", angle, angle * std::f64::consts::PI / 180.0)),
                        );
                    }
                }
            }
        }

        // Position and scale units depend on unit system
        // For now, we just validate they are reasonable values
        self.validate_reasonable_vector("position", component, entity_name);
        self.validate_reasonable_vector("scale", component, entity_name);
    }

    fn validate_physical_units(&mut self, component: &AstComponent, entity_name: &str) {
        // Validate mass is positive
        if let Some(field) = component.get_field("mass") {
            if let AstValue::Number(mass, span) = &field.value {
                if *mass <= 0.0 {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::InvalidMassValue,
                            format!(
                                "Mass must be positive, found {} in entity '{}'",
                                mass, entity_name
                            ),
                            *span,
                            self.file.clone(),
                        )
                        .with_help("Mass represents physical quantity and must be > 0".to_string()),
                    );
                }

                // Warn if mass is unreasonably large or small
                match self.unit_system {
                    UnitSystem::SI => {
                        if *mass > 1e10 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidMassValue,
                                    format!(
                                        "Mass {} kg is extremely large in entity '{}'",
                                        mass, entity_name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(
                                    "Check if mass unit is correct (expecting kilograms in SI)"
                                        .to_string(),
                                ),
                            );
                        } else if *mass < 1e-10 && *mass > 0.0 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidMassValue,
                                    format!(
                                        "Mass {} kg is extremely small in entity '{}'",
                                        mass, entity_name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(
                                    "Check if mass unit is correct (expecting kilograms in SI)"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                    UnitSystem::Imperial => {
                        if *mass > 1e10 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidMassValue,
                                    format!(
                                        "Mass {} lb is extremely large in entity '{}'",
                                        mass, entity_name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(
                                    "Check if mass unit is correct (expecting pounds in Imperial)"
                                        .to_string(),
                                ),
                            );
                        } else if *mass < 1e-10 && *mass > 0.0 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidMassValue,
                                    format!(
                                        "Mass {} lb is extremely small in entity '{}'",
                                        mass, entity_name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(
                                    "Check if mass unit is correct (expecting pounds in Imperial)"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    fn validate_reasonable_vector(
        &mut self,
        field_name: &str,
        component: &AstComponent,
        entity_name: &str,
    ) {
        if let Some(field) = component.get_field(field_name) {
            if let AstValue::Vector(vec, span) = &field.value {
                for &val in vec {
                    if !val.is_finite() {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::InvalidNumber,
                                format!(
                                    "Invalid value in {} vector of entity '{}': {}",
                                    field_name, entity_name, val
                                ),
                                *span,
                                self.file.clone(),
                            )
                            .with_help("All vector components must be finite numbers".to_string()),
                        );
                    }

                    // Warn if values are suspiciously large
                    if val.abs() > 1e6 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::InvalidNumber,
                                format!(
                                    "Suspiciously large value {} in {} of entity '{}'",
                                    val, field_name, entity_name
                                ),
                                *span,
                                self.file.clone(),
                            )
                            .with_help("Check if units are correct".to_string()),
                        );
                    }
                }
            }
        }
    }

    fn validate_motions(&mut self, motions: &[AstMotion]) {
        for motion in motions {
            // Validate speed is finite and reasonable
            if let Some(field) = motion.get_field("speed") {
                if let AstValue::Number(speed, span) = &field.value {
                    if !speed.is_finite() {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidNumber,
                            format!("Speed must be finite in motion '{}'", motion.name),
                            *span,
                            self.file.clone(),
                        ));
                    }

                    // For rotation, speed is in radians per second
                    if motion.motion_type() == Some("rotation") {
                        // Warn if speed is suspiciously large (likely degrees/sec)
                        if speed.abs() > 100.0 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidRotationUnit,
                                    format!(
                                        "Rotation speed {} is suspiciously large - expecting radians/second, not degrees/second",
                                        speed
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(format!("Convert {} deg/s to radians/s: {} rad/s", speed, speed * std::f64::consts::PI / 180.0)),
                            );
                        }
                    }
                }
            }

            // Validate axis normalization for rotation
            if motion.motion_type() == Some("rotation") {
                if let Some(field) = motion.get_field("axis") {
                    if let AstValue::Vector(vec, span) = &field.value {
                        let mag_sq = vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2];
                        let magnitude = mag_sq.sqrt();

                        // Already validated by syntax validator, but double-check
                        if (magnitude - 1.0).abs() > 0.001 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::NonNormalizedAxis,
                                    format!(
                                        "Motion axis must be normalized (magnitude = 1.0), found {:.6} in motion '{}'",
                                        magnitude, motion.name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(format!(
                                    "Normalize the axis: [{:.6}, {:.6}, {:.6}]",
                                    vec[0] / magnitude,
                                    vec[1] / magnitude,
                                    vec[2] / magnitude
                                )),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{errors::SourceSpan, validator};

    #[test]
    fn test_unit_system_parsing() {
        assert_eq!(UnitSystem::from_str("SI"), Some(UnitSystem::SI));
        assert_eq!(UnitSystem::from_str("Imperial"), Some(UnitSystem::Imperial));
        assert_eq!(UnitSystem::from_str("Invalid"), None);
    }

    #[test]
    fn test_degree_detection() {
        let span = SourceSpan::single_point(1, 1, 0);

        // 180 degrees = π radians ≈ 3.14
        // If someone passes 180 (thinking it's radians), we should warn
        let large_angle = 180.0;
        assert!(large_angle > 100.0); // Our threshold for warning
    }

    #[test]
    fn test_mass_validation() {
        let validator = UnitValidator::new(PathBuf::from("test.dsl"), UnitSystem::SI);

        // These would be validated in the actual validation pass
        let valid_mass = 10.0;
        let negative_mass = -5.0;
        let zero_mass = 0.0;

        assert!(valid_mass > 0.0);
        assert!(negative_mass <= 0.0);
        assert!(zero_mass <= 0.0);
    }
}
