//! Step Trace & Equation Highlight System
//!
//! # Architecture
//!
//! This module is the instrumentation layer between the runtime math engine and
//! everything above it. It sits at this position in the architecture:
//!
//! ```text
//! RuntimeMathEngine (existing)
//! │ emits events during evaluate_scalar()
//! ▼
//! ┌─────────────┐
//! │ step_trace │ ◄── this module
//! └──────┬──────┘
//! │ broadcasts MathStepEvent
//! ▼
//! explanation_bus → Frontend Panels
//! ```
//!
//! # Design Principles
//!
//! - **No upstream dependencies on new modules**: Only depends on existing types
//!   from dsl/ast.rs (for node_id values) and produces events consumed by
//!   explanation_bus and snapshot_builder.rs
//!
//! - **Atomic events**: MathStepEvent is the smallest unit of tracing. One event
//!   per sub-expression evaluation.
//!
//! - **Shared identity**: HighlightToken ties together equation spans, prose
//!   spans, and 3D objects using consistent color indices.
//!
//! - **Post-order evaluation**: Events are emitted in natural mathematical order
//!   (leaves first, root last), which is how the frontend displays them.
//!
//! # Key Data Structures
//!
//! - **MathStepEvent**: Atomic record of one sub-expression evaluation
//! - **MathStepTrace**: Ordered log of events with navigation support
//! - **HighlightToken**: Pairs token string with color index
//! - **HighlightTokenRegistry**: Registry mapping tokens to colors
//! - **ColorTable**: 16-color palette shared with frontend
//! - **StepEmitter**: Pub-sub broadcaster for events
//!
//! # Typical Flow
//!
//! 1. RuntimeMathEngine calls emitter.begin_trace(equation_id)
//! 2. For each sub-expression evaluated:
//!    - RuntimeMathEngine creates MathStepEvent
//!    - Calls emitter.emit(event)
//!    - Subscribers receive on_step callback
//! 3. After root expression returns, emitter.end_trace()
//!    - Subscribers receive on_trace_complete callback
//! 4. SnapshotBuilder reads the completed trace
//! 5. Trace is serialized and sent to frontend over WebSocket

pub mod color_table;
pub mod emitter;
pub mod event;
pub mod token;
pub mod trace;

// Public re-exports
pub use color_table::{ColorEntry, ColorTable};
pub use emitter::{StepEmitter, StepSubscriber};
pub use event::{MathStepEvent, StepOperation};
pub use token::{HighlightToken, HighlightTokenRegistry};
pub use trace::MathStepTrace;
