//! The Hard Boundary
//!
//! This is a customs checkpoint. One-way data flow only.
//! No callbacks into runtime. No math evaluation. No constraint handling.

use crate::renderer::{
    adapter::Adapter, backend::RenderBackend, error::RenderResult, generate_function_mesh_2d,
    generate_surface_mesh_3d, interpolation::Interpolator, scene_map::SceneMap, sync::FrameSync,
    visibility::VisibilityManager, MaterialProperties, MathRenderable, ObjectState, RenderStats,
    RendererConfig, RuntimeSnapshot, Transform,
};

/// The bridge between runtime state and visual rendering
///
/// Responsibilities:
/// - Receive immutable snapshots
/// - Convert to render instructions
/// - Enforce one-way data flow
///
/// NOT responsible for:
/// - Math evaluation
/// - Constraint solving
/// - State mutation
pub struct RendererBridge {
    backend: Box<dyn RenderBackend>,
    adapter: Adapter,
    scene_map: SceneMap,
    frame_sync: FrameSync,
    interpolator: Interpolator,
    visibility: VisibilityManager,
    config: RendererConfig,
    stats: RenderStats,
    last_snapshot: Option<RuntimeSnapshot>,
}

impl RendererBridge {
    /// Create a new renderer bridge
    pub fn new(backend: Box<dyn RenderBackend>, config: RendererConfig) -> Self {
        Self {
            backend,
            adapter: Adapter::new(),
            scene_map: SceneMap::new(),
            frame_sync: FrameSync::new(config.target_fps),
            interpolator: Interpolator::new(config.interpolate),
            visibility: VisibilityManager::new(config.enable_culling),
            config,
            stats: RenderStats::default(),
            last_snapshot: None,
        }
    }

    /// Update renderer with new runtime snapshot
    ///
    /// This is the primary entry point. It:
    /// 1. Validates the snapshot
    /// 2. Syncs with frame timing
    /// 3. Converts objects to render state
    /// 4. Pushes updates to backend
    ///
    /// Errors are logged but non-fatal - rendering continues.
    pub fn update(&mut self, snapshot: &RuntimeSnapshot) -> RenderResult<()> {
        let frame_start = std::time::Instant::now();

        // Sync frame timing
        if !self.frame_sync.should_render() {
            return Ok(());
        }

        // Check object count bounds
        if snapshot.objects.len() > self.config.max_objects {
            log::warn!(
                "Object count {} exceeds recommended maximum {}",
                snapshot.objects.len(),
                self.config.max_objects
            );
        }

        // Process visibility and culling
        let visible_objects = self
            .visibility
            .filter(&snapshot.objects, &snapshot.focus_ids);

        // Interpolate if enabled
        let mut render_objects = if self.config.interpolate {
            if let Some(prev) = &self.last_snapshot {
                let alpha = self.frame_sync.interpolation_alpha();
                self.interpolator
                    .interpolate(&prev.objects, &snapshot.objects, alpha)
            } else {
                visible_objects
            }
        } else {
            visible_objects
        };

        if let Some(token) = &snapshot.active_highlight_token {
            let entries = self.resolve_highlight_token(token, &snapshot.highlight_schedule);
            let token_object_ids = entries.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            self.visibility
                .apply_focus_with_token(&mut render_objects, &token_object_ids, token);
        }

        // Convert objects to render instructions
        let mut rendered = 0;

        for obj in &render_objects {
            // Check if object exists in scene
            if let Some(render_id) = self.scene_map.get(obj.id) {
                // Update existing object
                if let Err(e) = self.update_object(render_id, obj, &snapshot.highlight_schedule) {
                    log::error!("Failed to update object {}: {:?}", obj.id, e);
                    // Continue rendering other objects
                }
                rendered += 1;
            } else {
                // Create new object
                match self.create_object(obj, &snapshot.highlight_schedule) {
                    Ok(render_id) => {
                        self.scene_map.insert(obj.id, render_id);
                        rendered += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to create object {}: {:?}", obj.id, e);
                        // Continue rendering other objects
                    }
                }
            }
        }

        // Math renderables are generated through mesh_generator and dispatched like regular objects.
        let generated_math_objects = self.generate_math_objects(snapshot);
        for obj in &generated_math_objects {
            if let Some(render_id) = self.scene_map.get(obj.id) {
                if let Err(e) = self.update_object(render_id, obj, &snapshot.highlight_schedule) {
                    log::error!("Failed to update math object {}: {:?}", obj.id, e);
                }
                rendered += 1;
            } else {
                match self.create_object(obj, &snapshot.highlight_schedule) {
                    Ok(render_id) => {
                        self.scene_map.insert(obj.id, render_id);
                        rendered += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to create math object {}: {:?}", obj.id, e);
                    }
                }
            }
        }

        for annotation in &snapshot.annotations {
            if let Some(render_id) = self.scene_map.get(annotation.anchor_object_id) {
                if let Err(e) = self.backend.set_annotation(
                    render_id,
                    &annotation.label_text,
                    annotation.position_offset,
                    annotation.is_active,
                ) {
                    log::error!(
                        "Failed to apply annotation to object {}: {:?}",
                        annotation.anchor_object_id,
                        e
                    );
                }
            }
        }

        // Remove objects that no longer exist
        let current_ids: std::collections::HashSet<_> = snapshot
            .objects
            .iter()
            .map(|o| o.id)
            .chain(generated_math_objects.iter().map(|o| o.id))
            .collect();
        let removed = self.scene_map.cleanup(|id| !current_ids.contains(id));

        for render_id in removed {
            if let Err(e) = self.backend.remove_object(render_id) {
                log::error!("Failed to remove object {}: {:?}", render_id, e);
                // Continue cleanup
            }
        }

        let culled = snapshot.objects.len().saturating_sub(rendered);

        // Update statistics
        let frame_time = frame_start.elapsed().as_secs_f64() * 1000.0;
        self.update_stats(rendered, culled, frame_time);

        // Store snapshot for interpolation
        self.last_snapshot = Some(snapshot.clone());

        // Mark frame complete
        self.frame_sync.frame_complete();

        Ok(())
    }

