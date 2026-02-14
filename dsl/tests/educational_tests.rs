use std::fs;
use std::path::Path;

#[test]
fn educational_docs_exist_and_non_empty() {
    let docs = [
        "docs/educational_guide.md",
        "docs/mathematical_dsl_reference.md",
        "docs/numerical_methods.md",
    ];

    for doc in docs {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(doc);
        assert!(path.exists(), "missing doc: {}", path.display());
        let content = fs::read_to_string(&path).expect("doc should be readable");
        assert!(
            !content.trim().is_empty(),
            "doc is empty: {}",
            path.display()
        );
    }
}

#[test]
fn educational_examples_exist_and_non_empty() {
    let examples = [
        "examples/calculus_examples.dsl",
        "examples/linear_algebra_examples.dsl",
        "examples/differential_equations.dsl",
    ];

    for example in examples {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(example);
        assert!(path.exists(), "missing example: {}", path.display());
        let content = fs::read_to_string(&path).expect("example should be readable");
        assert!(
            !content.trim().is_empty(),
            "example is empty: {}",
            path.display()
        );
    }
}
