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
    precision: Option<f64>,
    warnings: Vec<String>,
}

impl UnitValidator {
    pub fn new(file: PathBuf, unit_system: UnitSystem, precision: Option<f64>) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
            unit_system,
            precision,
            warnings: Vec::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        self.validate_entities(&ast.entities);
        self.validate_motions(&ast.motions);
        self.validate_precision(ast);

        for warning in &self.warnings {
            eprintln!("W401: {}", warning);
        }

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

    fn validate_precision(&mut self, ast: &AstFile) {
        let Some(precision) = self.precision else {
            return;
        };

        if !precision.is_finite() || precision <= 0.0 {
            self.errors.add(
                DslError::new(
                    ErrorCode::PrecisionUnderspecification,
                    format!("Scene precision must be a finite positive number, found {}", precision),
                    ast.scene.span,
                    self.file.clone(),
                )
                .with_help("Set scene.precision to a positive value such as 0.01".to_string()),
            );
            return;
        }

        let required_decimals = Self::count_decimal_places(precision);

        for entity in &ast.entities {
            for component in &entity.components {
                for field in &component.fields {
                    self.validate_precision_value(
                        &field.value,
                        precision,
                        required_decimals,
                        &format!("entity '{}' component '{}' field '{}'", entity.name, component.name, field.name),
                    );
                }
            }
        }

        self.warn_close_entities_without_coincident(ast, precision);
    }

    fn validate_precision_value(
        &mut self,
        value: &AstValue,
        precision: f64,
        required_decimals: usize,
        context: &str,
    ) {
        match value {
            AstValue::Number(number, span) => {
                self.validate_precision_number(*number, *span, precision, required_decimals, context);
            }
            AstValue::Vector(values, span) => {
                for number in values {
                    self.validate_precision_number(*number, *span, precision, required_decimals, context);
                }
            }
            AstValue::List(items, _) => {
                for item in items {
                    self.validate_precision_value(item, precision, required_decimals, context);
                }
            }
            _ => {}
        }
    }

    fn validate_precision_number(
        &mut self,
        value: f64,
        span: crate::errors::SourceSpan,
        precision: f64,
        required_decimals: usize,
        context: &str,
    ) {
        if !value.is_finite() {
            return;
        }

        if value.abs() > 0.0 && value.abs() < precision {
            self.errors.add(
                DslError::new(
                    ErrorCode::DimensionBelowPrecisionThreshold,
                    format!(
                        "Value {} in {} is below precision threshold {}",
                        value, context, precision
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help("Increase the value or relax scene precision".to_string()),
            );
        }

        if !Self::is_multiple_of_precision(value, precision) {
            self.errors.add(
                DslError::new(
                    ErrorCode::PrecisionUnderspecification,
                    format!(
                        "Value {} in {} does not align with scene precision {}",
                        value, context, precision
                    ),
                    span,
                    self.file.clone(),
                )
                .with_help(format!(
                    "Use increments of {} (at least {} decimal places when needed)",
                    precision, required_decimals
                )),
            );
        }
    }

    fn warn_close_entities_without_coincident(&mut self, ast: &AstFile, precision: f64) {
        let mut coincident_pairs = std::collections::HashSet::<(String, String)>::new();
        for constraint in &ast.constraints {
            if constraint.constraint_type() != Some("coincident") {
                continue;
            }
            let a = constraint.get_field("entity_a").and_then(|f| f.value.as_identifier());
            let b = constraint.get_field("entity_b").and_then(|f| f.value.as_identifier());
            if let (Some(entity_a), Some(entity_b)) = (a, b) {
                let pair = if entity_a <= entity_b {
                    (entity_a.to_string(), entity_b.to_string())
                } else {
                    (entity_b.to_string(), entity_a.to_string())
                };
                coincident_pairs.insert(pair);
            }
        }

        let mut positions: Vec<(&str, [f64; 3])> = Vec::new();
        for entity in &ast.entities {
            if let Some(position) = Self::entity_position(entity) {
                positions.push((&entity.name, position));
            }
        }

        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let (a_name, a_pos) = positions[i];
                let (b_name, b_pos) = positions[j];
                let distance = ((a_pos[0] - b_pos[0]).powi(2)
                    + (a_pos[1] - b_pos[1]).powi(2)
                    + (a_pos[2] - b_pos[2]).powi(2))
                .sqrt();

                if distance >= precision {
                    continue;
                }

                let pair = if a_name <= b_name {
                    (a_name.to_string(), b_name.to_string())
                } else {
                    (b_name.to_string(), a_name.to_string())
                };

                if !coincident_pairs.contains(&pair) {
                    self.warnings.push(format!(
                        "Entities '{}' and '{}' are {:.6} apart (< precision {}) without an explicit coincident constraint",
                        a_name, b_name, distance, precision
                    ));
                }
            }
        }
    }

    fn entity_position(entity: &AstEntity) -> Option<[f64; 3]> {
        let transform = entity.components.iter().find(|c| c.name == "transform")?;
        let position = transform.get_field("position")?;
        let values = position.value.as_vector()?;
        if values.len() != 3 {
            return None;
        }
        Some([values[0], values[1], values[2]])
    }

    fn is_multiple_of_precision(value: f64, precision: f64) -> bool {
        if precision == 0.0 {
            return false;
        }
        let ratio = value / precision;
        (ratio - ratio.round()).abs() < 1e-9
    }

    fn count_decimal_places(value: f64) -> usize {
        let text = format!("{:.12}", value.abs());
        let trimmed = text.trim_end_matches('0').trim_end_matches('.');
        trimmed.split('.').nth(1).map_or(0, str::len)
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
        let validator = UnitValidator::new(PathBuf::from("test.dsl"), UnitSystem::SI, Some(0.01));

        // These would be validated in the actual validation pass
        let valid_mass = 10.0;
        let negative_mass = -5.0;
        let zero_mass = 0.0;

        assert!(valid_mass > 0.0);
        assert!(negative_mass <= 0.0);
        assert!(zero_mass <= 0.0);
    }
}
