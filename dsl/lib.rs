/// DSL Compiler Library
///
/// A strict, declarative scene description language compiler.
/// Transforms DSL files into validated IR through ordered validation passes.
///
/// # Architecture
///
/// ```text
/// Source Text
///     ↓
/// Lexer (lexer.rs) → Tokens
///     ↓
/// Parser (parser.rs) → AST
///     ↓
/// Validator Pipeline:
///   1. Syntax Validation (validator/syntax.rs)
///   2. Schema Validation (validator/schema.rs)
///   3. Reference Resolution (validator/references.rs)
///   4. Unit Validation (validator/units.rs)
///   5. Library Compatibility (validator/library.rs)
///     ↓
/// IR Lowering (lower_to_ir.rs) → IR
/// ```
///
/// # Example Usage
///
/// ```rust,no_run
/// use dsl_compiler::Compiler;
/// use std::path::PathBuf;
///
/// let source = r#"
/// scene {
///   name: "Example"
///   version: 1
///   ir_version: "0.1.0"
///   unit_system: "SI"
/// }
///
/// library_imports {
///   math: "core_mechanics"
///   geometry: "basic_solids"
/// }
///
/// entity cube1 {
///   kind: solid
///   components {
///     transform {
///       position: [0, 0, 0]
///       rotation: [0, 0, 0]
///       scale: [1, 1, 1]
///     }
///   }
/// }
/// "#;
///
/// let file = PathBuf::from("example.dsl");
/// let compiler = Compiler::new();
/// let ir = compiler.compile(source, file).expect("Compilation failed");
/// ```
pub mod ast;
pub mod errors;
pub mod lexer;
pub mod lower_to_ir;
pub mod parser;

pub mod validator {
    pub mod compound_motion;
    pub mod field;
    pub mod geometry;
    pub mod library;
    pub mod material;
    pub mod math_semantic;
    pub mod references;
    pub mod schema;
    pub mod syntax;
    pub mod trajectory;
    pub mod units;
}

use crate::ast::AstFile;
use crate::errors::{DslError, DslResult};
use crate::lexer::Lexer;
use crate::lower_to_ir::{IrLowering, IrScene};
use crate::parser::Parser;
use crate::validator::compound_motion::CompoundMotionValidator;
use crate::validator::field::FieldValidator;
use crate::validator::geometry::GeometryValidator;
use crate::validator::library::LibraryValidator;
use crate::validator::material::MaterialValidator;
use crate::validator::math_semantic::MathSemanticValidator;
use crate::validator::references::ReferenceValidator;
use crate::validator::schema::SchemaValidator;
use crate::validator::syntax::SyntaxValidator;
use crate::validator::trajectory::TrajectoryValidator;
use crate::validator::units::{UnitSystem, UnitValidator};
use std::path::PathBuf;

/// Main compiler interface
pub struct Compiler {
    // Custom configuration options could go here
}

impl Compiler {
    /// Create a new compiler instance
    pub fn new() -> Self {
        Self {}
    }

    /// Compile a DSL source file to IR
    ///
    /// # Arguments
    ///
    /// * `source` - The DSL source code
    /// * `file` - Path to the source file (for error reporting)
    ///
    /// # Returns
    ///
    /// * `Ok(IrScene)` - Successfully compiled IR
    /// * `Err(Vec<DslError>)` - Compilation errors with detailed diagnostics
    pub fn compile(
        &self,
        source: impl Into<String>,
        file: PathBuf,
    ) -> Result<IrScene, Vec<DslError>> {
        let source = source.into();

        // 1. Lexical analysis
        let tokens = {
            let mut lexer = Lexer::new(source.clone(), file.clone());
            lexer.tokenize().map_err(|e| vec![e])?
        };

        // 2. Parsing
        let ast = {
            let mut parser = Parser::new(tokens, file.clone());
            parser.parse().map_err(|e| vec![e])?
        };

        // 3. Validation pipeline (ordered, fail-fast)
        self.validate(&ast, &file)?;

        // 4. Lower to IR
        let ir = IrLowering::lower(ast).map_err(|e| vec![e])?;

        Ok(ir)
    }