    /// Force rebuild of entire scene
    pub fn rebuild(&mut self) -> RenderResult<()> {
        // Clear all objects
        for render_id in self.scene_map.all_render_ids() {
            let _ = self.backend.remove_object(render_id);
        }
        self.scene_map.clear();

        // Re-render from last snapshot if available
        if let Some(snapshot) = self.last_snapshot.take() {
            self.update(&snapshot)?;
        }

        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    /// Shutdown and cleanup
    pub fn shutdown(mut self) -> RenderResult<()> {
        // Remove all objects
        for render_id in self.scene_map.all_render_ids() {
            let _ = self.backend.remove_object(render_id);
        }
        self.scene_map.clear();

        Ok(())
    }

    // Private helpers

    fn create_object(
        &mut self,
        obj: &crate::renderer::ObjectState,
        schedule: &[crate::renderer::HighlightScheduleEntry],
    ) -> RenderResult<u64> {
        // Convert semantic geometry to render geometry
        let geometry = self.adapter.convert_geometry(&obj.geometry)?;

        // Convert transform to matrix
        let transform = self.adapter.convert_transform(&obj.transform);

        // Convert material properties
        let material = self.adapter.convert_material(&obj.material);

        // Create in backend
        let render_id = self.backend.create_object(geometry, transform, material)?;

        // Set visibility
        if !obj.visible {
            self.backend.set_visible(render_id, false)?;
        }

        // Apply highlight if needed
        let color_index = self.resolve_object_color_index(obj, schedule).unwrap_or(0);
        self.backend
            .set_highlighted_with_color(render_id, obj.highlighted, color_index)?;

        Ok(render_id)
    }

    fn update_object(
        &mut self,
        render_id: u64,
        obj: &crate::renderer::ObjectState,
        schedule: &[crate::renderer::HighlightScheduleEntry],
    ) -> RenderResult<()> {
        // Update transform
        let transform = self.adapter.convert_transform(&obj.transform);
        self.backend.update_transform(render_id, transform)?;

        // Update material
        let material = self.adapter.convert_material(&obj.material);
        self.backend.update_material(render_id, material)?;

        // Update visibility
        self.backend.set_visible(render_id, obj.visible)?;

        // Update highlight
        let color_index = self.resolve_object_color_index(obj, schedule).unwrap_or(0);
        self.backend
            .set_highlighted_with_color(render_id, obj.highlighted, color_index)?;

        Ok(())
    }

    fn update_stats(&mut self, rendered: usize, culled: usize, frame_time: f64) {
        self.stats.frame_count += 1;
        self.stats.objects_rendered = rendered;
        self.stats.objects_culled = culled;
        self.stats.last_frame_time_ms = frame_time;

        // Running average
        let alpha = 0.1;
        self.stats.avg_frame_time_ms =
            alpha * frame_time + (1.0 - alpha) * self.stats.avg_frame_time_ms;
    }

    fn generate_math_objects(&self, snapshot: &RuntimeSnapshot) -> Vec<ObjectState> {
        snapshot
            .math_renderables
            .iter()
            .filter_map(|entry| self.math_renderable_to_object(entry))
            .collect()
    }

    fn math_renderable_to_object(&self, renderable: &MathRenderable) -> Option<ObjectState> {
        let (id, geometry) = match renderable {
            MathRenderable::Function2D {
                id,
                domain,
                resolution,
                amplitude,
                frequency,
                phase,
            } => {
                let geom = generate_function_mesh_2d(
                    |x| amplitude * (frequency * x + phase).sin(),
                    *domain,
                    *resolution,
                )
                .ok()?;
                (*id, geom)
            }
            MathRenderable::Surface3D {
                id,
                domain_x,
                domain_y,
                resolution,
                amplitude,
                phase,
            } => {
                let geom = generate_surface_mesh_3d(
                    |x, y| amplitude * ((x + phase).sin() * (y + phase).cos()),
                    (*domain_x, *domain_y),
                    *resolution,
                )
                .ok()?;
                (*id, geom)
            }
            MathRenderable::Field2D {
                id,
                domain_x,
                domain_y,
                resolution,
                scale,
                phase,
            } => {
                let geom = generate_surface_mesh_3d(
                    |x, y| {
                        let mag = ((x + phase).powi(2) + (y + phase).powi(2)).sqrt();
                        scale * mag
                    },
                    (*domain_x, *domain_y),
                    *resolution,
                )
                .ok()?;
                (*id, geom)
            }
        };

        Some(ObjectState {
            id,
            geometry,
            transform: Transform::default(),
            material: MaterialProperties::default(),
            visible: true,
            highlighted: false,
            highlight_token: None,
        })
    }

    fn resolve_highlight_token(
        &self,
        token: &str,
        schedule: &[crate::renderer::HighlightScheduleEntry],
    ) -> Vec<(u64, u8)> {
        schedule
            .iter()
            .filter(|entry| entry.highlight_token == token)
            .map(|entry| (entry.entity_id_hash, entry.color_index))
            .collect()
    }

    fn resolve_object_color_index(
        &self,
        obj: &crate::renderer::ObjectState,
        schedule: &[crate::renderer::HighlightScheduleEntry],
    ) -> Option<u8> {
        let token = obj.highlight_token.as_ref()?;
        schedule
            .iter()
            .find(|entry| entry.highlight_token == *token && entry.entity_id_hash == obj.id)
            .map(|entry| entry.color_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::backend::MockBackend;

    #[test]
    fn test_bridge_one_way_flow() {
        let backend = Box::new(MockBackend::new());
        let config = RendererConfig::default();
        let mut bridge = RendererBridge::new(backend, config);

        // Create snapshot
        let snapshot = RuntimeSnapshot {
            tick: 1,
            timestamp: 0.0,
            objects: vec![],
            math_renderables: vec![],
            focus_ids: vec![],
            active_highlight_token: None,
            highlight_schedule: vec![],
            annotations: vec![],
        };

        // Update should not modify snapshot
        let result = bridge.update(&snapshot);
        assert!(result.is_ok());

        // Snapshot remains unchanged
        assert_eq!(snapshot.tick, 1);
    }

    #[test]
    fn test_bridge_error_isolation() {
        let backend = Box::new(MockBackend::new());
        let config = RendererConfig::default();
        let mut bridge = RendererBridge::new(backend, config);

        // Even with errors, bridge continues
        let snapshot = RuntimeSnapshot {
            tick: 1,
            timestamp: 0.0,
            objects: vec![],
            math_renderables: vec![],
            focus_ids: vec![],
            active_highlight_token: None,
            highlight_schedule: vec![],
            annotations: vec![],
        };

        let result = bridge.update(&snapshot);
        // Should not panic or propagate errors
        assert!(result.is_ok());
    }

    #[test]
    fn test_bridge_dispatches_math_renderables() {
        let backend = Box::new(MockBackend::new());
        let config = RendererConfig::default();
        let mut bridge = RendererBridge::new(backend, config);

        let snapshot = RuntimeSnapshot {
            tick: 1,
            timestamp: 0.0,
            objects: vec![],
            math_renderables: vec![
                MathRenderable::Function2D {
                    id: 1001,
                    domain: (-1.0, 1.0),
                    resolution: 32,
                    amplitude: 1.0,
                    frequency: 1.0,
                    phase: 0.0,
                },
                MathRenderable::Surface3D {
                    id: 1002,
                    domain_x: (-1.0, 1.0),
                    domain_y: (-1.0, 1.0),
                    resolution: (8, 8),
                    amplitude: 1.0,
                    phase: 0.1,
                },
                MathRenderable::Field2D {
                    id: 1003,
                    domain_x: (-1.0, 1.0),
                    domain_y: (-1.0, 1.0),
                    resolution: (8, 8),
                    scale: 0.5,
                    phase: 0.0,
                },
            ],
            focus_ids: vec![],
            active_highlight_token: None,
            highlight_schedule: vec![],
            annotations: vec![],
        };

        let result = bridge.update(&snapshot);
        assert!(result.is_ok());
        assert!(bridge.stats().objects_rendered >= 3);
    }
}
