// compiler/runtime/step_trace/emitter.rs
// StepEmitter — pub-sub broadcaster

use super::event::MathStepEvent;
use super::trace::MathStepTrace;
use super::token::HighlightTokenRegistry;
use std::sync::{Arc, RwLock};

/// Trait for subscribers to step trace events
/// Implemented by components that want to receive events
pub trait StepSubscriber: Send + Sync {
    /// Called when a step event is emitted
    fn on_step(&mut self, event: &MathStepEvent, trace: &MathStepTrace);

    /// Called when a trace is completed
    fn on_trace_complete(&mut self, trace: &MathStepTrace);
}

/// The pub-sub broadcaster for step trace events
/// Collects events into MathStepTrace and notifies registered subscribers
/// Wrapped in Arc<Mutex<StepEmitter>> by RuntimeEngine for thread-safe access
pub struct StepEmitter {
    /// The trace currently being built
    /// Reset at the start of each new expression evaluation
    current_trace: MathStepTrace,

    /// Registered listeners
    /// SnapshotBuilder is the primary subscriber
    subscribers: Vec<Box<dyn StepSubscriber>>,

    /// Shared reference to session's token registry
    /// Used to look up color index for events with highlight_token
    token_registry: Arc<RwLock<HighlightTokenRegistry>>,
}

impl std::fmt::Debug for StepEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepEmitter")
            .field("current_trace", &self.current_trace)
            .field("subscriber_count", &self.subscribers.len())
            .finish()
    }
}

impl StepEmitter {
    /// Create a new step emitter
    pub fn new(token_registry: Arc<RwLock<HighlightTokenRegistry>>) -> Self {
        Self {
            current_trace: MathStepTrace::new(None),
            subscribers: Vec::new(),
            token_registry,
        }
    }

    /// Register a new subscriber
    pub fn subscribe(&mut self, subscriber: Box<dyn StepSubscriber>) {
        self.subscribers.push(subscriber);
    }

    /// Begin a new trace
    /// Called by RuntimeMathEngine before evaluating an expression
    /// Resets current_trace with a fresh MathStepTrace
    pub fn begin_trace(&mut self, equation_id: Option<String>) {
        self.current_trace = MathStepTrace::new(equation_id);
    }

    /// Emit a step event
    /// Called by RuntimeMathEngine after each node evaluation
    /// Pushes event to current_trace, then calls on_step() on all subscribers
    pub fn emit(&mut self, event: MathStepEvent) {
        self.current_trace.push_event(event.clone());

        // Notify all subscribers
        for subscriber in &mut self.subscribers {
            subscriber.on_step(&event, &self.current_trace);
        }
    }

    /// End the current trace
    /// Called by RuntimeMathEngine after the root expression returns
    /// Marks trace complete and calls on_trace_complete() on all subscribers
    pub fn end_trace(&mut self) {
        self.current_trace.mark_complete();

        // Notify all subscribers that trace is complete
        for subscriber in &mut self.subscribers {
            subscriber.on_trace_complete(&self.current_trace);
        }
    }

    /// Get read-only access to the in-progress or last-completed trace
    /// Used by RuntimeEngine to read active_highlight_token for state update
    pub fn current_trace(&self) -> &MathStepTrace {
        &self.current_trace
    }

    /// Get mutable access to the current trace
    /// Avoid using this; prefer current_trace() for most operations
    pub fn current_trace_mut(&mut self) -> &mut MathStepTrace {
        &mut self.current_trace
    }

    /// Take ownership of the completed trace and reset the emitter
    /// Called after a trace is fully consumed (e.g., sent to frontend)
    pub fn take_trace(&mut self) -> MathStepTrace {
        let trace = std::mem::replace(&mut self.current_trace, MathStepTrace::new(None));
        trace
    }

    /// Get the token registry
    pub fn token_registry(&self) -> Arc<RwLock<HighlightTokenRegistry>> {
        Arc::clone(&self.token_registry)
    }

