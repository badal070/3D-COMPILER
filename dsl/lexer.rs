/// Lexical analyzer for the DSL.
/// Transforms source text into a stream of tokens.
/// No execution, no interpretation - pure tokenization.
use crate::errors::{DslError, DslResult, ErrorCode, SourceSpan};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Identifier(String),
    Number(f64),
    String(String),

    // Punctuation
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    LeftParen,    // (
    RightParen,   // )
    Equals,       // =
    Colon,        // :
    Comma,        // ,
    Pipe,         // |
    DoublePipe,   // || or ∥
    LeftAngle,    // ⟨
    RightAngle,   // ⟩

    // Keywords (reserved identifiers)
    Scene,
    LibraryImports,
    Entity,
    Constraint,
    Motion,
    Timeline,
    Event,
    Components,

    // New keywords
    Materials,
    Material,
    Fields,
    Field,
    CompoundMotion,
    Trajectory,
    ConceptRef,
    Annotations,
    Annotation,
    HighlightSchedule,
    At,
    AnchorKw,

    // Math-specific
    MathKeyword(MathKeyword),
    MathOperator(MathOperator),
    MathConstant(MathConstant),
    SetSymbol(SetSymbol),
    GreekLetter(String),
    LatexCommand(String),

    // Special
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathKeyword {
    Function,
    Curve,
    Surface,
    VectorField,
    ScalarField,
    Domain,
    Range,
    Limit,
    Derivative,
    Integral,
    Parameter,
    Implicit,
    Parametric,
    Polar,
    Complex,
    Matrix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathOperator {
    PartialDerivative, // ∂
    Integral,          // ∫
    Summation,         // ∑
    Product,           // ∏
    Gradient,          // ∇
    Cross,             // ×
    Dot,               // ·
    Plus,              // +
    Minus,             // -
    Multiply,          // *
    Divide,            // /
    Power,             // ^
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathConstant {
    Pi,        // π
    Euler,     // e
    Imaginary, // i
    Infinity,  // ∞
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetSymbol {
    In,        // ∈
    NotIn,     // ∉
    Subset,    // ⊂
    Union,     // ∪
    Intersect, // ∩
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

pub struct Lexer {
    #[allow(dead_code)]
    source: String,
    file: PathBuf,
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: String, file: PathBuf) -> Self {
        let chars: Vec<char> = source.chars().collect();
        Self {
            source,
            file,
            chars,
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> DslResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.is_eof() {
                let span = SourceSpan::single_point(self.line, self.column, self.position);
                tokens.push(Token::new(TokenKind::Eof, span));
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> DslResult<Token> {
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.position;

        let ch = self.current_char();

        let kind = match ch {
            '{' => {
                self.advance();
                TokenKind::LeftBrace
            }
            '}' => {
                self.advance();
                TokenKind::RightBrace
            }
            '[' => {
                self.advance();
                TokenKind::LeftBracket
            }
            ']' => {
                self.advance();
                TokenKind::RightBracket
            }
            '(' => {
                self.advance();
                TokenKind::LeftParen
            }
            ')' => {
                self.advance();
                TokenKind::RightParen
            }
            '=' => {
                self.advance();
                TokenKind::Equals
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '|' => {
                self.advance();
                if !self.is_eof() && self.current_char() == '|' {
                    self.advance();
                    TokenKind::DoublePipe
                } else {
                    TokenKind::Pipe
                }
            }
            '∥' => {
                self.advance();
                TokenKind::DoublePipe
            }
            '⟨' => {
                self.advance();
                TokenKind::LeftAngle
            }
            '⟩' => {
                self.advance();
                TokenKind::RightAngle
            }
            '∂' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::PartialDerivative)
            }
            '∫' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Integral)
            }
            '∑' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Summation)
            }
            '∏' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Product)
            }
            '∇' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Gradient)
            }
            '×' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Cross)
            }
            '·' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Dot)
            }
            '*' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Multiply)
            }
            '/' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Divide)
            }
            '^' => {
                self.advance();
                TokenKind::MathOperator(MathOperator::Power)
            }
            '∈' => {
                self.advance();
                TokenKind::SetSymbol(SetSymbol::In)
            }
            '∉' => {
                self.advance();
                TokenKind::SetSymbol(SetSymbol::NotIn)
            }
            '⊂' => {
                self.advance();
                TokenKind::SetSymbol(SetSymbol::Subset)
            }
            '∪' => {
                self.advance();
                TokenKind::SetSymbol(SetSymbol::Union)
            }
            '∩' => {
                self.advance();
                TokenKind::SetSymbol(SetSymbol::Intersect)
            }
            'π' => {
                self.advance();
                TokenKind::MathConstant(MathConstant::Pi)
            }
            '∞' => {
                self.advance();
                TokenKind::MathConstant(MathConstant::Infinity)
            }
            '\\' => return self.scan_latex_command(start_line, start_col, start_pos),
            '"' => return self.scan_string(start_line, start_col, start_pos),
            '0'..='9' => return self.scan_number(start_line, start_col, start_pos),
            '-' | '+' => {
                if self
                    .peek_char()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    return self.scan_number(start_line, start_col, start_pos);
                }
                let op = if ch == '+' {
                    MathOperator::Plus
                } else {
                    MathOperator::Minus
                };
                self.advance();
                TokenKind::MathOperator(op)
            }
            _ if Self::is_identifier_start(ch) => {
                return self.scan_identifier(start_line, start_col, start_pos)
            }
            _ => {
                return Err(DslError::new(
                    ErrorCode::UnexpectedCharacter,
                    format!("Unexpected character: '{}'", ch),
                    SourceSpan::single_point(self.line, self.column, self.position),
                    self.file.clone(),
                ))
            }
        };

        let span = SourceSpan::new(
            start_line,
            start_col,
            self.line,
            self.column,
            start_pos,
            self.position,
        );

        Ok(Token::new(kind, span))
    }

    fn scan_identifier(
        &mut self,
        start_line: usize,
        start_col: usize,
        start_pos: usize,
    ) -> DslResult<Token> {
        let mut ident = String::new();

        while !self.is_eof() {
            let ch = self.current_char();
            if Self::is_identifier_continue(ch) {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if ident.chars().count() == 1 {
            let ch = ident.chars().next().unwrap_or_default();
            if Self::is_greek_letter(ch) {
                let span = SourceSpan::new(
                    start_line,
                    start_col,
                    self.line,
                    self.column,
                    start_pos,
                    self.position,
                );
                return Ok(Token::new(TokenKind::GreekLetter(ident), span));
            }
        }

        let kind = match ident.as_str() {
            "scene" => TokenKind::Scene,
            "library_imports" => TokenKind::LibraryImports,
            "entity" => TokenKind::Entity,
            "constraint" => TokenKind::Constraint,
            "motion" => TokenKind::Motion,
            "timeline" => TokenKind::Timeline,
            "event" => TokenKind::Event,
            "components" => TokenKind::Components,
            "materials" => TokenKind::Materials,
            "material" => TokenKind::Material,
            "fields" => TokenKind::Fields,
            "field" => TokenKind::Field,
            "compound_motion" => TokenKind::CompoundMotion,
            "trajectory" => TokenKind::Trajectory,
            "concept_ref" => TokenKind::ConceptRef,
            "annotations" => TokenKind::Annotations,
            "annotation" => TokenKind::Annotation,
            "highlight_schedule" => TokenKind::HighlightSchedule,
            "at" => TokenKind::At,
            "anchor" => TokenKind::AnchorKw,
            "function" => TokenKind::MathKeyword(MathKeyword::Function),
            "curve" => TokenKind::MathKeyword(MathKeyword::Curve),
            "surface" => TokenKind::MathKeyword(MathKeyword::Surface),
            "vector_field" => TokenKind::MathKeyword(MathKeyword::VectorField),
            "scalar_field" => TokenKind::MathKeyword(MathKeyword::ScalarField),
            "domain" => TokenKind::MathKeyword(MathKeyword::Domain),
            "range" => TokenKind::MathKeyword(MathKeyword::Range),
            "limit" => TokenKind::MathKeyword(MathKeyword::Limit),
            "derivative" => TokenKind::MathKeyword(MathKeyword::Derivative),
            "integral" => TokenKind::MathKeyword(MathKeyword::Integral),
            "parameter" => TokenKind::MathKeyword(MathKeyword::Parameter),
            "implicit" => TokenKind::MathKeyword(MathKeyword::Implicit),
            "parametric" => TokenKind::MathKeyword(MathKeyword::Parametric),
            "polar" => TokenKind::MathKeyword(MathKeyword::Polar),
            "complex" => TokenKind::MathKeyword(MathKeyword::Complex),
            "matrix" => TokenKind::MathKeyword(MathKeyword::Matrix),
            "pi" => TokenKind::MathConstant(MathConstant::Pi),
            "e" => TokenKind::MathConstant(MathConstant::Euler),
            "i" => TokenKind::MathConstant(MathConstant::Imaginary),
            _ => TokenKind::Identifier(ident),
        };

        let span = SourceSpan::new(
            start_line,
            start_col,
            self.line,
            self.column,
            start_pos,
            self.position,
        );

        Ok(Token::new(kind, span))
    }

    fn scan_number(
        &mut self,
        start_line: usize,
        start_col: usize,
        start_pos: usize,
    ) -> DslResult<Token> {
        let mut num_str = String::new();

        // Optional sign
        if self.current_char() == '-' || self.current_char() == '+' {
            num_str.push(self.current_char());
            self.advance();
        }

        // Integer part
        while !self.is_eof() && self.current_char().is_ascii_digit() {
            num_str.push(self.current_char());
            self.advance();
        }

        // Decimal part
        if !self.is_eof() && self.current_char() == '.' {
            num_str.push('.');
            self.advance();

            while !self.is_eof() && self.current_char().is_ascii_digit() {
                num_str.push(self.current_char());
                self.advance();
            }
        }

        // Scientific notation
        if !self.is_eof() && (self.current_char() == 'e' || self.current_char() == 'E') {
            num_str.push(self.current_char());
            self.advance();

            if !self.is_eof() && (self.current_char() == '+' || self.current_char() == '-') {
                num_str.push(self.current_char());
                self.advance();
            }

            while !self.is_eof() && self.current_char().is_ascii_digit() {
                num_str.push(self.current_char());
                self.advance();
            }
        }

        let value = num_str.parse::<f64>().map_err(|_| {
            DslError::new(
                ErrorCode::InvalidNumber,
                format!("Invalid number format: '{}'", num_str),
                SourceSpan::new(
                    start_line,
                    start_col,
                    self.line,
                    self.column,
                    start_pos,
                    self.position,
                ),
                self.file.clone(),
            )
        })?;

        let span = SourceSpan::new(
            start_line,
            start_col,
            self.line,
            self.column,
            start_pos,
            self.position,
        );

        Ok(Token::new(TokenKind::Number(value), span))
    }

    fn scan_string(
        &mut self,
        start_line: usize,
        start_col: usize,
        start_pos: usize,
    ) -> DslResult<Token> {
        self.advance(); // Skip opening quote
        let mut string = String::new();

        while !self.is_eof() && self.current_char() != '"' {
            let ch = self.current_char();

            // Basic escape sequences
            if ch == '\\' {
                self.advance();
                if self.is_eof() {
                    break;
                }
                let escaped = self.current_char();
                match escaped {
                    'n' => string.push('\n'),
                    't' => string.push('\t'),
                    'r' => string.push('\r'),
                    '\\' => string.push('\\'),
                    '"' => string.push('"'),
                    _ => {
                        string.push('\\');
                        string.push(escaped);
                    }
                }
                self.advance();
            } else {
                string.push(ch);
                self.advance();
            }
        }

        if self.is_eof() || self.current_char() != '"' {
            return Err(DslError::new(
                ErrorCode::UnterminatedString,
                "Unterminated string literal".to_string(),
                SourceSpan::new(
                    start_line,
                    start_col,
                    self.line,
                    self.column,
                    start_pos,
                    self.position,
                ),
                self.file.clone(),
            ));
        }

        self.advance(); // Skip closing quote

        let span = SourceSpan::new(
            start_line,
            start_col,
            self.line,
            self.column,
            start_pos,
            self.position,
        );

        Ok(Token::new(TokenKind::String(string), span))
    }

    fn scan_latex_command(
        &mut self,
        start_line: usize,
        start_col: usize,
        start_pos: usize,
    ) -> DslResult<Token> {
        self.advance(); // Skip leading '\'
        let mut command = String::new();

        while !self.is_eof() && self.current_char().is_alphabetic() {
            command.push(self.current_char());
            self.advance();
        }

        if command.is_empty() {
            return Err(DslError::new(
                ErrorCode::UnexpectedCharacter,
                "Expected LaTeX command after '\\'".to_string(),
                SourceSpan::new(
                    start_line,
                    start_col,
                    self.line,
                    self.column,
                    start_pos,
                    self.position,
                ),
                self.file.clone(),
            ));
        }

        let span = SourceSpan::new(
            start_line,
            start_col,
            self.line,
            self.column,
            start_pos,
            self.position,
        );

        Ok(Token::new(TokenKind::LatexCommand(command), span))
    }

    fn is_identifier_start(ch: char) -> bool {
        ch == '_' || ch.is_alphabetic()
    }

    fn is_identifier_continue(ch: char) -> bool {
        ch == '_' || ch.is_alphanumeric()
    }

    fn is_greek_letter(ch: char) -> bool {
        ('\u{0370}'..='\u{03FF}').contains(&ch) || ('\u{1F00}'..='\u{1FFF}').contains(&ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_eof() {
            let ch = self.current_char();

            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek_char() == Some('/') {
                // Single-line comment
                while !self.is_eof() && self.current_char() != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn current_char(&self) -> char {
        self.chars[self.position]
    }

    fn peek_char(&self) -> Option<char> {
        if self.position + 1 < self.chars.len() {
            Some(self.chars[self.position + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.position < self.chars.len() {
            if self.chars[self.position] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.position >= self.chars.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source.to_string(), PathBuf::from("test.dsl"));
        lexer
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_basic_tokens() {
        let tokens = lex("{ } [ ] ( ) = : ,");
        assert_eq!(
            tokens,
            vec![
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::Equals,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("scene entity motion timeline");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Scene,
                TokenKind::Entity,
                TokenKind::Motion,
                TokenKind::Timeline,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_new_keywords() {
        let tokens = lex("materials material fields field compound_motion trajectory");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Materials,
                TokenKind::Material,
                TokenKind::Fields,
                TokenKind::Field,
                TokenKind::CompoundMotion,
                TokenKind::Trajectory,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex("cube1 gearA my_entity");
        assert!(matches!(tokens[0], TokenKind::Identifier(_)));
        assert!(matches!(tokens[1], TokenKind::Identifier(_)));
        assert!(matches!(tokens[2], TokenKind::Identifier(_)));
    }

    #[test]
    fn test_numbers() {
        let tokens = lex("42 3.14159 -1.0 2.5e-3");
        assert!(matches!(tokens[0], TokenKind::Number(42.0)));
        assert!(matches!(tokens[1], TokenKind::Number(_)));
        assert!(matches!(tokens[2], TokenKind::Number(-1.0)));
        assert!(matches!(tokens[3], TokenKind::Number(_)));
    }

    #[test]
    fn test_strings() {
        let tokens = lex(r#""Hello World" "test""#);
        assert!(matches!(tokens[0], TokenKind::String(_)));
        assert!(matches!(tokens[1], TokenKind::String(_)));
    }

    #[test]
    fn test_comments() {
        let tokens = lex("scene // this is a comment\nentity");
        assert_eq!(
            tokens,
            vec![TokenKind::Scene, TokenKind::Entity, TokenKind::Eof]
        );
    }

    #[test]
    fn test_math_operators_constants_and_set_symbols() {
        let tokens = lex("∂ ∫ ∑ ∏ ∇ × · π ∞ ∈ ∉ ⊂ ∪ ∩");
        assert_eq!(
            tokens,
            vec![
                TokenKind::MathOperator(MathOperator::PartialDerivative),
                TokenKind::MathOperator(MathOperator::Integral),
                TokenKind::MathOperator(MathOperator::Summation),
                TokenKind::MathOperator(MathOperator::Product),
                TokenKind::MathOperator(MathOperator::Gradient),
                TokenKind::MathOperator(MathOperator::Cross),
                TokenKind::MathOperator(MathOperator::Dot),
                TokenKind::MathConstant(MathConstant::Pi),
                TokenKind::MathConstant(MathConstant::Infinity),
                TokenKind::SetSymbol(SetSymbol::In),
                TokenKind::SetSymbol(SetSymbol::NotIn),
                TokenKind::SetSymbol(SetSymbol::Subset),
                TokenKind::SetSymbol(SetSymbol::Union),
                TokenKind::SetSymbol(SetSymbol::Intersect),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_math_keywords_and_constants() {
        let tokens = lex("function curve surface vector_field scalar_field domain range limit derivative integral parameter implicit parametric polar complex matrix pi e i");
        assert_eq!(
            tokens,
            vec![
                TokenKind::MathKeyword(MathKeyword::Function),
                TokenKind::MathKeyword(MathKeyword::Curve),
                TokenKind::MathKeyword(MathKeyword::Surface),
                TokenKind::MathKeyword(MathKeyword::VectorField),
                TokenKind::MathKeyword(MathKeyword::ScalarField),
                TokenKind::MathKeyword(MathKeyword::Domain),
                TokenKind::MathKeyword(MathKeyword::Range),
                TokenKind::MathKeyword(MathKeyword::Limit),
                TokenKind::MathKeyword(MathKeyword::Derivative),
                TokenKind::MathKeyword(MathKeyword::Integral),
                TokenKind::MathKeyword(MathKeyword::Parameter),
                TokenKind::MathKeyword(MathKeyword::Implicit),
                TokenKind::MathKeyword(MathKeyword::Parametric),
                TokenKind::MathKeyword(MathKeyword::Polar),
                TokenKind::MathKeyword(MathKeyword::Complex),
                TokenKind::MathKeyword(MathKeyword::Matrix),
                TokenKind::MathConstant(MathConstant::Pi),
                TokenKind::MathConstant(MathConstant::Euler),
                TokenKind::MathConstant(MathConstant::Imaginary),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_greek_letters_and_latex_commands() {
        let tokens = lex("α β γ \\frac \\sqrt");
        assert_eq!(
            tokens,
            vec![
                TokenKind::GreekLetter("α".to_string()),
                TokenKind::GreekLetter("β".to_string()),
                TokenKind::GreekLetter("γ".to_string()),
                TokenKind::LatexCommand("frac".to_string()),
                TokenKind::LatexCommand("sqrt".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_interval_notation_tokens() {
        let tokens = lex("[a,b) (x,y] |z| ⟨u,v⟩ ∥w∥");
        assert_eq!(
            tokens,
            vec![
                TokenKind::LeftBracket,
                TokenKind::Identifier("a".to_string()),
                TokenKind::Comma,
                TokenKind::Identifier("b".to_string()),
                TokenKind::RightParen,
                TokenKind::LeftParen,
                TokenKind::Identifier("x".to_string()),
                TokenKind::Comma,
                TokenKind::Identifier("y".to_string()),
                TokenKind::RightBracket,
                TokenKind::Pipe,
                TokenKind::Identifier("z".to_string()),
                TokenKind::Pipe,
                TokenKind::LeftAngle,
                TokenKind::Identifier("u".to_string()),
                TokenKind::Comma,
                TokenKind::Identifier("v".to_string()),
                TokenKind::RightAngle,
                TokenKind::DoublePipe,
                TokenKind::Identifier("w".to_string()),
                TokenKind::DoublePipe,
                TokenKind::Eof
            ]
        );
    }
}
