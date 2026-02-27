/// Error handling for the DSL compiler.
/// Every error must be actionable, precise, and machine-readable.
use std::fmt;
use std::path::PathBuf;

/// Source location span for precise error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl SourceSpan {
    pub fn new(
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
            start_offset,
            end_offset,
        }
    }

    pub fn single_point(line: usize, col: usize, offset: usize) -> Self {
        Self::new(line, col, line, col, offset, offset)
    }
}

/// Machine-readable error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Lexical errors (E001-E099)
    UnexpectedCharacter,
    UnterminatedString,
    InvalidNumber,
    InvalidIdentifier,

    // Syntax errors (E100-E199)
    UnexpectedToken,
    ExpectedToken,
    InvalidBlockStructure,
    MissingRequiredField,
    DuplicateField,
    InvalidSectionOrder,
    MalformedVector,

    // Schema errors (E200-E299)
    UnknownComponentType,
    InvalidFieldType,
    MissingRequiredComponent,
    DuplicateComponent,
    InvalidKind,
    InvalidUnitSystem,
    VersionMismatch,
    InvalidVersionFormat,
    InvalidMathDomain,
    DimensionMismatch,
    PotentialSingularity,

    // Reference errors (E300-E399)
    UndefinedEntity,
    UndefinedMotion,
    UndefinedConstraintType,
    DuplicateEntityName,
    DuplicateMotionName,
    DuplicateConstraintName,
    DuplicateTimelineName,
    CircularDependency,

    // Unit validation errors (E400-E499)
    InvalidVectorLength,
    PrecisionUnderspecification,
    InvalidMassValue,
    InvalidRotationUnit,
    NonNormalizedAxis,
    InvalidTimeValue,
    InvalidDurationValue,

    // Library compatibility errors (E500-E599)
    UndefinedLibrary,
    LibraryVersionMismatch,
    InvalidLibraryImport,
    UnknownLibraryConstruct,

    // Modeling errors (E1000-E1099)
    InvalidBooleanOperation,
    SketchNotClosed,
    ConstraintConflictDetected,
    DimensionBelowPrecisionThreshold,
    FeatureDependsOnDeletedGeometry,
}

impl ErrorCode {
    pub fn code(&self) -> u16 {
        match self {
            // Lexical
            ErrorCode::UnexpectedCharacter => 1,
            ErrorCode::UnterminatedString => 2,
            ErrorCode::InvalidNumber => 3,
            ErrorCode::InvalidIdentifier => 4,

            // Syntax
            ErrorCode::UnexpectedToken => 100,
            ErrorCode::ExpectedToken => 101,
            ErrorCode::InvalidBlockStructure => 102,
            ErrorCode::MissingRequiredField => 103,
            ErrorCode::DuplicateField => 104,
            ErrorCode::InvalidSectionOrder => 105,
            ErrorCode::MalformedVector => 106,

            // Schema
            ErrorCode::UnknownComponentType => 200,
            ErrorCode::InvalidFieldType => 201,
            ErrorCode::MissingRequiredComponent => 202,
            ErrorCode::DuplicateComponent => 203,
            ErrorCode::InvalidKind => 204,
            ErrorCode::InvalidUnitSystem => 205,
            ErrorCode::VersionMismatch => 206,
            ErrorCode::InvalidVersionFormat => 207,
            ErrorCode::InvalidMathDomain => 208,
            ErrorCode::DimensionMismatch => 209,
            ErrorCode::PotentialSingularity => 210,

            // Reference
            ErrorCode::UndefinedEntity => 300,
            ErrorCode::UndefinedMotion => 301,
            ErrorCode::UndefinedConstraintType => 302,
            ErrorCode::DuplicateEntityName => 303,
            ErrorCode::DuplicateMotionName => 304,
            ErrorCode::DuplicateConstraintName => 305,
            ErrorCode::DuplicateTimelineName => 306,
            ErrorCode::CircularDependency => 307,

            // Unit validation
            ErrorCode::InvalidVectorLength => 400,
            ErrorCode::PrecisionUnderspecification => 401,
            ErrorCode::InvalidMassValue => 402,
            ErrorCode::InvalidRotationUnit => 403,
            ErrorCode::NonNormalizedAxis => 404,
            ErrorCode::InvalidTimeValue => 405,
            ErrorCode::InvalidDurationValue => 406,

            // Library compatibility
            ErrorCode::UndefinedLibrary => 500,
            ErrorCode::LibraryVersionMismatch => 501,
            ErrorCode::InvalidLibraryImport => 502,
            ErrorCode::UnknownLibraryConstruct => 503,

            // Modeling
            ErrorCode::InvalidBooleanOperation => 1001,
            ErrorCode::SketchNotClosed => 1002,
            ErrorCode::ConstraintConflictDetected => 1003,
            ErrorCode::DimensionBelowPrecisionThreshold => 1004,
            ErrorCode::FeatureDependsOnDeletedGeometry => 1005,
        }
    }

