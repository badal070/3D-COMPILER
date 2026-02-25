//! LLM Orchestrator — orchestration layer between LLM and the system
//!
//! This crate provides typed representations of orchestrator messages, a small
//! mock client for testing, and helpers for converting orchestrator responses
//! into highlight token assignments that can be registered into the runtime's
//! `HighlightTokenRegistry` (the runtime crate owns the concrete registry).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-level intent produced by an LLM -> what animation or teaching action is
/// intended. This mirrors the "AnimationIntent" concepts in the architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationIntent {
    pub intent_id: String,
    pub description: Option<String>,
    /// Optional mapping from expression node ids to highlight tokens
    pub highlight_assignments: Vec<HighlightAssignment>,
}

/// A single assignment of a highlight token to an expression node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HighlightAssignment {
    /// AST node id (AnnotatedExpr.node_id)
    pub node_id: String,
    /// Token string, e.g. "hk_03"
    pub token: String,
    /// Optional display name for the token
    pub display_name: Option<String>,
    /// Optional color index (0-15). If None, caller should pick one.
    pub color_index: Option<u8>,
}

/// Full response object from the orchestrator (LLM pipeline)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrchestratorResponse {
    pub request_id: String,
    pub intent: AnimationIntent,
    /// Additional metadata (score, provenance)
    pub metadata: HashMap<String, String>,
}

/// Simplified trait that an orchestrator client should implement.
/// Implementations may call external LLM services; this trait is small for
/// testability and mocking.
pub trait OrchestratorClient {
    /// Send the given prompt/inputs and receive an `OrchestratorResponse`.
    fn request(&self, prompt: &str) -> Result<OrchestratorResponse, String>;
}

/// A trivial in-process mock orchestrator for unit tests and local development.
pub struct MockOrchestrator;

impl OrchestratorClient for MockOrchestrator {
    fn request(&self, prompt: &str) -> Result<OrchestratorResponse, String> {
        let resp = OrchestratorResponse {
            request_id: "mock_1".to_string(),
            intent: AnimationIntent {
                intent_id: "explain_step".to_string(),
                description: Some(format!("Mock response to prompt: {}", prompt)),
                highlight_assignments: vec![HighlightAssignment {
                    node_id: "node_root".to_string(),
                    token: "hk_01".to_string(),
                    display_name: Some("root expression".to_string()),
                    color_index: Some(3),
                }],
            },
            metadata: HashMap::new(),
        };
        Ok(resp)
    }
}

/// Convert an `OrchestratorResponse` into a compact vector of token registration
/// entries: (token, color_index, display_name). The runtime can use this to
/// populate its `HighlightTokenRegistry`.
pub fn response_to_token_registrations(
    resp: &OrchestratorResponse,
) -> Vec<(String, u8, String)> {
    let mut regs = Vec::new();
    // Simple allocation policy: prefer provided color_index, else hash token
    for a in &resp.intent.highlight_assignments {
        let color = a.color_index.unwrap_or_else(|| {
            // deterministic fallback: hash token string to 0..15
            let mut hash: u64 = 1469598103934665603u64;
            for b in a.token.as_bytes() {
                hash ^= *b as u64;
                hash = hash.wrapping_mul(1099511628211u64);
            }
            (hash % 16) as u8
        });
        let display = a
            .display_name
            .clone()
            .unwrap_or_else(|| format!("token {}", a.token));
        regs.push((a.token.clone(), color, display));
    }
    regs
}

// Optional HTTP client implementation
#[cfg(feature = "http")]
pub mod http_client {
    use super::*;
    use reqwest::blocking::Client;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    use std::collections::HashMap;

    /// Configuration for HTTP orchestrator client
    pub struct HttpClientConfig {
        pub endpoint: String,
        pub api_key: Option<String>,
        pub extra_headers: HashMap<String, String>,
    }

    /// Blocking HTTP client implementation of `OrchestratorClient`.
    ///
    /// Behavior: POSTs JSON { "prompt": "..." } to `endpoint` with
    /// `Authorization: Bearer <api_key>` header (if provided). Expects the
    /// response to be JSON matching `OrchestratorResponse`.
    pub struct HttpOrchestratorClient {
        client: Client,
        config: HttpClientConfig,
    }

    impl HttpOrchestratorClient {
        pub fn new(config: HttpClientConfig) -> Result<Self, String> {
            let client = Client::builder()
                .build()
                .map_err(|e| format!("failed to build reqwest client: {}", e))?;
            Ok(Self { client, config })
        }
    }

    impl OrchestratorClient for HttpOrchestratorClient {
        fn request(&self, prompt: &str) -> Result<OrchestratorResponse, String> {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            if let Some(key) = &self.config.api_key {
                let val = format!("Bearer {}", key);
                headers.insert(AUTHORIZATION, HeaderValue::from_str(&val).map_err(|e| e.to_string())?);
            }
            for (k, v) in &self.config.extra_headers {
                headers.insert(
                    HeaderName::from_bytes(k.as_bytes()).map_err(|e| e.to_string())?,
                    HeaderValue::from_str(v).map_err(|e| e.to_string())?,
                );
            }

            let body = serde_json::json!({ "prompt": prompt });

            let resp = self
                .client
                .post(&self.config.endpoint)
                .headers(headers)
                .json(&body)
                .send()
                .map_err(|e| format!("http request failed: {}", e))?;

            if !resp.status().is_success() {
                return Err(format!("orchestrator returned error status: {}", resp.status()));
            }

            let orch: OrchestratorResponse = resp.json().map_err(|e| format!("failed to parse response: {}", e))?;
            Ok(orch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_orchestrator_returns_response() {
        let client = MockOrchestrator;
        let resp = client.request("explain this expression").expect("ok");
        assert_eq!(resp.request_id, "mock_1");
        assert_eq!(resp.intent.highlight_assignments.len(), 1);
    }

    #[test]
    fn response_to_regs_prefers_color() {
        let resp = OrchestratorResponse {
            request_id: "r1".to_string(),
            intent: AnimationIntent {
                intent_id: "i1".to_string(),
                description: None,
                highlight_assignments: vec![HighlightAssignment {
                    node_id: "n1".to_string(),
                    token: "hk_05".to_string(),
                    display_name: Some("name".to_string()),
                    color_index: Some(7),
                }],
            },
            metadata: HashMap::new(),
        };

        let regs = response_to_token_registrations(&resp);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].1, 7);
    }

    #[test]
    fn response_to_regs_falls_back_to_hash() {
        let resp = OrchestratorResponse {
            request_id: "r2".to_string(),
            intent: AnimationIntent {
                intent_id: "i2".to_string(),
                description: None,
                highlight_assignments: vec![HighlightAssignment {
                    node_id: "n2".to_string(),
                    token: "hk_xx".to_string(),
                    display_name: None,
                    color_index: None,
                }],
            },
            metadata: HashMap::new(),
        };

        let regs = response_to_token_registrations(&resp);
        assert_eq!(regs.len(), 1);
        assert!(regs[0].1 < 16);
    }
}
