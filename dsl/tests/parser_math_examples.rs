use dsl_compiler::Compiler;
use std::fs;
use std::path::PathBuf;

fn parse_example(path: &str) {
    let file_path = PathBuf::from(path);
    let source = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", file_path.display(), e));
    let compiler = Compiler::new();
    let parsed = compiler.parse_only(source, file_path.clone());
    assert!(
        parsed.is_ok(),
        "parse_only failed for {}: {:?}",
        file_path.display(),
        parsed.err()
    );
}

#[test]
fn parses_calculus_examples() {
    parse_example("../examples/calculus_examples.dsl");
}

#[test]
fn parses_linear_algebra_examples() {
    parse_example("../examples/linear_algebra_examples.dsl");
}

#[test]
fn parses_differential_equations_examples() {
    parse_example("../examples/differential_equations.dsl");
}
