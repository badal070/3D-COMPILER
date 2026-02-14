/// Syntax validation pass.
/// Ensures AST conforms to structural rules.
/// Pure, fail-fast, no mutation.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector, SourceSpan};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct SyntaxValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl SyntaxValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        self.validate_scene(&ast.scene);
        self.validate_library_imports(&ast.library_imports);
        self.validate_entities(&ast.entities);
        self.validate_constraints(&ast.constraints);
        self.validate_motions(&ast.motions);
        self.validate_timelines(&ast.timelines);
        self.validate_math_objects(&ast.math_objects);

        self.errors.into_result(())
    }

    fn validate_scene(&mut self, scene: &AstScene) {
        // Validate version is positive
        if scene.version < 1 {
            self.errors.add(DslError::new(
                ErrorCode::InvalidVersionFormat,
                format!("Scene version must be >= 1, found {}", scene.version),
                scene.span,
                self.file.clone(),
            ));
        }

        // Validate IR version format (simple semantic versioning check)
        if !is_valid_semver(&scene.ir_version) {
            self.errors.add(
                DslError::new(
                    ErrorCode::InvalidVersionFormat,
                    format!("Invalid IR version format: '{}'", scene.ir_version),
                    scene.span,
                    self.file.clone(),
                )
                .with_help("Expected format: MAJOR.MINOR.PATCH (e.g., '0.1.0')".to_string()),
            );
        }

        // Validate unit system
        let valid_systems = ["SI", "Imperial"];
        if !valid_systems.contains(&scene.unit_system.as_str()) {
            self.errors.add(
                DslError::new(
                    ErrorCode::InvalidUnitSystem,
                    format!("Unknown unit system: '{}'", scene.unit_system),
                    scene.span,
                    self.file.clone(),
                )
                .with_help("Valid unit systems: SI, Imperial".to_string()),
            );
        }
    }

    fn validate_library_imports(&mut self, imports: &AstLibraryImports) {
        let mut seen_aliases = HashSet::new();

        for import in &imports.imports {
            // Check for duplicate aliases
            if !seen_aliases.insert(&import.alias) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateField,
                    format!("Duplicate library alias: '{}'", import.alias),
                    import.span,
                    self.file.clone(),
                ));
            }

            // Validate alias is not a reserved keyword
            if is_reserved_keyword(&import.alias) {
                self.errors.add(DslError::new(
                    ErrorCode::InvalidIdentifier,
                    format!(
                        "Library alias cannot be a reserved keyword: '{}'",
                        import.alias
                    ),
                    import.span,
                    self.file.clone(),
                ));
            }
        }
    }

    fn validate_entities(&mut self, entities: &[AstEntity]) {
        let mut seen_names = HashSet::new();

        for entity in entities {
            // Check for duplicate names
            if !seen_names.insert(&entity.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateEntityName,
                    format!("Duplicate entity name: '{}'", entity.name),
                    entity.span,
                    self.file.clone(),
                ));
            }

            // Validate entity has at least one component
            if entity.components.is_empty() {
                self.errors.add(
                    DslError::new(
                        ErrorCode::MissingRequiredComponent,
                        format!("Entity '{}' has no components", entity.name),
                        entity.span,
                        self.file.clone(),
                    )
                    .with_help("Entities must have at least one component".to_string()),
                );
            }

            // Check for duplicate component types
            let mut seen_components = HashSet::new();
            for component in &entity.components {
                if !seen_components.insert(&component.name) {
                    self.errors.add(DslError::new(
                        ErrorCode::DuplicateComponent,
                        format!(
                            "Duplicate component '{}' in entity '{}'",
                            component.name, entity.name
                        ),
                        component.span,
                        self.file.clone(),
                    ));
                }

                self.validate_component(component, &entity.name);
            }
        }
    }

    fn validate_component(&mut self, component: &AstComponent, entity_name: &str) {
        // Check for duplicate fields
        let mut seen_fields = HashSet::new();
        for field in &component.fields {
            if !seen_fields.insert(&field.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateField,
                    format!(
                        "Duplicate field '{}' in component '{}' of entity '{}'",
                        field.name, component.name, entity_name
                    ),
                    field.span,
                    self.file.clone(),
                ));
            }

            // Validate vector length if it's a vector
            if let AstValue::Vector(vec, span) = &field.value {
                if vec.len() != 3 {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidVectorLength,
                        format!("Vector must have exactly 3 components, found {}", vec.len()),
                        *span,
                        self.file.clone(),
                    ));
                }

                // Check for NaN or Infinity
                for (i, &val) in vec.iter().enumerate() {
                    if !val.is_finite() {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidNumber,
                            format!("Vector component {} is not finite: {}", i, val),
                            *span,
                            self.file.clone(),
                        ));
                    }
                }
            }
        }
    }

    fn validate_constraints(&mut self, constraints: &[AstConstraint]) {
        let mut seen_names = HashSet::new();

        for constraint in constraints {
            // Check for duplicate names
            if !seen_names.insert(&constraint.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateConstraintName,
                    format!("Duplicate constraint name: '{}'", constraint.name),
                    constraint.span,
                    self.file.clone(),
                ));
            }

            // Ensure 'type' field exists
            if constraint.constraint_type().is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!(
                        "Constraint '{}' missing required 'type' field",
                        constraint.name
                    ),
                    constraint.span,
                    self.file.clone(),
                ));
            }

            // Check for duplicate fields
            let mut seen_fields = HashSet::new();
            for field in &constraint.fields {
                if !seen_fields.insert(&field.name) {
                    self.errors.add(DslError::new(
                        ErrorCode::DuplicateField,
                        format!(
                            "Duplicate field '{}' in constraint '{}'",
                            field.name, constraint.name
                        ),
                        field.span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }

    fn validate_motions(&mut self, motions: &[AstMotion]) {
        let mut seen_names = HashSet::new();

        for motion in motions {
            // Check for duplicate names
            if !seen_names.insert(&motion.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateMotionName,
                    format!("Duplicate motion name: '{}'", motion.name),
                    motion.span,
                    self.file.clone(),
                ));
            }

            // Ensure required fields exist
            if motion.target().is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!("Motion '{}' missing required 'target' field", motion.name),
                    motion.span,
                    self.file.clone(),
                ));
            }

            if motion.motion_type().is_none() {
                self.errors.add(DslError::new(
                    ErrorCode::MissingRequiredField,
                    format!("Motion '{}' missing required 'type' field", motion.name),
                    motion.span,
                    self.file.clone(),
                ));
            }

            // Validate axis is normalized if present
            if let Some(axis_field) = motion.get_field("axis") {
                if let AstValue::Vector(vec, span) = &axis_field.value {
                    let magnitude = (vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2]).sqrt();
                    if (magnitude - 1.0).abs() > 0.001 {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::NonNormalizedAxis,
                                format!(
                                    "Motion axis must be normalized (magnitude = 1.0), found {:.3}",
                                    magnitude
                                ),
                                *span,
                                self.file.clone(),
                            )
                            .with_help("Normalize the axis vector before using it".to_string()),
                        );
                    }
                }
            }

            // Validate speed is finite if present
            if let Some(speed_field) = motion.get_field("speed") {
                if let AstValue::Number(speed, span) = &speed_field.value {
                    if !speed.is_finite() {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidNumber,
                            format!("Motion speed must be finite, found {}", speed),
                            *span,
                            self.file.clone(),
                        ));
                    }
                }
            }

            // Check for duplicate fields
            let mut seen_fields = HashSet::new();
            for field in &motion.fields {
                if !seen_fields.insert(&field.name) {
                    self.errors.add(DslError::new(
                        ErrorCode::DuplicateField,
                        format!(
                            "Duplicate field '{}' in motion '{}'",
                            field.name, motion.name
                        ),
                        field.span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }

    fn validate_timelines(&mut self, timelines: &[AstTimeline]) {
        let mut seen_names = HashSet::new();

        for timeline in timelines {
            // Check for duplicate names
            if !seen_names.insert(&timeline.name) {
                self.errors.add(DslError::new(
                    ErrorCode::DuplicateTimelineName,
                    format!("Duplicate timeline name: '{}'", timeline.name),
                    timeline.span,
                    self.file.clone(),
                ));
            }

            for event in &timeline.events {
                // Ensure required fields exist
                if event.motion().is_none() {
                    self.errors.add(DslError::new(
                        ErrorCode::MissingRequiredField,
                        format!(
                            "Event in timeline '{}' missing required 'motion' field",
                            timeline.name
                        ),
                        event.span,
                        self.file.clone(),
                    ));
                }

                // Validate timing
                if let Some(start) = event.start() {
                    if start < 0.0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidTimeValue,
                            format!("Event start time must be >= 0, found {}", start),
                            event.span,
                            self.file.clone(),
                        ));
                    }
                    if !start.is_finite() {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidTimeValue,
                            format!("Event start time must be finite, found {}", start),
                            event.span,
                            self.file.clone(),
                        ));
                    }
                } else {
                    self.errors.add(DslError::new(
                        ErrorCode::MissingRequiredField,
                        format!(
                            "Event in timeline '{}' missing required 'start' field",
                            timeline.name
                        ),
                        event.span,
                        self.file.clone(),
                    ));
                }

                if let Some(duration) = event.duration() {
                    if duration <= 0.0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidDurationValue,
                            format!("Event duration must be > 0, found {}", duration),
                            event.span,
                            self.file.clone(),
                        ));
                    }
                    if !duration.is_finite() {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidDurationValue,
                            format!("Event duration must be finite, found {}", duration),
                            event.span,
                            self.file.clone(),
                        ));
                    }
                } else {
                    self.errors.add(DslError::new(
                        ErrorCode::MissingRequiredField,
                        format!(
                            "Event in timeline '{}' missing required 'duration' field",
                            timeline.name
                        ),
                        event.span,
                        self.file.clone(),
                    ));
                }
            }
        }
    }

    fn validate_math_objects(&mut self, math_objects: &[MathObjectNode]) {
        let mut seen_names = HashSet::new();

        for math_object in math_objects {
            match math_object {
                MathObjectNode::Function(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.body, node.span);
                }
                MathObjectNode::Curve(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.definition, node.span);
                }
                MathObjectNode::Surface(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.definition, node.span);
                }
                MathObjectNode::VectorField(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    if node.dimension != node.components.len() {
                        self.errors.add(DslError::new(
                            ErrorCode::DimensionMismatch,
                            format!(
                                "Vector field '{}' dimension {} does not match component count {}",
                                node.name,
                                node.dimension,
                                node.components.len()
                            ),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    for component in &node.components {
                        self.validate_math_expression(component, node.span);
                    }
                }
                MathObjectNode::ScalarField(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.expression, node.span);
                }
                MathObjectNode::Transformation(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.expression, node.span);
                }
                MathObjectNode::DifferentialEquation(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    if node.order == 0 {
                        self.errors.add(DslError::new(
                            ErrorCode::InvalidFieldType,
                            format!("Differential equation '{}' must have order >= 1", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    self.validate_math_expression(&node.equation, node.span);
                }
                MathObjectNode::MatrixDefinition(node) => {
                    if !seen_names.insert(node.name.clone()) {
                        self.errors.add(DslError::new(
                            ErrorCode::DuplicateEntityName,
                            format!("Duplicate math object name: '{}'", node.name),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    if node.rows != node.elements.len()
                        || node.elements.iter().any(|row| row.len() != node.cols)
                    {
                        self.errors.add(DslError::new(
                            ErrorCode::DimensionMismatch,
                            format!(
                                "Matrix '{}' declared as {}x{} but element shape differs",
                                node.name, node.rows, node.cols
                            ),
                            node.span,
                            self.file.clone(),
                        ));
                    }
                    for row in &node.elements {
                        for expr in row {
                            self.validate_math_expression(expr, node.span);
                        }
                    }
                }
            }
        }
    }

    fn validate_math_expression(&mut self, expr: &MathExpression, span: SourceSpan) {
        match expr {
            MathExpression::Number(n) => {
                if !n.is_finite() {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidNumber,
                        format!("Math literal must be finite, found {}", n),
                        span,
                        self.file.clone(),
                    ));
                }
            }
            MathExpression::ComplexNumber { real, imag } => {
                if !real.is_finite() || !imag.is_finite() {
                    self.errors.add(DslError::new(
                        ErrorCode::InvalidNumber,
                        "Complex literal components must be finite".to_string(),
                        span,
                        self.file.clone(),
                    ));
                }
            }
            MathExpression::UnaryOp(_, inner) => self.validate_math_expression(inner, span),
            MathExpression::BinaryOp(lhs, _, rhs) => {
                self.validate_math_expression(lhs, span);
                self.validate_math_expression(rhs, span);
            }
            MathExpression::FunctionCall(_, args) => {
                for arg in args {
                    self.validate_math_expression(arg, span);
                }
            }
            MathExpression::Derivative { expression, .. }
            | MathExpression::Integral { expression, .. }
            | MathExpression::Limit { expression, .. }
            | MathExpression::Summation { expression, .. }
            | MathExpression::Product { expression, .. } => {
                self.validate_math_expression(expression, span);
            }
            MathExpression::Piecewise(cases) => {
                for (_, value) in cases {
                    self.validate_math_expression(value, span);
                }
            }
            MathExpression::MatrixExpr(rows) => {
                for row in rows {
                    for value in row {
                        self.validate_math_expression(value, span);
                    }
                }
            }
            MathExpression::VectorExpr(values) => {
                for value in values {
                    self.validate_math_expression(value, span);
                }
            }
            MathExpression::Variable(_) | MathExpression::Constant(_) => {}
        }
    }
}

fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

fn is_reserved_keyword(word: &str) -> bool {
    matches!(
        word,
        "scene"
            | "library_imports"
            | "entity"
            | "constraint"
            | "motion"
            | "timeline"
            | "event"
            | "components"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_validation() {
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("1.2.3"));
        assert!(is_valid_semver("10.20.30"));
        assert!(!is_valid_semver("1.2"));
        assert!(!is_valid_semver("1.2.3.4"));
        assert!(!is_valid_semver("a.b.c"));
    }

    #[test]
    fn test_reserved_keywords() {
        assert!(is_reserved_keyword("scene"));
        assert!(is_reserved_keyword("entity"));
        assert!(!is_reserved_keyword("cube"));
        assert!(!is_reserved_keyword("my_entity"));
    }
}
