/// Reference resolution validation pass.
/// Ensures all entity and motion references are valid.
/// Detects undefined references and circular dependencies.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector, SourceSpan};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct ReferenceValidator {
    file: PathBuf,
    errors: ErrorCollector,
}

impl ReferenceValidator {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            errors: ErrorCollector::new(),
        }
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        // Build symbol tables
        let entities = Self::build_entity_table(&ast.entities);
        let motions = Self::build_motion_table(&ast.motions);
        let math_objects = Self::build_math_object_table(&ast.math_objects);

        // Validate constraint references
        self.validate_constraint_references(&ast.constraints, &entities);

        // Validate motion references
        self.validate_motion_references(&ast.motions, &entities);

        // Validate timeline references
        self.validate_timeline_references(&ast.timelines, &motions);

        // Validate math object references
        self.validate_math_object_references(&ast.math_objects, &math_objects);

        // Check for circular dependencies
        self.detect_circular_dependencies(&ast.constraints, &entities);

        self.errors.into_result(())
    }

    fn build_entity_table<'a>(entities: &'a [AstEntity]) -> HashMap<String, &'a AstEntity> {
        entities.iter().map(|e| (e.name.clone(), e)).collect()
    }

    fn build_motion_table<'a>(motions: &'a [AstMotion]) -> HashMap<String, &'a AstMotion> {
        motions.iter().map(|m| (m.name.clone(), m)).collect()
    }

    fn build_math_object_table(math_objects: &[MathObjectNode]) -> HashSet<String> {
        math_objects
            .iter()
            .map(|obj| match obj {
                MathObjectNode::Function(node) => node.name.clone(),
                MathObjectNode::Curve(node) => node.name.clone(),
                MathObjectNode::Surface(node) => node.name.clone(),
                MathObjectNode::VectorField(node) => node.name.clone(),
                MathObjectNode::ScalarField(node) => node.name.clone(),
                MathObjectNode::Transformation(node) => node.name.clone(),
                MathObjectNode::DifferentialEquation(node) => node.name.clone(),
                MathObjectNode::MatrixDefinition(node) => node.name.clone(),
            })
            .collect()
    }

    fn validate_constraint_references(
        &mut self,
        constraints: &[AstConstraint],
        entities: &HashMap<String, &AstEntity>,
    ) {
        for constraint in constraints {
            // Check all fields that reference entities
            for field in &constraint.fields {
                if let AstValue::Identifier(ref_name, span) = &field.value {
                    // Common entity reference field names
                    if matches!(
                        field.name.as_str(),
                        "driver" | "driven" | "parent" | "child" | "target" | "source"
                    ) {
                        if !entities.contains_key(ref_name) {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::UndefinedEntity,
                                    format!(
                                        "Undefined entity '{}' referenced in constraint '{}'",
                                        ref_name, constraint.name
                                    ),
                                    *span,
                                    self.file.clone(),
                                )
                                .with_help(format!(
                                    "Entity '{}' must be defined before it can be referenced",
                                    ref_name
                                )),
                            );
                        }
                    }
                }
            }
        }
    }

    fn validate_motion_references(
        &mut self,
        motions: &[AstMotion],
        entities: &HashMap<String, &AstEntity>,
    ) {
        for motion in motions {
            // Validate target entity exists
            if let Some(target) = motion.target() {
                if !entities.contains_key(target) {
                    let target_field = motion.get_field("target").unwrap();
                    self.errors.add(
                        DslError::new(
                            ErrorCode::UndefinedEntity,
                            format!(
                                "Undefined entity '{}' referenced in motion '{}'",
                                target, motion.name
                            ),
                            target_field.span,
                            self.file.clone(),
                        )
                        .with_help(format!(
                            "Entity '{}' must be defined before it can be used as a motion target",
                            target
                        )),
                    );
                }
            }
        }
    }

    fn validate_timeline_references(
        &mut self,
        timelines: &[AstTimeline],
        motions: &HashMap<String, &AstMotion>,
    ) {
        for timeline in timelines {
            // Track motion usage for overlap detection
            let mut motion_events: HashMap<&str, Vec<(f64, f64, SourceSpan)>> = HashMap::new();

            for event in &timeline.events {
                // Validate motion exists
                if let Some(motion_name) = event.motion() {
                    if !motions.contains_key(motion_name) {
                        let motion_field = event.get_field("motion").unwrap();
                        self.errors.add(
                            DslError::new(
                                ErrorCode::UndefinedMotion,
                                format!(
                                    "Undefined motion '{}' referenced in timeline '{}'",
                                    motion_name, timeline.name
                                ),
                                motion_field.span,
                                self.file.clone(),
                            )
                            .with_help(format!(
                                "Motion '{}' must be defined before it can be used in a timeline",
                                motion_name
                            )),
                        );
                    } else {
                        // Check for overlapping events with the same motion
                        if let (Some(start), Some(duration)) = (event.start(), event.duration()) {
                            let end = start + duration;

                            motion_events
                                .entry(motion_name)
                                .or_insert_with(Vec::new)
                                .push((start, end, event.span));
                        }
                    }
                }
            }

            // Detect overlapping events for the same motion
            for (motion_name, events) in motion_events {
                for i in 0..events.len() {
                    for j in (i + 1)..events.len() {
                        let (start1, end1, _span1) = events[i];
                        let (start2, end2, span2) = events[j];

                        // Check for overlap: [start1, end1) overlaps [start2, end2)
                        if start1 < end2 && start2 < end1 {
                            self.errors.add(
                                DslError::new(
                                    ErrorCode::InvalidBlockStructure,
                                    format!(
                                        "Overlapping events for motion '{}' in timeline '{}': [{}, {}) and [{}, {})",
                                        motion_name, timeline.name, start1, end1, start2, end2
                                    ),
                                    span2,
                                    self.file.clone(),
                                ).with_help("Events for the same motion cannot overlap in time".to_string())
                            );
                        }
                    }
                }
            }
        }
    }

    fn detect_circular_dependencies(
        &mut self,
        constraints: &[AstConstraint],
        entities: &HashMap<String, &AstEntity>,
    ) {
        // Build dependency graph for constraints
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        for constraint in constraints {
            // Extract driver/driven or parent/child relationships
            let driver = constraint
                .get_field("driver")
                .or_else(|| constraint.get_field("parent"))
                .and_then(|f| f.value.as_identifier());

            let driven = constraint
                .get_field("driven")
                .or_else(|| constraint.get_field("child"))
                .and_then(|f| f.value.as_identifier());

            if let (Some(from), Some(to)) = (driver, driven) {
                if entities.contains_key(from) && entities.contains_key(to) {
                    graph.entry(from).or_insert_with(Vec::new).push(to);
                }
            }
        }

        // Detect cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &node in graph.keys() {
            if !visited.contains(node) {
                if let Some(cycle) =
                    self.dfs_detect_cycle(node, &graph, &mut visited, &mut rec_stack, &mut path)
                {
                    // Find a constraint involved in the cycle for error reporting
                    let constraint = constraints.iter().find(|c| {
                        c.get_field("driver")
                            .or_else(|| c.get_field("parent"))
                            .and_then(|f| f.value.as_identifier())
                            .map(|d| cycle.iter().any(|s| s.as_str() == d))
                            .unwrap_or(false)
                    });

                    if let Some(constraint) = constraint {
                        self.errors.add(
                            DslError::new(
                                ErrorCode::CircularDependency,
                                format!("Circular dependency detected: {}", cycle.join(" -> ")),
                                constraint.span,
                                self.file.clone(),
                            )
                            .with_help(
                                "Remove or reorganize constraints to break the cycle".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    fn dfs_detect_cycle<'a>(
        &self,
        node: &'a str,
        graph: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
        path: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(neighbors) = graph.get(node) {
            for &neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) =
                        self.dfs_detect_cycle(neighbor, graph, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle - extract it from path
                    let cycle_start = path.iter().position(|&n| n == neighbor).unwrap();
                    let cycle: Vec<String> =
                        path[cycle_start..].iter().map(|s| s.to_string()).collect();
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    fn validate_math_object_references(
        &mut self,
        math_objects: &[MathObjectNode],
        known_objects: &HashSet<String>,
    ) {
        let builtin_fns: HashSet<&str> = [
            "sin", "cos", "tan", "asin", "acos", "atan", "exp", "ln", "log", "sqrt", "abs", "min",
            "max",
        ]
        .into_iter()
        .collect();

        for object in math_objects {
            match object {
                MathObjectNode::Function(node) => {
                    self.validate_expression_refs(
                        &node.body,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::Curve(node) => {
                    self.validate_expression_refs(
                        &node.definition,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::Surface(node) => {
                    self.validate_expression_refs(
                        &node.definition,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::VectorField(node) => {
                    for expr in &node.components {
                        self.validate_expression_refs(expr, known_objects, &builtin_fns, node.span);
                    }
                }
                MathObjectNode::ScalarField(node) => {
                    self.validate_expression_refs(
                        &node.expression,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::Transformation(node) => {
                    self.validate_expression_refs(
                        &node.expression,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::DifferentialEquation(node) => {
                    self.validate_expression_refs(
                        &node.equation,
                        known_objects,
                        &builtin_fns,
                        node.span,
                    );
                }
                MathObjectNode::MatrixDefinition(node) => {
                    for row in &node.elements {
                        for expr in row {
                            self.validate_expression_refs(
                                expr,
                                known_objects,
                                &builtin_fns,
                                node.span,
                            );
                        }
                    }
                }
            }
        }
    }

    fn validate_expression_refs(
        &mut self,
        expr: &MathExpression,
        known_objects: &HashSet<String>,
        builtins: &HashSet<&str>,
        span: SourceSpan,
    ) {
        match expr {
            MathExpression::FunctionCall(name, args) => {
                let is_known = builtins.contains(name.as_str()) || known_objects.contains(name);
                if !is_known {
                    self.errors.add(DslError::new(
                        ErrorCode::UndefinedEntity,
                        format!("Undefined math function/object reference '{}'", name),
                        span,
                        self.file.clone(),
                    ));
                }
                for arg in args {
                    self.validate_expression_refs(arg, known_objects, builtins, span);
                }
            }
            MathExpression::UnaryOp(_, inner) => {
                self.validate_expression_refs(inner, known_objects, builtins, span);
            }
            MathExpression::BinaryOp(lhs, _, rhs) => {
                self.validate_expression_refs(lhs, known_objects, builtins, span);
                self.validate_expression_refs(rhs, known_objects, builtins, span);
            }
            MathExpression::Derivative { expression, .. }
            | MathExpression::Integral { expression, .. }
            | MathExpression::Limit { expression, .. }
            | MathExpression::Summation { expression, .. }
            | MathExpression::Product { expression, .. } => {
                self.validate_expression_refs(expression, known_objects, builtins, span);
            }
            MathExpression::Piecewise(cases) => {
                for (_, value) in cases {
                    self.validate_expression_refs(value, known_objects, builtins, span);
                }
            }
            MathExpression::MatrixExpr(rows) => {
                for row in rows {
                    for value in row {
                        self.validate_expression_refs(value, known_objects, builtins, span);
                    }
                }
            }
            MathExpression::VectorExpr(values) => {
                for value in values {
                    self.validate_expression_refs(value, known_objects, builtins, span);
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
    use crate::errors::SourceSpan;

    fn make_entity(name: &str) -> AstEntity {
        AstEntity {
            name: name.to_string(),
            kind: "solid".to_string(),
            components: vec![],
            span: SourceSpan::single_point(1, 1, 0),
        }
    }

    fn make_motion(name: &str, target: &str) -> AstMotion {
        AstMotion {
            name: name.to_string(),
            fields: vec![AstField {
                name: "target".to_string(),
                value: AstValue::Identifier(target.to_string(), SourceSpan::single_point(1, 1, 0)),
                span: SourceSpan::single_point(1, 1, 0),
            }],
            span: SourceSpan::single_point(1, 1, 0),
        }
    }

    #[test]
    fn test_valid_references() {
        let entities = vec![make_entity("cube1"), make_entity("cube2")];
        let motions = vec![make_motion("spin", "cube1")];

        let entity_table = ReferenceValidator::build_entity_table(&entities);
        let motion_table = ReferenceValidator::build_motion_table(&motions);

        assert_eq!(entity_table.len(), 2);
        assert_eq!(motion_table.len(), 1);
    }

    #[test]
    fn test_undefined_entity_in_motion() {
        let entities = vec![make_entity("cube1")];
        let motions = vec![make_motion("spin", "nonexistent")];

        let ast = AstFile {
            scene: AstScene {
                name: "Test".to_string(),
                version: 1,
                ir_version: "0.1.0".to_string(),
                unit_system: "SI".to_string(),
                domain: None,
                span: SourceSpan::single_point(1, 1, 0),
            },
            library_imports: AstLibraryImports {
                imports: vec![],
                span: SourceSpan::single_point(1, 1, 0),
            },
            materials: vec![],
            fields: vec![],
            entities,
            constraints: vec![],
            motions,
            math_objects: vec![],
            compound_motions: vec![],
            trajectories: vec![],
            timelines: vec![],
            span: SourceSpan::single_point(1, 1, 0),
        };

        let validator = ReferenceValidator::new(PathBuf::from("test.dsl"));
        let result = validator.validate(&ast);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, ErrorCode::UndefinedEntity);
    }
}
