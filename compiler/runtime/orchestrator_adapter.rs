// compiler/runtime/orchestrator_adapter.rs
// Adapter between the llm_orchestrator crate and the runtime's token registry

use llm_orchestrator::{OrchestratorClient, OrchestratorResponse};
use crate::step_trace::token::HighlightTokenRegistry;
use std::sync::{Arc, RwLock};

/// Simple adapter that can call an `OrchestratorClient` and register tokens
/// returned in an `OrchestratorResponse` into the runtime's
/// `HighlightTokenRegistry`.
pub struct OrchestratorAdapter {
    pub client: Arc<dyn OrchestratorClient + Send + Sync>,
    pub token_registry: Arc<RwLock<HighlightTokenRegistry>>,
}

impl OrchestratorAdapter {
    pub fn new(
        client: Arc<dyn OrchestratorClient + Send + Sync>,
        token_registry: Arc<RwLock<HighlightTokenRegistry>>,
    ) -> Self {
        Self { client, token_registry }
    }

    /// Call orchestrator with the given prompt and register any returned
    /// highlight tokens into the runtime's registry.
    pub fn request_and_register(&self, prompt: &str) -> Result<(), String> {
        let resp = self.client.request(prompt)?;
        self.register_response(&resp);
        Ok(())
    }

    /// Register tokens from a pre-obtained `OrchestratorResponse`.
    pub fn register_response(&self, resp: &OrchestratorResponse) {
        let regs = llm_orchestrator::response_to_token_registrations(resp);
        if let Ok(mut reg) = self.token_registry.write() {
            for (token, color, display) in regs {
                reg.register(token, color, display.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_orchestrator::MockOrchestrator;
    use crate::step_trace::token::HighlightTokenRegistry;
    use std::sync::Arc;

    #[test]
    fn adapter_registers_tokens() {
        let registry = Arc::new(RwLock::new(HighlightTokenRegistry::new()));
        let client = Arc::new(MockOrchestrator);
        let adapter = OrchestratorAdapter::new(client, Arc::clone(&registry));

        adapter.request_and_register("test prompt").expect("ok");

        let reg = registry.read().unwrap();
        assert!(reg.contains("hk_01"));
    }
}
