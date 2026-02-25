// compiler/semantic/concept_library.rs
// Concept & Curriculum System — minimal implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single teaching step inside a concept section
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    pub index: usize,
    pub title: String,
    pub description: String,
    pub highlight_token: Option<String>,
}

/// Named equation referenced by concepts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NamedEquation {
    pub id: String,
    pub latex: String,
    pub description: Option<String>,
}

/// A section inside a concept (e.g., "Derivatives: chain rule")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub steps: Vec<Step>,
    pub named_equations: Vec<NamedEquation>,
}

/// Top-level concept entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Concept {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub sections: Vec<Section>,
}

/// Simple in-memory concept library with lookup and loading
#[derive(Debug, Default)]
pub struct ConceptLibrary {
    concepts: HashMap<String, Concept>,
}

impl ConceptLibrary {
    /// Create a new empty library
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
        }
    }

    /// Register a concept (overwrites existing with same id)
    pub fn register(&mut self, concept: Concept) {
        self.concepts.insert(concept.id.clone(), concept);
    }

    /// Get a concept by id
    pub fn get(&self, id: &str) -> Option<&Concept> {
        self.concepts.get(id)
    }

    /// Find a named equation by id across all concepts
    pub fn find_named_equation(&self, eq_id: &str) -> Option<&NamedEquation> {
        for concept in self.concepts.values() {
            for section in &concept.sections {
                for eq in &section.named_equations {
                    if eq.id == eq_id {
                        return Some(eq);
                    }
                }
            }
        }
        None
    }

    /// Load concepts from a JSON string (array of Concept objects)
    pub fn load_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let list: Vec<Concept> = serde_json::from_str(json)?;
        for c in list {
            self.register(c);
        }
        Ok(())
    }

    /// Export entire library to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self.concepts.values().collect::<Vec<_>>())
    }

    /// Return concept ids
    pub fn ids(&self) -> Vec<String> {
        self.concepts.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_concept() -> Concept {
        Concept {
            id: "calc_derivative".to_string(),
            title: "Derivatives".to_string(),
            summary: Some("Basic derivative rules".to_string()),
            sections: vec![Section {
                id: "chain_rule".to_string(),
                title: "Chain Rule".to_string(),
                steps: vec![Step {
                    index: 0,
                    title: "Inner derivative".to_string(),
                    description: "Differentiate inner function".to_string(),
                    highlight_token: Some("hk_01".to_string()),
                }],
                named_equations: vec![NamedEquation {
                    id: "eq_chain".to_string(),
                    latex: "\\frac{d}{dx} f(g(x)) = f'(g(x)) g'(x)".to_string(),
                    description: Some("Chain rule".to_string()),
                }],
            }],
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut lib = ConceptLibrary::new();
        let c = sample_concept();
        lib.register(c.clone());
        let got = lib.get("calc_derivative").unwrap();
        assert_eq!(got.title, c.title);
    }

    #[test]
    fn test_find_named_equation() {
        let mut lib = ConceptLibrary::new();
        lib.register(sample_concept());
        let eq = lib.find_named_equation("eq_chain").unwrap();
        assert!(eq.latex.contains("f'(g(x))"));
    }

    #[test]
    fn test_load_from_json() {
        let mut lib = ConceptLibrary::new();
        let json = serde_json::to_string(&vec![sample_concept()]).unwrap();
        lib.load_from_json(&json).unwrap();
        assert!(lib.get("calc_derivative").is_some());
    }
}
