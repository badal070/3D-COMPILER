/// Parser for the DSL.
/// Transforms token stream into AST.
/// Enforces strict syntax rules and mandatory ordering.
use crate::ast::*;
use crate::errors::{DslError, DslResult, ErrorCode, SourceSpan};
use crate::lexer::{MathConstant, MathKeyword, MathOperator, Token, TokenKind};
use std::path::PathBuf;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    file: PathBuf,
    node_id_counter: usize,
}

#[derive(Debug, Clone, Copy)]
enum ListKind {
    Numeric,
    Other,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: PathBuf) -> Self {
        Self {
            tokens,
            position: 0,
            file,
            node_id_counter: 0,
        }
    }

    pub fn parse(&mut self) -> DslResult<AstFile> {
        if self.check(TokenKind::Scene) {
            return self.parse_scene_document();
        }
        self.parse_math_document()
    }

    fn parse_scene_document(&mut self) -> DslResult<AstFile> {
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
        let concept_ref = if self.check(TokenKind::ConceptRef) {
            Some(self.parse_concept_ref()?)
        } else {
            None
        };
        let annotations = if self.check(TokenKind::Annotations) {
            self.parse_annotations()?
        } else {
            Vec::new()
        };
        let highlight_schedule = if self.check(TokenKind::HighlightSchedule) {
            self.parse_highlight_schedule()?
        } else {
            Vec::new()
        };

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
            math_objects: Vec::new(),
            compound_motions,
            trajectories,
            timelines,
            concept_ref,
            annotations,
            highlight_schedule,
            span,
        })
    }

    fn parse_math_document(&mut self) -> DslResult<AstFile> {
        let start_span = self.current_span();

        while !self.check(TokenKind::Eof) {
            self.parse_math_statement()?;
        }
        self.expect(TokenKind::Eof)?;

        let end_span = self.previous_span();
        let span = self.span_between(start_span, end_span);

        Ok(AstFile {
            scene: AstScene {
                name: "Math Program".to_string(),
                version: 1,
                ir_version: "0.1.0".to_string(),
                unit_system: "SI".to_string(),
                domain: Some("math".to_string()),
                span,
            },
            library_imports: AstLibraryImports {
                imports: Vec::new(),
                span,
            },
            materials: Vec::new(),
            fields: Vec::new(),
            entities: Vec::new(),
            constraints: Vec::new(),
            motions: Vec::new(),
            math_objects: Vec::new(),
            compound_motions: Vec::new(),
            trajectories: Vec::new(),
            timelines: Vec::new(),
            concept_ref: None,
            annotations: Vec::new(),
            highlight_schedule: Vec::new(),
            span,
        })
    }

    fn parse_math_statement(&mut self) -> DslResult<()> {
        if self.check_math_identifier("ode")
            || self.check_math_identifier("ode_system")
            || self.check_math_identifier("visualize")
        {
            self.advance();
            self.expect(TokenKind::LeftBrace)?;
            self.consume_balanced_braces()?;
            return Ok(());
        }

        if self.is_math_top_level_starter() {
            self.advance(); // statement keyword

            // Optional identifier, e.g. function f(x), matrix A, transformation T
            if self.check_identifier_like() {
                self.advance();
            }

            // Optional signature params, e.g. f(x,y)
            if self.check(TokenKind::LeftParen) {
                self.consume_balanced_parentheses()?;
            }

            // Assignment form: ... = ...
            if self.check(TokenKind::Equals) {
                self.advance();
                self.consume_rhs_until_statement_boundary()?;
                return Ok(());
            }

            // Block form: ... { ... }
            if self.check(TokenKind::LeftBrace) {
                self.advance();
                self.consume_balanced_braces()?;
                return Ok(());
            }

            // Domain/metadata line forms such as: domain: ...
            if self.check(TokenKind::Colon) {
                self.advance();
                self.consume_rhs_until_statement_boundary()?;
                return Ok(());
            }

            return Ok(());
        }

        Err(self.error(
            ErrorCode::UnexpectedToken,
            format!(
                "Unexpected token in math document: {:?}",
                self.current().kind
            ),
        ))
    }

    fn consume_rhs_until_statement_boundary(&mut self) -> DslResult<()> {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        while !self.check(TokenKind::Eof) {
            if paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && self.is_math_statement_boundary()
            {
                break;
            }

            match self.current().kind {
                TokenKind::LeftParen => paren_depth += 1,
                TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LeftBracket => bracket_depth += 1,
                TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LeftBrace => brace_depth += 1,
                TokenKind::RightBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                }
                _ => {}
            }

            self.advance();
        }

        Ok(())
    }

    fn consume_balanced_parentheses(&mut self) -> DslResult<()> {
        self.expect(TokenKind::LeftParen)?;
        let mut depth = 1usize;
        while depth > 0 {
            if self.check(TokenKind::Eof) {
                return Err(
                    self.error(ErrorCode::UnexpectedToken, "Unterminated parenthesis group")
                );
            }
            match self.current().kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => depth -= 1,
                _ => {}
            }
            self.advance();
        }
        Ok(())
    }

    fn consume_balanced_braces(&mut self) -> DslResult<()> {
        let mut depth = 1usize;
        while depth > 0 {
            if self.check(TokenKind::Eof) {
                return Err(self.error(ErrorCode::UnexpectedToken, "Unterminated brace block"));
            }
            match self.current().kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => depth -= 1,
                _ => {}
            }
            self.advance();
        }
        Ok(())
    }

    fn is_math_statement_boundary(&self) -> bool {
        self.check(TokenKind::RightBrace) || self.is_math_top_level_starter()
    }

    fn check_identifier_like(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Identifier(_) | TokenKind::GreekLetter(_)
        )
    }

    fn check_math_identifier(&self, value: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(id) if id == value)
    }

    fn is_math_top_level_starter(&self) -> bool {
        match &self.current().kind {
            TokenKind::MathKeyword(MathKeyword::Function)
            | TokenKind::MathKeyword(MathKeyword::Curve)
            | TokenKind::MathKeyword(MathKeyword::Surface)
            | TokenKind::MathKeyword(MathKeyword::Matrix)
            | TokenKind::MathKeyword(MathKeyword::VectorField)
            | TokenKind::MathKeyword(MathKeyword::ScalarField) => true,
            TokenKind::Identifier(id)
                if matches!(
                    id.as_str(),
                    "function"
                        | "curve"
                        | "surface"
                        | "matrix"
                        | "ode"
                        | "ode_system"
                        | "visualize"
                        | "transformation"
                        | "vector_field"
                        | "scalar_field"
                ) =>
            {
                true
            }
            _ => false,
        }
    }

    fn parse_scene(&mut self) -> DslResult<AstScene> {
        let start_span = self.expect(TokenKind::Scene)?.span;
        self.expect(TokenKind::LeftBrace)?;

        let mut name = None;
        let mut version = None;
        let mut ir_version = None;
        let mut unit_system = None;
        let mut domain = None;

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
                        return Err(
                            self.error(ErrorCode::DuplicateField, "Duplicate 'version' field")
                        );
                    }
                    let num = self.expect_number()?;
                    version = Some(num as i64);
                }
                "ir_version" => {
                    if ir_version.is_some() {
                        return Err(
                            self.error(ErrorCode::DuplicateField, "Duplicate 'ir_version' field")
                        );
                    }
                    ir_version = Some(self.expect_string()?);
                }
                "unit_system" => {
                    if unit_system.is_some() {
                        return Err(
                            self.error(ErrorCode::DuplicateField, "Duplicate 'unit_system' field")
                        );
                    }
                    unit_system = Some(self.expect_string()?);
                }
                "domain" => {
                    if domain.is_some() {
                        return Err(
                            self.error(ErrorCode::DuplicateField, "Duplicate 'domain' field")
                        );
                    }
                    let value = match &self.current().kind {
                        TokenKind::Identifier(_) => self.expect_identifier()?,
                        TokenKind::String(_) => self.expect_string()?,
                        _ => {
                            return Err(self.error(
                                ErrorCode::InvalidFieldType,
                                "Scene 'domain' must be an identifier or string",
                            ))
                        }
                    };
                    domain = Some(value);
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

        let name = name
            .ok_or_else(|| self.error(ErrorCode::MissingRequiredField, "Missing 'name' field"))?;
        let version = version.ok_or_else(|| {
            self.error(ErrorCode::MissingRequiredField, "Missing 'version' field")
        })?;
        let ir_version = ir_version.ok_or_else(|| {
            self.error(
                ErrorCode::MissingRequiredField,
                "Missing 'ir_version' field",
            )
        })?;
        let unit_system = unit_system.ok_or_else(|| {
            self.error(
                ErrorCode::MissingRequiredField,
                "Missing 'unit_system' field",
            )
        })?;

        let span = self.span_between(start_span, end_span);

        Ok(AstScene {
            name,
            version,
            ir_version,
            unit_system,
            domain,
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
            TokenKind::String(s) => {
                let val = s.clone();
                let span = self.advance().span;
                Ok(AstValue::String(val, span))
            }
            TokenKind::LeftBracket => self.parse_list_value(),
            TokenKind::Identifier(id) if !self.is_math_expression_starting_here() => {
                if id == "true" || id == "false" {
                    let val = id == "true";
                    let span = self.advance().span;
                    return Ok(AstValue::Boolean(val, span));
                }
                let val = id.clone();
                let span = self.advance().span;
                Ok(AstValue::Identifier(val, span))
            }
            TokenKind::Number(n) if !self.is_math_expression_starting_here() => {
                let val = *n;
                let span = self.advance().span;
                Ok(AstValue::Number(val, span))
            }
            _ => {
                let start = self.current_span();
                let expr = self.parse_math_expression()?;
                let end = self.previous_span();
                Ok(AstValue::MathExpression(
                    expr,
                    self.span_between(start, end),
                ))
            }
        }
    }

    fn parse_list_value(&mut self) -> DslResult<AstValue> {
        match self.peek_list_kind()? {
            ListKind::Numeric => self.parse_vector(),
            ListKind::Other => self.parse_list(),
        }
    }

    fn parse_list(&mut self) -> DslResult<AstValue> {
        let start_span = self.expect(TokenKind::LeftBracket)?.span;
        let mut values = Vec::new();

        if self.check(TokenKind::RightBracket) {
            let end_span = self.expect(TokenKind::RightBracket)?.span;
            let span = self.span_between(start_span, end_span);
            return Ok(AstValue::List(values, span));
        }

        values.push(self.parse_value()?);

        while self.check(TokenKind::Comma) {
            self.advance();
            values.push(self.parse_value()?);
        }

        let end_span = self.expect(TokenKind::RightBracket)?.span;
        let span = self.span_between(start_span, end_span);

        Ok(AstValue::List(values, span))
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

    fn peek_list_kind(&self) -> DslResult<ListKind> {
        let mut idx = self.position;
        if !matches!(
            self.tokens.get(idx).map(|t| &t.kind),
            Some(TokenKind::LeftBracket)
        ) {
            return Err(self.error(ErrorCode::UnexpectedToken, "Expected '[' to start a list"));
        }
        idx += 1;

        if matches!(
            self.tokens.get(idx).map(|t| &t.kind),
            Some(TokenKind::RightBracket)
        ) {
            return Ok(ListKind::Other);
        }

        let mut numeric = true;
        loop {
            match self.tokens.get(idx).map(|t| &t.kind) {
                Some(TokenKind::Number(_)) => idx += 1,
                _ => {
                    numeric = false;
                    break;
                }
            }

            match self.tokens.get(idx).map(|t| &t.kind) {
                Some(TokenKind::Comma) => idx += 1,
                Some(TokenKind::RightBracket) => break,
                _ => {
                    numeric = false;
                    break;
                }
            }
        }

        Ok(if numeric {
            ListKind::Numeric
        } else {
            ListKind::Other
        })
    }

    fn parse_math_expression(&mut self) -> DslResult<AnnotatedExpr> {
        self.parse_additive_expression()
    }

    fn parse_additive_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let mut expr = self.parse_multiplicative_expression()?;

        loop {
            let op = if self.check_math_operator(MathOperator::Plus) {
                Some(MathBinaryOperator::Add)
            } else if self.check_math_operator(MathOperator::Minus) {
                Some(MathBinaryOperator::Subtract)
            } else {
                None
            };

            if let Some(op) = op {
                let op_span = self.advance().span;
                let rhs = self.parse_multiplicative_expression()?;
                expr = self.annotate_expr(
                    MathExpression::BinaryOp(Box::new(expr), op, Box::new(rhs)),
                    op_span,
                );
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_multiplicative_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let mut expr = self.parse_power_expression()?;

        loop {
            let op = if self.check_math_operator(MathOperator::Multiply) {
                Some(MathBinaryOperator::Multiply)
            } else if self.check_math_operator(MathOperator::Divide) {
                Some(MathBinaryOperator::Divide)
            } else {
                None
            };

            if let Some(op) = op {
                let op_span = self.advance().span;
                let rhs = self.parse_power_expression()?;
                expr = self.annotate_expr(
                    MathExpression::BinaryOp(Box::new(expr), op, Box::new(rhs)),
                    op_span,
                );
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_power_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let lhs = self.parse_unary_expression()?;
        if self.check_math_operator(MathOperator::Power) {
            let op_span = self.advance().span;
            let rhs = self.parse_power_expression()?;
            Ok(self.annotate_expr(
                MathExpression::BinaryOp(Box::new(lhs), MathBinaryOperator::Power, Box::new(rhs)),
                op_span,
            ))
        } else {
            Ok(lhs)
        }
    }

    fn parse_unary_expression(&mut self) -> DslResult<AnnotatedExpr> {
        if self.check_math_operator(MathOperator::Minus) {
            let op_span = self.advance().span;
            let expr = self.parse_unary_expression()?;
            return Ok(self.annotate_expr(
                MathExpression::UnaryOp(MathUnaryOperator::Negate, Box::new(expr)),
                op_span,
            ));
        }

        if self.check_math_operator(MathOperator::Gradient) {
            let op_span = self.advance().span;
            let expr = self.parse_unary_expression()?;
            return Ok(self.annotate_expr(
                MathExpression::UnaryOp(MathUnaryOperator::Gradient, Box::new(expr)),
                op_span,
            ));
        }

        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> DslResult<AnnotatedExpr> {
        match &self.current().kind {
            TokenKind::Number(n) => {
                let value = *n;
                let span = self.advance().span;
                Ok(self.annotate_expr(MathExpression::Number(value), span))
            }
            TokenKind::MathConstant(c) => {
                let constant = match c {
                    MathConstant::Pi => crate::ast::MathConstant::Pi,
                    MathConstant::Euler => crate::ast::MathConstant::Euler,
                    MathConstant::Imaginary => crate::ast::MathConstant::ImaginaryUnit,
                    MathConstant::Infinity => crate::ast::MathConstant::Infinity,
                };
                let span = self.advance().span;
                Ok(self.annotate_expr(MathExpression::Constant(constant), span))
            }
            TokenKind::Identifier(_) | TokenKind::GreekLetter(_) => {
                self.parse_identifier_expression()
            }
            TokenKind::MathKeyword(MathKeyword::Derivative) => self.parse_derivative_expression(),
            TokenKind::MathKeyword(MathKeyword::Integral)
            | TokenKind::MathOperator(MathOperator::Integral) => self.parse_integral_expression(),
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_math_expression()?;
                self.expect(TokenKind::RightParen)?;
                Ok(expr)
            }
            _ => Err(self.error(
                ErrorCode::UnexpectedToken,
                format!(
                    "Expected primary expression, found {:?}",
                    self.current().kind
                ),
            )),
        }
    }

    fn parse_identifier_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let start_span = self.current_span();
        let name = self.expect_identifier_or_greek()?;

        if self.check(TokenKind::LeftParen) {
            self.advance();
            let mut args = Vec::new();
            if !self.check(TokenKind::RightParen) {
                args.push(self.parse_math_expression()?);
                while self.check(TokenKind::Comma) {
                    self.advance();
                    args.push(self.parse_math_expression()?);
                }
            }
            self.expect(TokenKind::RightParen)?;
            Ok(self.annotate_expr(MathExpression::FunctionCall(name, args), start_span))
        } else {
            Ok(self.annotate_expr(MathExpression::Variable(name), start_span))
        }
    }

    fn parse_derivative_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let start_span = self.advance().span; // derivative keyword
        self.expect(TokenKind::LeftParen)?;
        let expression = self.parse_math_expression()?;
        self.expect(TokenKind::Comma)?;
        let variable = self.expect_identifier_or_greek()?;
        let mut order = 1usize;
        if self.check(TokenKind::Comma) {
            self.advance();
            order = self.expect_number()? as usize;
        }
        self.expect(TokenKind::RightParen)?;

        Ok(self.annotate_expr(
            MathExpression::Derivative {
                expression: Box::new(expression),
                variable,
                order,
            },
            start_span,
        ))
    }

    fn parse_integral_expression(&mut self) -> DslResult<AnnotatedExpr> {
        let start_span = self.advance().span; // integral keyword or ∫ symbol
        self.expect(TokenKind::LeftParen)?;
        let expression = self.parse_math_expression()?;
        self.expect(TokenKind::Comma)?;
        let variable = self.expect_identifier_or_greek()?;

        let bounds = if self.check(TokenKind::Comma) {
            self.advance();
            let lower = self.parse_math_expression()?;
            self.expect(TokenKind::Comma)?;
            let upper = self.parse_math_expression()?;
            Some(Box::new(IntervalConstraint {
                lower: Box::new(lower),
                upper: Box::new(upper),
                lower_inclusive: true,
                upper_inclusive: true,
            }))
        } else {
            None
        };

        self.expect(TokenKind::RightParen)?;
        Ok(self.annotate_expr(
            MathExpression::Integral {
                expression: Box::new(expression),
                variable,
                bounds,
            },
            start_span,
        ))
    }

    fn annotate_expr(&mut self, expr: MathExpression, span: SourceSpan) -> AnnotatedExpr {
        let node_id = format!(
            "{}:{}:{}",
            span.start_line, span.start_col, self.node_id_counter
        );
        self.node_id_counter += 1;
        AnnotatedExpr {
            node_id,
            highlight_token: None,
            expr,
        }
    }

    fn expect_identifier_or_greek(&mut self) -> DslResult<String> {
        match &self.current().kind {
            TokenKind::Identifier(id) => {
                let value = id.clone();
                self.advance();
                Ok(value)
            }
            TokenKind::GreekLetter(name) => {
                let value = name.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(self.error(
                ErrorCode::ExpectedToken,
                "Expected identifier or Greek letter",
            )),
        }
    }

    fn check_math_operator(&self, op: MathOperator) -> bool {
        matches!(&self.current().kind, TokenKind::MathOperator(found) if *found == op)
    }

    fn is_math_expression_starting_here(&self) -> bool {
        match &self.current().kind {
            TokenKind::LeftParen
            | TokenKind::MathConstant(_)
            | TokenKind::GreekLetter(_)
            | TokenKind::MathKeyword(MathKeyword::Derivative)
            | TokenKind::MathKeyword(MathKeyword::Integral)
            | TokenKind::MathOperator(MathOperator::Integral) => true,
            TokenKind::Number(_) | TokenKind::Identifier(_) => self
                .tokens
                .get(self.position + 1)
                .is_some_and(|next| match &next.kind {
                    TokenKind::LeftParen => true,
                    TokenKind::MathOperator(_) => true,
                    _ => false,
                }),
            _ => false,
        }
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

    fn parse_concept_ref(&mut self) -> DslResult<ConceptAnnotation> {
        self.expect(TokenKind::ConceptRef)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut concept_id: Option<String> = None;
        let mut section_id: Option<String> = None;
        let mut step_index: Option<usize> = None;

        while !self.check(TokenKind::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            match field_name.as_str() {
                "concept_id" => concept_id = Some(self.expect_string()?),
                "section_id" => section_id = Some(self.expect_string()?),
                "step_index" => step_index = Some(self.expect_number()? as usize),
                _ => {
                    return Err(self.error(
                        ErrorCode::InvalidBlockStructure,
                        format!("Unknown concept_ref field: '{}'", field_name),
                    ))
                }
            }
        }

        self.expect(TokenKind::RightBrace)?;

        Ok(ConceptAnnotation {
            concept_id: concept_id.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing concept_ref.concept_id",
                )
            })?,
            section_id: section_id.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing concept_ref.section_id",
                )
            })?,
            step_index: step_index.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing concept_ref.step_index",
                )
            })?,
        })
    }

    fn parse_annotations(&mut self) -> DslResult<Vec<AnnotationNode>> {
        self.expect(TokenKind::Annotations)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut annotations = Vec::new();
        while self.check(TokenKind::Annotation) {
            annotations.push(self.parse_single_annotation()?);
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(annotations)
    }

    fn parse_single_annotation(&mut self) -> DslResult<AnnotationNode> {
        let start_span = self.expect(TokenKind::Annotation)?.span;
        let named_label = self.expect_string()?;
        self.expect(TokenKind::LeftBrace)?;

        let mut anchor_entity_id: Option<String> = None;
        let mut position_offset = [0.0, 0.0, 0.0];
        let mut label_text: Option<String> = None;
        let mut equation_node_id: Option<String> = None;
        let mut highlight_token: Option<String> = None;

        while !self.check(TokenKind::RightBrace) {
            let field_name = self.expect_annotation_field_name()?;
            self.expect(TokenKind::Colon)?;
            match field_name.as_str() {
                "anchor" => anchor_entity_id = Some(self.expect_identifier()?),
                "offset" => position_offset = self.parse_vector3_literal()?,
                "label" => label_text = Some(self.expect_string()?),
                "equation_node_id" => equation_node_id = Some(self.expect_string()?),
                "highlight_token" => highlight_token = Some(self.expect_string()?),
                _ => {
                    return Err(self.error(
                        ErrorCode::InvalidBlockStructure,
                        format!("Unknown annotation field: '{}'", field_name),
                    ))
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(AnnotationNode {
            label_text: label_text.unwrap_or(named_label),
            anchor_entity_id: anchor_entity_id.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing annotation.anchor field",
                )
            })?,
            position_offset,
            equation_node_id,
            highlight_token,
            span: self.span_between(start_span, end_span),
        })
    }

    fn parse_highlight_schedule(&mut self) -> DslResult<Vec<HighlightScheduleEntry>> {
        self.expect(TokenKind::HighlightSchedule)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut entries = Vec::new();
        while self.check(TokenKind::At) {
            entries.push(self.parse_schedule_entry()?);
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(entries)
    }

    fn parse_schedule_entry(&mut self) -> DslResult<HighlightScheduleEntry> {
        let start_span = self.expect(TokenKind::At)?.span;
        let at_time = self.expect_number()?;
        self.expect(TokenKind::LeftBrace)?;

        let mut highlight_token: Option<String> = None;
        let mut entity_id: Option<String> = None;
        let mut color_index: Option<u8> = None;

        while !self.check(TokenKind::RightBrace) {
            let field_name = self.expect_identifier()?;
            self.expect(TokenKind::Colon)?;
            match field_name.as_str() {
                "token" => highlight_token = Some(self.expect_string()?),
                "entity" => entity_id = Some(self.expect_identifier()?),
                "color_index" => {
                    let value = self.expect_number()? as i64;
                    if !(0..=255).contains(&value) {
                        return Err(self.error(
                            ErrorCode::InvalidFieldType,
                            "highlight_schedule color_index must be in [0,255]",
                        ));
                    }
                    color_index = Some(value as u8);
                }
                _ => {
                    return Err(self.error(
                        ErrorCode::InvalidBlockStructure,
                        format!("Unknown highlight_schedule entry field: '{}'", field_name),
                    ))
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(HighlightScheduleEntry {
            at_time,
            highlight_token: highlight_token.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing highlight_schedule token field",
                )
            })?,
            entity_id: entity_id.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing highlight_schedule entity field",
                )
            })?,
            color_index: color_index.ok_or_else(|| {
                self.error(
                    ErrorCode::MissingRequiredField,
                    "Missing highlight_schedule color_index field",
                )
            })?,
            span: self.span_between(start_span, end_span),
        })
    }

    fn parse_vector3_literal(&mut self) -> DslResult<[f64; 3]> {
        self.expect(TokenKind::LeftBracket)?;
        let x = self.expect_number()?;
        self.expect(TokenKind::Comma)?;
        let y = self.expect_number()?;
        self.expect(TokenKind::Comma)?;
        let z = self.expect_number()?;
        self.expect(TokenKind::RightBracket)?;
        Ok([x, y, z])
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
            TokenKind::AnchorKw => {
                self.advance();
                Ok("anchor".to_string())
            }
            _ => Err(self.error(ErrorCode::ExpectedToken, "Expected identifier")),
        }
    }

    fn expect_annotation_field_name(&mut self) -> DslResult<String> {
        match &self.current().kind {
            TokenKind::AnchorKw => {
                self.advance();
                Ok("anchor".to_string())
            }
            _ => self.expect_identifier(),
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

    fn parse_expr(source: &str) -> DslResult<AnnotatedExpr> {
        let mut lexer = Lexer::new(source.to_string(), PathBuf::from("expr.dsl"));
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens, PathBuf::from("expr.dsl"));
        parser.parse_math_expression()
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

    #[test]
    fn test_expression_precedence() {
        let expr = parse_expr("1 + 2 * 3").unwrap();
        match expr.expr {
            MathExpression::BinaryOp(_, MathBinaryOperator::Add, rhs) => {
                assert!(matches!(
                    rhs.expr,
                    MathExpression::BinaryOp(_, MathBinaryOperator::Multiply, _)
                ));
            }
            _ => panic!("expected additive expression root"),
        }
    }

    #[test]
    fn test_function_call_expression() {
        let expr = parse_expr("sin(x)").unwrap();
        match expr.expr {
            MathExpression::FunctionCall(name, args) => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected function call"),
        }
    }

    #[test]
    fn test_derivative_expression() {
        let expr = parse_expr("derivative(x^2, x)").unwrap();
        assert!(matches!(expr.expr, MathExpression::Derivative { .. }));
    }

    #[test]
    fn test_integral_expression_with_bounds() {
        let expr = parse_expr("integral(x^2, x, 0, 1)").unwrap();
        match expr.expr {
            MathExpression::Integral { bounds, .. } => assert!(bounds.is_some()),
            _ => panic!("expected integral expression"),
        }
    }
}
