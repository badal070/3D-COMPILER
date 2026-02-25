// compiler/runtime/step_trace/token.rs
// HighlightToken — color assignment and identity

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HighlightToken pairs a short string token with a color index
/// It is the shared identity that ties together:
/// - a span in the rendered KaTeX equation
/// - a span in the narrative prose
/// - a 3D object in the scene
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HighlightToken {
    /// Short alphanumeric identifier
    /// Format: "hk_" prefix + two-digit zero-padded index
    /// Example: "hk_03"
    /// Assigned by the LLMOrchestrator
    pub token: String,

    /// Index 0–15 into the ColorTable
    /// Same index used by:
    /// - Frontend CSS (for equation and prose spans)
    /// - Three.js backend (for emissive material color)
    pub color_index: u8,

    /// Human-readable name of what this token represents
    /// Examples: "outer function", "inner function", "exponent", "coefficient"
    /// Shown in "Why this step?" drawer
    pub display_name: String,
}

impl HighlightToken {
    /// Create a new highlight token
    pub fn new(token: String, color_index: u8, display_name: String) -> Self {
        Self {
            token,
            color_index,
            display_name,
        }
    }

    /// Get the token string
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Get the color index
    pub fn color_index(&self) -> u8 {
        self.color_index
    }

    /// Get the display name
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

/// Registry that maps token strings to HighlightToken definitions
/// Populated by LLMOrchestrator when processing a new OrchestratorResponse
/// Shared across the session and passed to snapshot builder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightTokenRegistry {
    /// Maps token string to HighlightToken
    tokens: HashMap<String, HighlightToken>,
}

impl HighlightTokenRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Add a token to the registry
    /// Returns a reference to the newly registered token
    pub fn register(
        &mut self,
        token: String,
        color_index: u8,
        display_name: String,
    ) -> &HighlightToken {
        let highlight_token = HighlightToken::new(token.clone(), color_index, display_name);
        self.tokens.insert(token.clone(), highlight_token);
        self.tokens.get(&token).unwrap()
    }

    /// Lookup a token by string identifier
    pub fn get(&self, token: &str) -> Option<&HighlightToken> {
        self.tokens.get(token)
    }

    /// Convenience lookup to get just the color index
    pub fn color_for(&self, token: &str) -> Option<u8> {
        self.get(token).map(|t| t.color_index)
    }

    /// Get all registered tokens
    pub fn all_tokens(&self) -> impl Iterator<Item = &HighlightToken> {
        self.tokens.values()
    }

    /// Check if a token is registered
    pub fn contains(&self, token: &str) -> bool {
        self.tokens.contains_key(token)
    }

    /// Get the number of registered tokens
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Clear all registered tokens
    /// Called when a new concept session begins
    pub fn clear(&mut self) {
        self.tokens.clear();
    }

    /// Get all token strings
    pub fn token_strings(&self) -> Vec<&str> {
        self.tokens.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for HighlightTokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_token_creation() {
        let token = HighlightToken::new(
            "hk_01".to_string(),
            5,
            "outer function".to_string(),
        );
        assert_eq!(token.token(), "hk_01");
        assert_eq!(token.color_index(), 5);
        assert_eq!(token.display_name(), "outer function");
    }

    #[test]
    fn test_registry_register() {
        let mut registry = HighlightTokenRegistry::new();
        registry.register("hk_01".to_string(), 3, "coefficient".to_string());

        assert!(registry.contains("hk_01"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_lookup() {
        let mut registry = HighlightTokenRegistry::new();
        registry.register("hk_02".to_string(), 7, "exponent".to_string());

        let token = registry.get("hk_02").unwrap();
        assert_eq!(token.color_index(), 7);
        assert_eq!(token.display_name(), "exponent");
    }

    #[test]
    fn test_registry_color_for() {
        let mut registry = HighlightTokenRegistry::new();
        registry.register("hk_03".to_string(), 11, "inner function".to_string());

        assert_eq!(registry.color_for("hk_03"), Some(11));
        assert_eq!(registry.color_for("hk_xx"), None);
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = HighlightTokenRegistry::new();
        registry.register("hk_01".to_string(), 1, "test".to_string());
        registry.register("hk_02".to_string(), 2, "test".to_string());

        assert_eq!(registry.len(), 2);
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_all_tokens() {
        let mut registry = HighlightTokenRegistry::new();
        registry.register("hk_01".to_string(), 1, "first".to_string());
        registry.register("hk_02".to_string(), 2, "second".to_string());

        let tokens: Vec<_> = registry.all_tokens().collect();
        assert_eq!(tokens.len(), 2);
    }
}
