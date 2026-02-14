// compiler/runtime/mod.rs
// Simplified runtime for web demo

pub mod config;
#[path = "src/educational/mod.rs"]
pub mod educational;
pub mod error;
pub mod loader;
#[path = "src/math/mod.rs"]
pub mod math;
#[path = "src/modules/mod.rs"]
pub mod modules;
#[path = "src/numerical/mod.rs"]
pub mod numerical;
pub mod snapshot_builder;
pub mod state;
#[path = "src/symbolic/mod.rs"]
pub mod symbolic;
#[path = "src/visualization/mod.rs"]
pub mod visualization;

pub use loader::SceneLoader;
pub use snapshot_builder::{RendererSnapshot, SnapshotBuilder};
pub use state::RuntimeState;
