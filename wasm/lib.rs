// wasm/lib.rs
// WASM bridge for web compilation and rendering

use runtime::config::RuntimeConfigs;
use runtime::math::{BinaryOperator, Expression, RuntimeMathEngine};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmCompiler {
    runtime_state: Option<RuntimeState>,
    snapshot_builder: SnapshotBuilder,
    math_engine: RuntimeMathEngine,
    runtime_configs: RuntimeConfigs,
    current_tick: u64,
}

#[wasm_bindgen]
impl WasmCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Set panic hook for better error messages
        console_error_panic_hook::set_once();

        Self {
            runtime_state: None,
            snapshot_builder: SnapshotBuilder::new(),
            math_engine: RuntimeMathEngine::new(),
            runtime_configs: RuntimeConfigs::load_default(),
            current_tick: 0,
        }
    }

    /// Compile DSL source and initialize runtime
    #[wasm_bindgen]
    pub fn compile(&mut self, source: &str) -> Result<JsValue, JsValue> {
        // 1. Compile DSL to IR
        let compiler = dsl_compiler::Compiler::new();
        let ir_scene = compiler
            .compile(source.to_string(), std::path::PathBuf::from("input.dsl"))
            .map_err(|errors: Vec<dsl_compiler::errors::DslError>| {
                let error_msg = errors
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                JsValue::from_str(&error_msg)
            })?;

        // 2. Load IR into runtime
        let runtime_state = runtime::loader::SceneLoader::load_scene(&ir_scene)
            .map_err(|e| JsValue::from_str(&format!("Runtime load error: {}", e)))?;

        self.runtime_state = Some(runtime_state);
        self.refresh_math_state()?;
        self.current_tick = 0;

        Ok(JsValue::from_str("Compilation successful"))
    }

    /// Step simulation forward by one frame
    #[wasm_bindgen]
    pub fn step(&mut self) -> Result<(), JsValue> {
        if let Some(state) = &mut self.runtime_state {
            // Step size sourced from numerical_methods.toml (fallbacks apply if file missing).
            let dt = self
                .runtime_configs
                .numerical_methods
                .ode_solving
                .default_step_size;

            state
                .time
                .advance(dt)
                .map_err(|e| JsValue::from_str(&format!("Time advance error: {}", e)))?;

            self.refresh_math_state()?;
            self.current_tick += 1;
            Ok(())
        } else {
            Err(JsValue::from_str("No scene loaded"))
        }
    }

    /// Get current snapshot for rendering
    #[wasm_bindgen]
    pub fn get_snapshot(&mut self) -> Result<JsValue, JsValue> {
        if let Some(state) = &self.runtime_state {
            let mut snapshot = self.snapshot_builder.build_snapshot(state);
            snapshot.math_preview = self.build_math_preview(state)?;
            let _renderer_snapshot = runtime_to_renderer_snapshot(&snapshot);

            // Serialize to JSON
            serde_wasm_bindgen::to_value(&snapshot)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
        } else {
            Err(JsValue::from_str("No scene loaded"))
        }
    }

    /// Reset simulation to initial state
    #[wasm_bindgen]
    pub fn reset(&mut self) -> Result<(), JsValue> {
        if let Some(state) = &mut self.runtime_state {
            state.time.reset();
            self.refresh_math_state()?;
            self.current_tick = 0;
            Ok(())
        } else {
            Err(JsValue::from_str("No scene loaded"))
        }
    }

    /// Get current simulation time
    #[wasm_bindgen]
    pub fn get_time(&self) -> f64 {
        self.runtime_state
            .as_ref()
            .map(|s| s.time.current_time)
            .unwrap_or(0.0)
    }

    fn refresh_math_state(&mut self) -> Result<(), JsValue> {
        let state = self
            .runtime_state
            .as_mut()
            .ok_or_else(|| JsValue::from_str("No scene loaded"))?;

        let t = state.time.current_time;
        let mut variables = std::collections::HashMap::new();
        variables.insert("t".to_string(), t);

        let phase_expr = Expression::Binary(
            Box::new(Expression::Binary(
                Box::new(Expression::Number(
                    self.runtime_configs
                        .mathematical_functions
                        .constants
                        .get("e")
                        .copied()
                        .unwrap_or(2.0),
                )),
                BinaryOperator::Multiply,
                Box::new(Expression::Variable("t".to_string())),
            )),
            BinaryOperator::Add,
            Box::new(Expression::Number(
                self.runtime_configs
                    .mathematical_functions
                    .constants
                    .get("pi")
                    .copied()
                    .unwrap_or(std::f64::consts::PI)
                    * 0.1,
            )),
        );
        let wave_expr = Expression::FunctionCall("sin".to_string(), vec![phase_expr]);
        let wave = self
            .math_engine
            .evaluate_expression(&wave_expr, &variables)
            .map_err(|e| JsValue::from_str(&format!("Math evaluation error: {}", e)))?;

        let decay_expr = Expression::FunctionCall(
            "exp".to_string(),
            vec![Expression::Unary(
                runtime::math::UnaryOperator::Negate,
                Box::new(Expression::Variable("t".to_string())),
            )],
        );
        let decay = self
            .math_engine
            .evaluate_expression(&decay_expr, &variables)
            .map_err(|e| JsValue::from_str(&format!("Math evaluation error: {}", e)))?;

        state
            .math_values
            .insert("time".to_string(), runtime::math::MathValue::Real(t));
        state.math_values.insert("wave".to_string(), wave);
        state.math_values.insert("decay".to_string(), decay);
        Ok(())
    }

    fn build_math_preview(
        &self,
        state: &RuntimeState,
    ) -> Result<Option<runtime::snapshot_builder::SnapshotMathPreview>, JsValue> {
        let wave = extract_real_math_value(state, "wave").unwrap_or(0.0);
        let decay = extract_real_math_value(state, "decay").unwrap_or(1.0);
        let phase = extract_real_math_value(state, "time").unwrap_or(0.0);

        let geometry = renderer::generate_function_mesh_2d(
            |x| decay * (x + phase).sin() + 0.1 * wave,
            (-3.0, 3.0),
            self.runtime_configs
                .visualization_defaults
                .plotting
                .default_resolution_2d
                .max(2),
        )
        .map_err(|e| JsValue::from_str(&format!("Renderer math preview error: {}", e)))?;

        let points = match geometry {
            renderer::GeometryType::Line { points } => points,
            _ => Vec::new(),
        };

        Ok(Some(runtime::snapshot_builder::SnapshotMathPreview {
            points,
        }))
    }
}

