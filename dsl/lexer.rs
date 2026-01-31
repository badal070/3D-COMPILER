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
    Colon,        // :
    Comma,        // ,

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

    // Special
    Eof,
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
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '"' => return self.scan_string(start_line, start_col, start_pos),
            '0'..='9' | '-' | '+' => return self.scan_number(start_line, start_col, start_pos),
            'a'..='z' | 'A'..='Z' | '_' => {
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
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
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
        let tokens = lex("{ } [ ] : ,");
        assert_eq!(
            tokens,
            vec![
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
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
}