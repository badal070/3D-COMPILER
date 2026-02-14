//! Backend Abstraction
//!
//! Renderer gets commands, not logic.

pub mod babylon;
pub mod native;
pub mod three_js;
pub mod traits;

pub use traits::{RenderBackend, RenderGeometry, RenderMaterial, RenderTransform};

#[cfg(test)]
pub use traits::MockBackend;
