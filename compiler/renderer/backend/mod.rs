//! Backend Abstraction
//!
//! Renderer gets commands, not logic.

pub mod native;
pub mod traits;
#[cfg(not(target_arch = "wasm32"))]
pub mod babylon;
#[cfg(not(target_arch = "wasm32"))]
pub mod three_js;

pub use traits::{RenderAnnotation, RenderBackend, RenderGeometry, RenderMaterial, RenderTransform};

#[cfg(test)]
pub use traits::MockBackend;