    /// Run the complete validation pipeline
    fn validate(&self, ast: &AstFile, file: &PathBuf) -> Result<(), Vec<DslError>> {
        // Pass 1: Syntax validation
        SyntaxValidator::new(file.clone()).validate(ast)?;

        // Pass 2: Schema validation
        SchemaValidator::new(file.clone()).validate(ast)?;

        // Pass 3: Reference resolution
        ReferenceValidator::new(file.clone()).validate(ast)?;

        // Pass 4: Unit validation
        let unit_system = UnitSystem::from_str(&ast.scene.unit_system).ok_or_else(|| {
            vec![DslError::new(
                errors::ErrorCode::InvalidUnitSystem,
                format!("Invalid unit system: '{}'", ast.scene.unit_system),
                ast.scene.span,
                file.clone(),
            )]
        })?;

        UnitValidator::new(file.clone(), unit_system, ast.scene.precision).validate(ast)?;

        // Pass 5: Library compatibility
        LibraryValidator::new(file.clone()).validate(ast)?;

        // Pass 6: Geometric integrity (NEW)
        GeometryValidator::new(file.clone()).validate(ast)?;

        // Pass 7: Material validation (NEW)
        let domain = ast
            .scene
            .domain
            .clone()
            .unwrap_or_else(|| "general".to_string());
        MaterialValidator::new(file.clone(), domain).validate(ast)?;

        // Pass 8: Field validation (NEW)
        FieldValidator::new(file.clone()).validate(ast)?;

        // Pass 9: Math semantic validation (NEW)
        MathSemanticValidator::new(file.clone()).validate(ast)?;

        // Pass 10: Compound motion validation (NEW)
        CompoundMotionValidator::new(file.clone()).validate(ast)?;

        // Pass 11: Trajectory validation (NEW)
        TrajectoryValidator::new(file.clone()).validate(ast)?;

        Ok(())
    }

    /// Parse source without validation (useful for tooling)
    pub fn parse_only(&self, source: impl Into<String>, file: PathBuf) -> DslResult<AstFile> {
        let source = source.into();

        let mut lexer = Lexer::new(source, file.clone());
        let tokens = lexer.tokenize()?;

        let mut parser = Parser::new(tokens, file);
        parser.parse()
    }

    /// Validate an AST (useful for testing validators separately)
    pub fn validate_only(&self, ast: &AstFile, file: &PathBuf) -> Result<(), Vec<DslError>> {
        self.validate(ast, file)
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to compile a DSL file
pub fn compile_file(path: impl Into<PathBuf>) -> Result<IrScene, Vec<DslError>> {
    let path = path.into();
    let source = std::fs::read_to_string(&path).map_err(|e| {
        vec![DslError::new(
            errors::ErrorCode::InvalidBlockStructure,
            format!("Failed to read file: {}", e),
            errors::SourceSpan::single_point(0, 0, 0),
            path.clone(),
        )]
    })?;

    let compiler = Compiler::new();
    compiler.compile(source, path)
}

/// Convenience function to compile DSL source code
pub fn compile_source(source: impl Into<String>) -> Result<IrScene, Vec<DslError>> {
    let compiler = Compiler::new();
    compiler.compile(source, PathBuf::from("input.dsl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_SCENE: &str = r#"
scene {
  name: "Test"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
  math: "core_mechanics"
  geometry: "basic_solids"
}
"#;

    const FULL_SCENE: &str = r#"
scene {
  name: "Rotating Cube"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
  math: "core_mechanics"
  geometry: "basic_solids"
}

entity cube1 {
  kind: solid
  components {
    transform {
      position: [0, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: cube
    }
    physical {
      mass: 1.0
      rigid: true
    }
  }
}

motion spin_cube {
  target: cube1
  type: rotation
  axis: [0, 1, 0]
  speed: 3.14159
}

timeline main {
  event {
    motion: spin_cube
    start: 0.0
    duration: 2.0
  }
}
"#;

    #[test]
    fn test_minimal_scene_compilation() {
        let result = compile_source(MINIMAL_SCENE);
        assert!(result.is_ok());
    }

    #[test]
    fn test_full_scene_compilation() {
        let result = compile_source(FULL_SCENE);
        assert!(result.is_ok());

        let ir = result.unwrap();
        assert_eq!(ir.metadata.name, "Rotating Cube");
        assert_eq!(ir.entities.len(), 1);
        assert_eq!(ir.motions.len(), 1);
        assert_eq!(ir.timelines.len(), 1);
    }

    #[test]
    fn test_invalid_syntax() {
        let source = r#"
scene {
  name: "Test"
  version: 1
}
"#; // Missing required fields

        let result = compile_source(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_only() {
        let compiler = Compiler::new();
        let result = compiler.parse_only(MINIMAL_SCENE, PathBuf::from("test.dsl"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_ir_json_output() {
        let result = compile_source(FULL_SCENE);
        assert!(result.is_ok());

        let ir = result.unwrap();
        let json = ir.to_json();

        assert!(json["metadata"]["name"].as_str().unwrap() == "Rotating Cube");
        assert!(json["entities"].as_array().unwrap().len() == 1);
    }
}