    pub fn category(&self) -> &'static str {
        match self.code() {
            1..=99 => "Lexical Error",
            100..=199 => "Syntax Error",
            200..=299 => "Schema Error",
            300..=399 => "Reference Error",
            400..=499 => "Unit Validation Error",
            500..=599 => "Library Compatibility Error",
            1000..=1099 => "Modeling Error",
            _ => "Unknown Error",
        }
    }
}

/// Comprehensive error structure
#[derive(Debug, Clone)]
pub struct DslError {
    pub code: ErrorCode,
    pub message: String,
    pub span: SourceSpan,
    pub file: PathBuf,
    pub help: Option<String>,
}

impl DslError {
    pub fn new(code: ErrorCode, message: String, span: SourceSpan, file: PathBuf) -> Self {
        Self {
            code,
            message,
            span,
            file,
            help: None,
        }
    }

    pub fn with_help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    /// Format error for display with source context
    pub fn format_with_source(&self, source: &str) -> String {
        let mut output = String::new();

        // Header line
        output.push_str(&format!("E{:03}: {}\n", self.code.code(), self.message));

        // File location
        output.push_str(&format!(
            " --> {}:{}:{}\n",
            self.file.display(),
            self.span.start_line,
            self.span.start_col
        ));

        // Source context
        let lines: Vec<&str> = source.lines().collect();
        if self.span.start_line > 0 && self.span.start_line <= lines.len() {
            let line_num = self.span.start_line;
            let line_num_width = line_num.to_string().len();

            output.push_str(&format!("{:width$} |\n", "", width = line_num_width));
            output.push_str(&format!("{} | {}\n", line_num, lines[line_num - 1]));

            // Underline the error
            output.push_str(&format!("{:width$} | ", "", width = line_num_width));
            output.push_str(&" ".repeat(self.span.start_col - 1));

            let underline_len = if self.span.start_line == self.span.end_line {
                (self.span.end_col - self.span.start_col).max(1)
            } else {
                lines[line_num - 1].len() - self.span.start_col + 1
            };

            output.push_str(&"^".repeat(underline_len));
            output.push_str(&format!(" {}\n", self.code.category()));
        }

        // Help text
        if let Some(help) = &self.help {
            output.push_str(&format!("\nhelp: {}\n", help));
        }

        output
    }
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "E{:03}: {} ({}:{}:{})",
            self.code.code(),
            self.message,
            self.file.display(),
            self.span.start_line,
            self.span.start_col
        )
    }
}

impl std::error::Error for DslError {}

/// Result type for DSL operations
pub type DslResult<T> = Result<T, DslError>;

/// Collection of errors for batch reporting
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: Vec<DslError>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add(&mut self, error: DslError) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn into_result<T>(self, value: T) -> Result<T, Vec<DslError>> {
        if self.errors.is_empty() {
            Ok(value)
        } else {
            Err(self.errors)
        }
    }

    pub fn errors(&self) -> &[DslError] {
        &self.errors
    }

    pub fn take_errors(self) -> Vec<DslError> {
        self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_ranges() {
        assert_eq!(ErrorCode::UnexpectedCharacter.code(), 1);
        assert_eq!(ErrorCode::UnexpectedToken.code(), 100);
        assert_eq!(ErrorCode::UnknownComponentType.code(), 200);
        assert_eq!(ErrorCode::UndefinedEntity.code(), 300);
        assert_eq!(ErrorCode::InvalidVectorLength.code(), 400);
        assert_eq!(ErrorCode::PrecisionUnderspecification.code(), 401);
        assert_eq!(ErrorCode::UndefinedLibrary.code(), 500);
        assert_eq!(ErrorCode::InvalidBooleanOperation.code(), 1001);
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(ErrorCode::UnexpectedCharacter.category(), "Lexical Error");
        assert_eq!(ErrorCode::UnexpectedToken.category(), "Syntax Error");
        assert_eq!(ErrorCode::UnknownComponentType.category(), "Schema Error");
        assert_eq!(ErrorCode::UndefinedEntity.category(), "Reference Error");
        assert_eq!(
            ErrorCode::InvalidVectorLength.category(),
            "Unit Validation Error"
        );
        assert_eq!(
            ErrorCode::UndefinedLibrary.category(),
            "Library Compatibility Error"
        );
        assert_eq!(
            ErrorCode::InvalidBooleanOperation.category(),
            "Modeling Error"
        );
    }

    #[test]
    fn test_error_collector() {
        let mut collector = ErrorCollector::new();
        assert!(!collector.has_errors());

        let error = DslError::new(
            ErrorCode::UndefinedEntity,
            "Test error".to_string(),
            SourceSpan::single_point(1, 1, 0),
            PathBuf::from("test.dsl"),
        );

        collector.add(error);
        assert!(collector.has_errors());
        assert_eq!(collector.error_count(), 1);
    }
}
