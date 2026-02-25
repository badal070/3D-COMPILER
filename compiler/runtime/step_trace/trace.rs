// compiler/runtime/step_trace/trace.rs
// MathStepTrace — ordered log of events, navigation

use super::event::MathStepEvent;
use serde::{Deserialize, Serialize};

/// Ordered, navigable log of all MathStepEvent objects emitted during
/// one complete evaluation of a MathExpression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathStepTrace {
    /// All events in emission order
    /// Index 0 is the first sub-expression (deepest leaf)
    /// Last index is the root expression's result
    pub events: Vec<MathStepEvent>,

    /// Index of the currently active/displayed step
    /// Starts at 0 and advances as user presses "Next Step"
    pub current_index: usize,

    /// The NamedEquation ID from concept_library module, if applicable
    /// None for ad-hoc evaluations
    pub equation_id: Option<String>,

    /// Cached event count for quick access
    pub total_steps: usize,

    /// True once root expression evaluated and all events logged
    pub is_complete: bool,
}

impl MathStepTrace {
    /// Create a new empty trace
    pub fn new(equation_id: Option<String>) -> Self {
        Self {
            events: Vec::new(),
            current_index: 0,
            equation_id,
            total_steps: 0,
            is_complete: false,
        }
    }

    /// Advance current_index forward by 1
    /// Returns the now-active event, or None if already at end
    pub fn advance(&mut self) -> Option<&MathStepEvent> {
        if self.current_index < self.events.len() - 1 {
            self.current_index += 1;
            self.current()
        } else {
            None
        }
    }

    /// Move current_index backward by 1
    /// Returns the now-active event, or None if already at start
    pub fn rewind(&mut self) -> Option<&MathStepEvent> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current()
        } else {
            None
        }
    }

    /// Seek to a specific index
    /// Used by scrubber control
    /// Returns the event at that index, or None if out of bounds
    pub fn jump_to(&mut self, index: usize) -> Option<&MathStepEvent> {
        if index < self.events.len() {
            self.current_index = index;
            self.current()
        } else {
            None
        }
    }

    /// Return the event at current_index
    pub fn current(&self) -> Option<&MathStepEvent> {
        self.events.get(self.current_index)
    }

    /// Return the highlight_token of the current event, or None
    /// Primary value read by SnapshotBuilder to populate
    /// RendererSnapshot.active_highlight_token
    pub fn active_highlight_token(&self) -> Option<&str> {
        self.current()
            .and_then(|event| event.highlight_token.as_deref())
    }

    /// Return the slice events[0..=current_index]
    /// Used by Equation Panel to render step history list
    pub fn history_up_to_current(&self) -> &[MathStepEvent] {
        if self.current_index < self.events.len() {
            &self.events[..=self.current_index]
        } else if !self.events.is_empty() {
            &self.events
        } else {
            &[]
        }
    }

    /// Reset current_index to 0
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Append an event to the trace
    /// Sets its step_index and increments total_steps
    /// Called by StepEmitter
    pub fn push_event(&mut self, mut event: MathStepEvent) {
        event.step_index = self.total_steps;
        self.events.push(event);
        self.total_steps += 1;
    }

    /// Mark the trace as complete
    /// Called by StepEmitter after root expression returns
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
    }

    /// Return the total number of steps
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if trace is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get read-only access to all events
    pub fn all_events(&self) -> &[MathStepEvent] {
        &self.events
    }

    /// Get the first event, if any
    pub fn first(&self) -> Option<&MathStepEvent> {
        self.events.first()
    }

    /// Get the last event, if any
    pub fn last(&self) -> Option<&MathStepEvent> {
        self.events.last()
    }
}

impl Default for MathStepTrace {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step_trace::event::StepOperation;

    #[test]
    fn test_trace_creation() {
        let trace = MathStepTrace::new(Some("eq_1".to_string()));
        assert_eq!(trace.len(), 0);
        assert!(trace.is_empty());
        assert_eq!(trace.equation_id, Some("eq_1".to_string()));
    }

    #[test]
    fn test_push_event() {
        let mut trace = MathStepTrace::new(None);
        let event = MathStepEvent::new(Some("n1".to_string()), StepOperation::Add, 5.0, vec![]);

        trace.push_event(event.clone());
        assert_eq!(trace.len(), 1);
        assert_eq!(trace.total_steps, 1);
        assert_eq!(trace.events[0].step_index, 0);
    }

    #[test]
    fn test_navigation() {
        let mut trace = MathStepTrace::new(None);

        for i in 0..3 {
            let event = MathStepEvent::new(
                Some(format!("n{}", i)),
                StepOperation::Add,
                i as f64,
                vec![],
            );
            trace.push_event(event);
        }

        assert_eq!(trace.current_index, 0);
        trace.advance();
        assert_eq!(trace.current_index, 1);
        trace.rewind();
        assert_eq!(trace.current_index, 0);

        trace.jump_to(2);
        assert_eq!(trace.current_index, 2);
    }

    #[test]
    fn test_history_up_to_current() {
        let mut trace = MathStepTrace::new(None);

        for i in 0..3 {
            let event = MathStepEvent::new(
                Some(format!("n{}", i)),
                StepOperation::Add,
                i as f64,
                vec![],
            );
            trace.push_event(event);
        }

        trace.jump_to(1);
        let history = trace.history_up_to_current();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_active_highlight_token() {
        let mut trace = MathStepTrace::new(None);

        let event = MathStepEvent::new(None, StepOperation::Add, 5.0, vec![])
            .with_highlight("hk_01".to_string());

        trace.push_event(event);
        assert_eq!(trace.active_highlight_token(), Some("hk_01"));
    }
}