// Re-export types needed by WASM
use runtime::snapshot_builder::SnapshotBuilder;
use runtime::state::RuntimeState;

fn extract_real_math_value(state: &RuntimeState, key: &str) -> Option<f64> {
    state.math_values.get(key).and_then(|value| match value {
        runtime::math::MathValue::Real(v) => Some(*v),
        runtime::math::MathValue::Integer(v) => Some(*v as f64),
        runtime::math::MathValue::Rational(num, den) if *den != 0 => {
            Some(*num as f64 / *den as f64)
        }
        _ => None,
    })
}

fn runtime_to_renderer_snapshot(
    snapshot: &runtime::snapshot_builder::RendererSnapshot,
) -> renderer::RuntimeSnapshot {
    let objects = snapshot
        .objects
        .iter()
        .map(|obj| renderer::ObjectState {
            id: obj.id,
            geometry: match &obj.geometry {
                runtime::snapshot_builder::SnapshotGeometry::Sphere { radius } => {
                    renderer::GeometryType::Sphere { radius: *radius }
                }
                runtime::snapshot_builder::SnapshotGeometry::Box {
                    width,
                    height,
                    depth,
                } => renderer::GeometryType::Box {
                    width: *width,
                    height: *height,
                    depth: *depth,
                },
                runtime::snapshot_builder::SnapshotGeometry::Cylinder { radius, height } => {
                    renderer::GeometryType::Cylinder {
                        radius: *radius,
                        height: *height,
                    }
                }
                runtime::snapshot_builder::SnapshotGeometry::Plane { width, height } => {
                    renderer::GeometryType::Plane {
                        width: *width,
                        height: *height,
                    }
                }
            },
            transform: renderer::Transform {
                position: obj.transform.position,
                rotation: obj.transform.rotation,
                scale: obj.transform.scale,
            },
            material: renderer::MaterialProperties {
                color: obj.material.color,
                metallic: obj.material.metallic,
                roughness: obj.material.roughness,
                opacity: obj.material.opacity,
                emissive: obj.material.emissive,
            },
            visible: obj.visible,
            highlighted: obj.highlighted,
        })
        .collect();

    let math_renderables = snapshot
        .math_renderables
        .iter()
        .map(|entry| match entry {
            runtime::snapshot_builder::SnapshotMathRenderable::Function {
                id,
                domain,
                resolution,
                amplitude,
                frequency,
                phase,
            } => renderer::MathRenderable::Function2D {
                id: *id,
                domain: (domain[0], domain[1]),
                resolution: *resolution,
                amplitude: *amplitude,
                frequency: *frequency,
                phase: *phase,
            },
            runtime::snapshot_builder::SnapshotMathRenderable::Surface {
                id,
                domain_x,
                domain_y,
                resolution,
                amplitude,
                phase,
            } => renderer::MathRenderable::Surface3D {
                id: *id,
                domain_x: (domain_x[0], domain_x[1]),
                domain_y: (domain_y[0], domain_y[1]),
                resolution: (resolution[0], resolution[1]),
                amplitude: *amplitude,
                phase: *phase,
            },
            runtime::snapshot_builder::SnapshotMathRenderable::Field {
                id,
                domain_x,
                domain_y,
                resolution,
                scale,
                phase,
            } => renderer::MathRenderable::Field2D {
                id: *id,
                domain_x: (domain_x[0], domain_x[1]),
                domain_y: (domain_y[0], domain_y[1]),
                resolution: (resolution[0], resolution[1]),
                scale: *scale,
                phase: *phase,
            },
        })
        .collect();

    renderer::RuntimeSnapshot {
        tick: snapshot.tick,
        timestamp: snapshot.timestamp,
        objects,
        math_renderables,
        focus_ids: snapshot.focus_ids.clone(),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;

    fn compile_and_step(path: &str) {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path, e));
        let mut compiler = WasmCompiler::new();
        compiler
            .compile(&source)
            .unwrap_or_else(|e| panic!("compile failed for {}: {:?}", path, e));
        compiler.step().expect("step should succeed");
        let snapshot_value = compiler.get_snapshot().expect("snapshot should serialize");
        let snapshot_json: serde_json::Value =
            serde_wasm_bindgen::from_value(snapshot_value).expect("snapshot should deserialize");

        assert!(snapshot_json.get("math_values").is_some());
    }

    #[test]
    fn wasm_compiles_and_steps_math_examples() {
        compile_and_step("../examples/calculus_examples.dsl");
        compile_and_step("../examples/linear_algebra_examples.dsl");
        compile_and_step("../examples/differential_equations.dsl");
    }
}
