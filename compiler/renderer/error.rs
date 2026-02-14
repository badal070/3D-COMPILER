//! Error Handling
//!
//! Renderer errors are:
//! - Logged
//! - Non-fatal
//! - Never propagated back into runtime
//!
//! If rendering fails, learning pauses visually, not mathematically.

use std::fmt;

/// Result type for renderer operations
pub type RenderResult<T> = Result<T, RenderError>;

/// Renderer error types
///
/// These errors affect visual rendering only.
/// They should NEVER impact runtime state or mathematics.
#[derive(Debug, Clone)]
pub enum RenderError {
    /// Backend failed to initialize
    BackendInit(String),

    /// Failed to create geometry
    GeometryCreation,

    /// Invalid geometry parameters
    InvalidGeometry(String),

    /// Failed to create material
    MaterialCreation,

    /// Failed to create renderable object
    ObjectCreation,

    /// Object not found by ID
    ObjectNotFound(u64),

    /// Failed to update object state
    UpdateFailed,

    /// Transform computation failed
    TransformError(String),

    /// Interpolation failed
    InterpolationError(String),

    /// Backend-specific error
    BackendError(String),

    /// General rendering error
    Other(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BackendInit(msg) => write!(f, "Backend initialization failed: {}", msg),
            RenderError::GeometryCreation => write!(f, "Failed to create geometry"),
            RenderError::InvalidGeometry(msg) => write!(f, "Invalid geometry: {}", msg),
            RenderError::MaterialCreation => write!(f, "Failed to create material"),
            RenderError::ObjectCreation => write!(f, "Failed to create render object"),
            RenderError::ObjectNotFound(id) => write!(f, "Render object {} not found", id),
            RenderError::UpdateFailed => write!(f, "Failed to update render state"),
            RenderError::TransformError(msg) => write!(f, "Transform error: {}", msg),
            RenderError::InterpolationError(msg) => write!(f, "Interpolation error: {}", msg),
            RenderError::BackendError(msg) => write!(f, "Backend error: {}", msg),
            RenderError::Other(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for RenderError {}

/// Log a render error without panicking
///
/// This is the standard way to handle render errors:
/// Log them, but don't stop the system.
pub fn log_render_error(error: &RenderError, context: &str) {
    log::error!("[Renderer] {} - {}", context, error);
}

/// Handle render error with recovery
///
/// Logs error and executes recovery action
pub fn handle_render_error<F>(error: RenderError, context: &str, recovery: F)
where
    F: FnOnce(),
{
    log_render_error(&error, context);
    recovery();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = RenderError::ObjectNotFound(42);
        assert_eq!(error.to_string(), "Render object 42 not found");

        let error = RenderError::InvalidGeometry("negative radius".into());
        assert_eq!(error.to_string(), "Invalid geometry: negative radius");
    }

    #[test]
    fn test_error_logging() {
        let error = RenderError::UpdateFailed;
        log_render_error(&error, "test context");
        // Should log without panicking
    }

    #[test]
    fn test_error_recovery() {
        let error = RenderError::ObjectCreation;
        let mut recovered = false;

        handle_render_error(error, "test", || {
            recovered = true;
        });

        assert!(recovered);
    }

    #[test]
    fn test_result_type() {
        fn test_function() -> RenderResult<u64> {
            Ok(42)
        }

        fn test_error() -> RenderResult<u64> {
            Err(RenderError::UpdateFailed)
        }

        assert!(test_function().is_ok());
        assert!(test_error().is_err());
    }
}
