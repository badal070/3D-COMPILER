/// Parser for the DSL.
/// Transforms token stream into AST.
/// Enforces strict syntax rules and mandatory ordering.

use crate::ast::*;
use crate::errors::{DslError, DslResult, ErrorCode, SourceSpan};
use crate::lexer::{Token, TokenKind};
use std::path::PathBuf;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    file: PathBuf,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: PathBuf) -> Self {
        Self {
            tokens,
            position: 0,
            file,
        }
    }

    pub fn parse(&mut self) -> DslResult<AstFile> {
        let start_span = self.current_span();

        // Mandatory order: scene, library_imports, materials (opt), fields (opt), 
        // entities, constraints, motions, compound_motions (opt), trajectories (opt), timelines
        let scene = self.parse_scene()?;
        let library_imports = self.parse_library_imports()?;

        // New sections (optional)
        let materials = if self.check(TokenKind::Materials) {
            self.parse_materials()?
        } else {
            Vec::new()
        };

        let fields = if self.check(TokenKind::Fields) {
            self.parse_fields_section()?
        } else {
            Vec::new()
        };

        let entities = self.parse_entities()?;
        let constraints = self.parse_constraints()?;
        let motions = self.parse_motions()?;

        // New compound motions and trajectories
        let compound_motions = self.parse_compound_motions()?;
        let trajectories = self.parse_trajectories()?;

        let timelines = self.parse_timelines()?;

        self.expect(TokenKind::Eof)?;

        let end_span = self.previous_span();
        let span = SourceSpan::new(
            start_span.start_line,
            start_span.start_col,
            end_span.end_line,
            end_span.end_col,
            start_span.start_offset,
            end_span.end_offset,
        );

        Ok(AstFile {
            scene,
            library_imports,
            materials,
            fields,
            entities,
            constraints,
            motions,
            compound_motions,
            trajectories,
            timelines,
            span,
        })
    }

    fn parse_scene(&mut self) -> DslResult<AstScene> {
        let start_span = self.expect(TokenKind::Scene)?.span;
        self.expect(TokenKind::LeftBrace)?;

        let mut name = None;
        let mut version = None;
        let mut ir_version = None;
        let mut unit_system = None;

        while !self.check(TokenKind::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;

            match field_name.as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(self.error(ErrorCode::DuplicateField, "Duplicate 'name' field"));
                    }
                    name = Some(self.expect_string()?);
                }
                "version" => {
                    if version.is_some() {
                        return Err(self.error(ErrorCode::DuplicateField, "Duplicate 'version' field"));
                    }
                    let num = self.expect_number()?;
                    version = Some(num as i64);
                }
                "ir_version" => {
                    if ir_version.is_some() {
                        return Err(self.error(ErrorCode::DuplicateField, "Duplicate 'ir_version' field"));
                    }
                    ir_version = Some(self.expect_string()?);
                }
                "unit_system" => {
                    if unit_system.is_some() {
                        return Err(self.error(ErrorCode::DuplicateField, "Duplicate 'unit_system' field"));
                    }
                    unit_system = Some(self.expect_string()?);
                }
                _ => {
                    return Err(self.error(
                        ErrorCode::InvalidBlockStructure,
                        format!("Unknown scene field: '{}'", field_name),
                    ));
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        let name = name.ok_or_else(|| self.error(ErrorCode::MissingRequiredField, "Missing 'name' field"))?;
        let version = version.ok_or_else(|| self.error(ErrorCode::MissingRequiredField, "Missing 'version' field"))?;
        let ir_version = ir_version.ok_or_else(|| self.error(ErrorCode::MissingRequiredField, "Missing 'ir_version' field"))?;
        let unit_system = unit_system.ok_or_else(|| self.error(ErrorCode::MissingRequiredField, "Missing 'unit_system' field"))?;

        let span = self.span_between(start_span, end_span);

        Ok(AstScene {
            name,
            version,
            ir_version,
            unit_system,
            span,
        })
    }

    fn parse_library_imports(&mut self) -> DslResult<AstLibraryImports> {
        let start_span = self.expect(TokenKind::LibraryImports)?.span;
        self.expect(TokenKind::LeftBrace)?;

        let mut imports = Vec::new();

        while !self.check(TokenKind::RightBrace) {
            let import_start = self.current_span();
            let alias = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            let library_name = self.expect_string()?;
            let import_end = self.previous_span();

            imports.push(AstImport {
                alias,
                library_name,
                span: self.span_between(import_start, import_end),
            });
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstLibraryImports { imports, span })
    }

    fn parse_materials(&mut self) -> DslResult<Vec<AstMaterial>> {
        self.expect(TokenKind::Materials)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut materials = Vec::new();

        while self.check(TokenKind::Material) {
            materials.push(self.parse_material()?);
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(materials)
    }

    fn parse_material(&mut self) -> DslResult<AstMaterial> {
        let start_span = self.expect(TokenKind::Material)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstMaterial { name, fields, span })
    }

    fn parse_fields_section(&mut self) -> DslResult<Vec<AstFieldDef>> {
        self.expect(TokenKind::Fields)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();

        while self.check(TokenKind::Field) {
            fields.push(self.parse_field_def()?);
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(fields)
    }

    fn parse_field_def(&mut self) -> DslResult<AstFieldDef> {
        let start_span = self.expect(TokenKind::Field)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstFieldDef { name, fields, span })
    }

    fn parse_entities(&mut self) -> DslResult<Vec<AstEntity>> {
        let mut entities = Vec::new();

        while self.check(TokenKind::Entity) {
            entities.push(self.parse_entity()?);
        }

        Ok(entities)
    }

    fn parse_entity(&mut self) -> DslResult<AstEntity> {
        let start_span = self.expect(TokenKind::Entity)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        // Expect "kind: <identifier>"
        self.expect_field_name("kind")?;
        self.expect(TokenKind::Colon)?;
        let kind = self.expect_identifier()?;

        // Expect "components { ... }"
        self.expect(TokenKind::Components)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut components = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            components.push(self.parse_component()?);
        }

        self.expect(TokenKind::RightBrace)?; // Close components
        let end_span = self.expect(TokenKind::RightBrace)?.span; // Close entity

        let span = self.span_between(start_span, end_span);

        Ok(AstEntity {
            name,
            kind,
            components,
            span,
        })
    }

    fn parse_component(&mut self) -> DslResult<AstComponent> {
        let start_span = self.current_span();
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstComponent { name, fields, span })
    }

    fn parse_fields(&mut self) -> DslResult<Vec<AstField>> {
        let mut fields = Vec::new();

        while !self.check(TokenKind::RightBrace) {
            let start_span = self.current_span();
            // Accept both identifiers and keywords that can appear as field names
            let name = match &self.current().kind {
                TokenKind::Identifier(id) => {
                    let val = id.clone();
                    self.advance();
                    val
                }
                TokenKind::Motion => {
                    self.advance();
                    "motion".to_string()
                }
                _ => self.expect_identifier()?,
            };
            self.expect(TokenKind::Colon)?;
            let value = self.parse_value()?;
            let end_span = self.previous_span();

            fields.push(AstField {
                name,
                value,
                span: self.span_between(start_span, end_span),
            });
        }

        Ok(fields)
    }

    fn parse_value(&mut self) -> DslResult<AstValue> {
        match &self.current().kind {
            TokenKind::Number(n) => {
                let val = *n;
                let span = self.advance().span;
                Ok(AstValue::Number(val, span))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                let span = self.advance().span;
                Ok(AstValue::String(val, span))
            }
            TokenKind::Identifier(id) => {
                let val = id.clone();
                let span = self.advance().span;
                Ok(AstValue::Identifier(val, span))
            }
            TokenKind::LeftBracket => self.parse_vector(),
            _ => Err(self.error(ErrorCode::UnexpectedToken, "Expected value")),
        }
    }

    fn parse_vector(&mut self) -> DslResult<AstValue> {
        let start_span = self.expect(TokenKind::LeftBracket)?.span;
        let mut values = Vec::new();

        values.push(self.expect_number()?);

        while self.check(TokenKind::Comma) {
            self.advance();
            values.push(self.expect_number()?);
        }

        let end_span = self.expect(TokenKind::RightBracket)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstValue::Vector(values, span))
    }

    fn parse_constraints(&mut self) -> DslResult<Vec<AstConstraint>> {
        let mut constraints = Vec::new();

        while self.check(TokenKind::Constraint) {
            constraints.push(self.parse_constraint()?);
        }

        Ok(constraints)
    }

    fn parse_constraint(&mut self) -> DslResult<AstConstraint> {
        let start_span = self.expect(TokenKind::Constraint)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstConstraint { name, fields, span })
    }

    fn parse_motions(&mut self) -> DslResult<Vec<AstMotion>> {
        let mut motions = Vec::new();

        while self.check(TokenKind::Motion) {
            motions.push(self.parse_motion()?);
        }

        Ok(motions)
    }

    fn parse_motion(&mut self) -> DslResult<AstMotion> {
        let start_span = self.expect(TokenKind::Motion)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstMotion { name, fields, span })
    }

    fn parse_compound_motions(&mut self) -> DslResult<Vec<AstCompoundMotion>> {
        let mut compound_motions = Vec::new();

        while self.check(TokenKind::CompoundMotion) {
            compound_motions.push(self.parse_compound_motion()?);
        }

        Ok(compound_motions)
    }

    fn parse_compound_motion(&mut self) -> DslResult<AstCompoundMotion> {
        let start_span = self.expect(TokenKind::CompoundMotion)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstCompoundMotion { name, fields, span })
    }

    fn parse_trajectories(&mut self) -> DslResult<Vec<AstTrajectory>> {
        let mut trajectories = Vec::new();

        while self.check(TokenKind::Trajectory) {
            trajectories.push(self.parse_trajectory()?);
        }

        Ok(trajectories)
    }

    fn parse_trajectory(&mut self) -> DslResult<AstTrajectory> {
        let start_span = self.expect(TokenKind::Trajectory)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstTrajectory { name, fields, span })
    }

    fn parse_timelines(&mut self) -> DslResult<Vec<AstTimeline>> {
        let mut timelines = Vec::new();

        while self.check(TokenKind::Timeline) {
            timelines.push(self.parse_timeline()?);
        }

        Ok(timelines)
    }

    fn parse_timeline(&mut self) -> DslResult<AstTimeline> {
        let start_span = self.expect(TokenKind::Timeline)?.span;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LeftBrace)?;

        let mut events = Vec::new();

        while self.check(TokenKind::Event) {
            events.push(self.parse_event()?);
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstTimeline { name, events, span })
    }

    fn parse_event(&mut self) -> DslResult<AstEvent> {
        let start_span = self.expect(TokenKind::Event)?.span;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_fields()?;

        let end_span = self.expect(TokenKind::RightBrace)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstEvent { fields, span })
    }

    // Helper methods

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn current_span(&self) -> SourceSpan {
        self.current().span
    }

    fn previous_span(&self) -> SourceSpan {
        self.tokens[self.position - 1].span
    }

    fn check(&self, kind: TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(&kind)
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.position];
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
        token
    }

    fn expect(&mut self, kind: TokenKind) -> DslResult<&Token> {
        if self.check(kind.clone()) {
            Ok(self.advance())
        } else {
            Err(self.error(
                ErrorCode::ExpectedToken,
                format!("Expected {:?}, found {:?}", kind, self.current().kind),
            ))
        }
    }

    fn expect_identifier(&mut self) -> DslResult<String> {
        match &self.current().kind {
            TokenKind::Identifier(id) => {
                let val = id.clone();
                self.advance();
                Ok(val)
            }
            _ => Err(self.error(ErrorCode::ExpectedToken, "Expected identifier")),
        }
    }

    fn expect_string(&mut self) -> DslResult<String> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let val = s.clone();
                self.advance();
                Ok(val)
            }
            _ => Err(self.error(ErrorCode::ExpectedToken, "Expected string")),
        }
    }

    fn expect_number(&mut self) -> DslResult<f64> {
        match &self.current().kind {
            TokenKind::Number(n) => {
                let val = *n;
                self.advance();
                Ok(val)
            }
            _ => Err(self.error(ErrorCode::ExpectedToken, "Expected number")),
        }
    }

    fn expect_field_name(&mut self, expected: &str) -> DslResult<()> {
        let name = self.expect_identifier()?;
        if name != expected {
            return Err(self.error(
                ErrorCode::ExpectedToken,
                format!("Expected field '{}', found '{}'", expected, name),
            ));
        }
        Ok(())
    }

    fn span_between(&self, start: SourceSpan, end: SourceSpan) -> SourceSpan {
        SourceSpan::new(
            start.start_line,
            start.start_col,
            end.end_line,
            end.end_col,
            start.start_offset,
            end.end_offset,
        )
    }

    fn error(&self, code: ErrorCode, message: impl Into<String>) -> DslError {
        DslError::new(code, message.into(), self.current_span(), self.file.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> DslResult<AstFile> {
        let mut lexer = Lexer::new(source.to_string(), PathBuf::from("test.dsl"));
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens, PathBuf::from("test.dsl"));
        parser.parse()
    }

    #[test]
    fn test_minimal_scene() {
        let source = r#"
scene {
  name: "Test"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
}
"#;
        let ast = parse(source).unwrap();
        assert_eq!(ast.scene.name, "Test");
        assert_eq!(ast.scene.version, 1);
    }

    #[test]
    fn test_entity_parsing() {
        let source = r#"
scene {
  name: "Test"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
}

entity cube1 {
  kind: solid
  components {
    transform {
      position: [0, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
  }
}
"#;
        let ast = parse(source).unwrap();
        assert_eq!(ast.entities.len(), 1);
        assert_eq!(ast.entities[0].name, "cube1");
        assert_eq!(ast.entities[0].kind, "solid");
    }
}