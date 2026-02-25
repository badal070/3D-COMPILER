//! Renderer Bridge + Visualization Adapter
//!
//! This module translates runtime state into render instructions.
//! It does NOT interpret, optimize, or make semantic decisions.
//!
//! Design law: Renderer may fail silently, but must never invent behavior.

pub mod adapter;
pub mod backend;
pub mod bridge;
pub mod error;
pub mod interpolation;
pub mod mesh_generator;
pub mod scene_map;
pub mod sync;
pub mod visibility;

pub use bridge::RendererBridge;
pub use error::{RenderError, RenderResult};
pub use mesh_generator::{
    generate_function_mesh_2d, generate_parametric_surface_mesh, generate_surface_mesh_3d,
};

/// Renderer configuration
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// Target frames per second
    pub target_fps: u32,
    /// Enable frame interpolation
    pub interpolate: bool,
    /// Enable culling optimizations
    pub enable_culling: bool,
    /// Maximum number of objects before warnings
    pub max_objects: usize,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            interpolate: true,
            enable_culling: true,
            max_objects: 10000,
        }
    }
}

/// Public interface for the renderer subsystem
pub struct Renderer {
    bridge: RendererBridge,
}

impl Renderer {
    /// Create a new renderer with the specified backend
    pub fn new(backend: Box<dyn backend::RenderBackend>, config: RendererConfig) -> Self {
        Self {
            bridge: RendererBridge::new(backend, config.clone()),
        }
    }

    /// Update the renderer with a new runtime snapshot
    /// This is the primary entry point for rendering
    pub fn update(&mut self, snapshot: &RuntimeSnapshot) -> RenderResult<()> {
        self.bridge.update(snapshot)
    }

    /// Force a full scene rebuild
    pub fn rebuild(&mut self) -> RenderResult<()> {
        self.bridge.rebuild()
    }

    /// Get current render statistics
    pub fn stats(&self) -> RenderStats {
        self.bridge.stats()
    }

    /// Shutdown and cleanup resources
    pub fn shutdown(self) -> RenderResult<()> {
        self.bridge.shutdown()
    }

    /// Build a 2D function plot as line geometry.
    pub fn render_function_plot<F>(
        &self,
        f: F,
        domain: (f64, f64),
        resolution: usize,
    ) -> RenderResult<GeometryType>
    where
        F: Fn(f64) -> f64,
    {
        generate_function_mesh_2d(f, domain, resolution)
    }

    /// Build a z=f(x,y) surface mesh.
    pub fn render_surface_plot<F>(
        &self,
        f: F,
        domain: ((f64, f64), (f64, f64)),
        resolution: (usize, usize),
    ) -> RenderResult<GeometryType>
    where
        F: Fn(f64, f64) -> f64,
    {
        generate_surface_mesh_3d(f, domain, resolution)
    }

    /// Build a parametric surface mesh.
    pub fn render_parametric_surface<F>(
        &self,
        f: F,
        domain: ((f64, f64), (f64, f64)),
        resolution: (usize, usize),
    ) -> RenderResult<GeometryType>
    where
        F: Fn(f64, f64) -> [f64; 3],
    {
        generate_parametric_surface_mesh(f, domain, resolution)
    }
}

/// Immutable snapshot from runtime
/// This is what the renderer receives - never modifies
#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub tick: u64,
    pub timestamp: f64,
    pub objects: Vec<ObjectState>,
    pub math_renderables: Vec<MathRenderable>,
    pub focus_ids: Vec<u64>,
    pub active_highlight_token: Option<String>,
    pub highlight_schedule: Vec<HighlightScheduleEntry>,
    pub annotations: Vec<AnnotationOverlay>,
}

#[derive(Debug, Clone)]
pub struct HighlightScheduleEntry {
    pub at_time: f64,
    pub highlight_token: String,
    pub entity_id_hash: u64,
    pub color_index: u8,
}

#[derive(Debug, Clone)]
pub struct AnnotationOverlay {
    pub label_text: String,
    pub anchor_object_id: u64,
    pub position_offset: [f64; 3],
    pub equation_node_id: Option<String>,
    pub highlight_token: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub enum MathRenderable {
    Function2D {
        id: u64,
        domain: (f64, f64),
        resolution: usize,
        amplitude: f64,
        frequency: f64,
        phase: f64,
    },
    Surface3D {
        id: u64,
        domain_x: (f64, f64),
        domain_y: (f64, f64),
        resolution: (usize, usize),
        amplitude: f64,
        phase: f64,
    },
    Field2D {
        id: u64,
        domain_x: (f64, f64),
        domain_y: (f64, f64),
        resolution: (usize, usize),
        scale: f64,
        phase: f64,
    },
}

/// State of a single object at a point in time
#[derive(Debug, Clone)]
pub struct ObjectState {
    pub id: u64,
    pub geometry: GeometryType,
    pub transform: Transform,
    pub material: MaterialProperties,
    pub visible: bool,
    pub highlighted: bool,
    pub highlight_token: Option<String>,
}

/// Geometry type from the semantic layer
#[derive(Debug, Clone)]
pub enum GeometryType {
    Sphere {
        radius: f64,
    },
    Box {
        width: f64,
        height: f64,
        depth: f64,
    },
    Cylinder {
        radius: f64,
        height: f64,
    },
    Cone {
        radius: f64,
        height: f64,
    },
    Plane {
        width: f64,
        height: f64,
    },
    Line {
        points: Vec<[f64; 3]>,
    },
    Mesh {
        vertices: Vec<[f64; 3]>,
        indices: Vec<u32>,
    },
}

/// Transform in 3D space
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: [f64; 3],
    pub rotation: [f64; 4], // Quaternion
    pub scale: [f64; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Material properties for visual appearance
#[derive(Debug, Clone)]
pub struct MaterialProperties {
    pub color: [f32; 4], // RGBA
    pub metallic: f32,
    pub roughness: f32,
    pub opacity: f32,
    pub emissive: [f32; 3],
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            opacity: 1.0,
            emissive: [0.0, 0.0, 0.0],
        }
    }
}

/// Rendering statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    pub frame_count: u64,
    pub objects_rendered: usize,
    pub objects_culled: usize,
    pub last_frame_time_ms: f64,
    pub avg_frame_time_ms: f64,
}