    /// Get the number of registered subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Check if there are any subscribers
    pub fn has_subscribers(&self) -> bool {
        !self.subscribers.is_empty()
    }

    /// Get the current highlight token, if any
    /// Returns the highlight_token of the current event in the trace
    pub fn current_highlight_token(&self) -> Option<&str> {
        self.current_trace.active_highlight_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step_trace::event::StepOperation;
    use std::sync::Mutex as StdMutex;

    // Mock subscriber for testing
    struct MockSubscriber {
        step_count: StdMutex<usize>,
        trace_complete_count: StdMutex<usize>,
    }

    impl StepSubscriber for MockSubscriber {
        fn on_step(&mut self, _event: &MathStepEvent, _trace: &MathStepTrace) {
            let mut v = self.step_count.lock().unwrap();
            *v += 1;
        }

        fn on_trace_complete(&mut self, _trace: &MathStepTrace) {
            let mut v = self.trace_complete_count.lock().unwrap();
            *v += 1;
        }
    }

    fn create_test_emitter() -> StepEmitter {
        let registry = Arc::new(RwLock::new(HighlightTokenRegistry::new()));
        StepEmitter::new(registry)
    }

    #[test]
    fn test_emitter_creation() {
        let emitter = create_test_emitter();
        assert_eq!(emitter.subscriber_count(), 0);
        assert!(!emitter.has_subscribers());
    }

    #[test]
    fn test_begin_trace() {
        let mut emitter = create_test_emitter();
        emitter.begin_trace(Some("eq_1".to_string()));

        assert_eq!(emitter.current_trace().equation_id, Some("eq_1".to_string()));
        assert_eq!(emitter.current_trace().len(), 0);
    }

    #[test]
    fn test_emit_event() {
        let mut emitter = create_test_emitter();
        emitter.begin_trace(None);

        let event = MathStepEvent::new(Some("n1".to_string()), StepOperation::Add, 5.0, vec![]);
        emitter.emit(event);

        assert_eq!(emitter.current_trace().len(), 1);
    }

    #[test]
    fn test_end_trace() {
        let mut emitter = create_test_emitter();
        emitter.begin_trace(None);

        let event = MathStepEvent::new(None, StepOperation::Add, 5.0, vec![]);
        emitter.emit(event);

        emitter.end_trace();

        assert!(emitter.current_trace().is_complete);
    }

    #[test]
    fn test_take_trace() {
        let mut emitter = create_test_emitter();
        emitter.begin_trace(Some("eq_test".to_string()));

        let event = MathStepEvent::new(None, StepOperation::Add, 5.0, vec![]);
        emitter.emit(event);
        emitter.end_trace();

        let trace = emitter.take_trace();
        assert_eq!(trace.len(), 1);
        assert!(trace.is_complete);

        // New trace should be empty
        assert_eq!(emitter.current_trace().len(), 0);
    }

    #[test]
    fn test_subscription() {
        let mut emitter = create_test_emitter();

        let subscriber = Box::new(MockSubscriber {
            step_count: StdMutex::new(0),
            trace_complete_count: StdMutex::new(0),
        });

        emitter.subscribe(subscriber);
        assert_eq!(emitter.subscriber_count(), 1);
    }

    #[test]
    fn test_current_highlight_token() {
        let mut emitter = create_test_emitter();
        emitter.begin_trace(None);

        let event = MathStepEvent::new(None, StepOperation::Add, 5.0, vec![])
            .with_highlight("hk_01".to_string());

        emitter.emit(event);

        assert_eq!(emitter.current_highlight_token(), Some("hk_01"));
    }

    #[test]
    fn test_token_registry_access() {
        let registry = Arc::new(RwLock::new(HighlightTokenRegistry::new()));
        let emitter = StepEmitter::new(Arc::clone(&registry));

        {
            let mut reg = registry.write().unwrap();
            reg.register("hk_01".to_string(), 3, "test".to_string());
        }

        {
            let registry_arc = emitter.token_registry();
            let reg = registry_arc.read().unwrap();
            assert!(reg.contains("hk_01"));
        }
    }
}
