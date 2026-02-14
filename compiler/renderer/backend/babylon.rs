//! Babylon.js Backend
//!
//! Heavier scenes, better tooling, optional physics visuals (not real physics).
//!
//! Note: Physics here is VISUAL ONLY - the real physics happens in the runtime.

use crate::renderer::{
    backend::{RenderBackend, RenderGeometry, RenderMaterial, RenderTransform},
    error::{RenderError, RenderResult},
};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Babylon.js rendering backend
///
/// More feature-rich than Three.js, better for complex scenes
pub struct BabylonBackend {
    objects: HashMap<u64, BabylonObject>,
    #[cfg(target_arch = "wasm32")]
    scene_handle: JsValue,
    #[cfg(target_arch = "wasm32")]
    engine_handle: JsValue,
}

struct BabylonObject {
    #[cfg(target_arch = "wasm32")]
    js_mesh: JsValue,
    visible: bool,
    highlighted: bool,
}

impl BabylonBackend {
    /// Create new Babylon.js backend
    #[cfg(target_arch = "wasm32")]
    pub fn new(canvas_id: &str) -> RenderResult<Self> {
        let (engine, scene) = Self::init_babylon(canvas_id)?;
        Ok(Self {
            objects: HashMap::new(),
            scene_handle: scene,
            engine_handle: engine,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(_canvas_id: &str) -> RenderResult<Self> {
        Ok(Self {
            objects: HashMap::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn init_babylon(canvas_id: &str) -> RenderResult<(JsValue, JsValue)> {
        use js_sys::Array;

        // Get canvas element
        let window = web_sys::window().ok_or(RenderError::BackendInit("No window".into()))?;
        let document = window
            .document()
            .ok_or(RenderError::BackendInit("No document".into()))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(RenderError::BackendInit(format!(
                "Canvas {} not found",
                canvas_id
            )))?;

        // Create engine
        let engine_args = Array::new();
        engine_args.push(&canvas);
        engine_args.push(&JsValue::TRUE); // antialias

        let engine =
            js_sys::Reflect::construct(&js_sys::eval("BABYLON.Engine").unwrap(), &engine_args)
                .map_err(|_| RenderError::BackendInit("Failed to create Babylon engine".into()))?;

        // Create scene
        let scene_args = Array::new();
        scene_args.push(&engine);

        let scene =
            js_sys::Reflect::construct(&js_sys::eval("BABYLON.Scene").unwrap(), &scene_args)
                .map_err(|_| RenderError::BackendInit("Failed to create Babylon scene".into()))?;

        // Create default camera
        let camera_args = Array::new();
        camera_args.push(&"camera".into());
        camera_args.push(&js_sys::eval("new BABYLON.Vector3(0, 5, -10)").unwrap());
        camera_args.push(&scene);

        let camera = js_sys::Reflect::construct(
            &js_sys::eval("BABYLON.ArcRotateCamera").unwrap(),
            &camera_args,
        )
        .map_err(|_| RenderError::BackendInit("Failed to create camera".into()))?;

        // Attach camera controls
        let attach_fn = js_sys::Reflect::get(&camera, &"attachControl".into()).unwrap();
        let attach_args = Array::new();
        attach_args.push(&canvas);
        attach_args.push(&JsValue::TRUE);
        js_sys::Reflect::apply(
            &attach_fn.dyn_into::<js_sys::Function>().unwrap(),
            &camera,
            &attach_args,
        )
        .ok();

        // Create default lighting
        let light_args = Array::new();
        light_args.push(&"light".into());
        light_args.push(&js_sys::eval("new BABYLON.Vector3(1, 1, 0)").unwrap());
        light_args.push(&scene);

        js_sys::Reflect::construct(
            &js_sys::eval("BABYLON.HemisphericLight").unwrap(),
            &light_args,
        )
        .ok();

        // Start render loop
        let render_fn = js_sys::Reflect::get(&engine, &"runRenderLoop".into()).unwrap();
        let scene_clone = scene.clone();
        let render_callback = Closure::wrap(Box::new(move || {
            let render = js_sys::Reflect::get(&scene_clone, &"render".into()).unwrap();
            js_sys::Reflect::apply(
                &render.dyn_into::<js_sys::Function>().unwrap(),
                &scene_clone,
                &Array::new(),
            )
            .ok();
        }) as Box<dyn FnMut()>);

        let loop_args = Array::new();
        loop_args.push(render_callback.as_ref());
        js_sys::Reflect::apply(
            &render_fn.dyn_into::<js_sys::Function>().unwrap(),
            &engine,
            &loop_args,
        )
        .ok();

        render_callback.forget();

        Ok((engine, scene))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_mesh_js(&self, geometry: &RenderGeometry) -> RenderResult<JsValue> {
        use js_sys::Array;

        let mesh = match geometry {
            RenderGeometry::Sphere { radius, segments } => {
                let args = Array::new();
                args.push(&"sphere".into());

                let options = js_sys::Object::new();
                js_sys::Reflect::set(
                    &options,
                    &"diameter".into(),
                    &JsValue::from_f64(*radius as f64 * 2.0),
                )
                .ok();
                js_sys::Reflect::set(
                    &options,
                    &"segments".into(),
                    &JsValue::from_f64(*segments as f64),
                )
                .ok();
                args.push(&options);
                args.push(&self.scene_handle);

                js_sys::Reflect::construct(
                    &js_sys::eval("BABYLON.MeshBuilder.CreateSphere").unwrap(),
                    &args,
                )
                .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Box {
                width,
                height,
                depth,
            } => {
                let args = Array::new();
                args.push(&"box".into());

                let options = js_sys::Object::new();
                js_sys::Reflect::set(&options, &"width".into(), &JsValue::from_f64(*width as f64))
                    .ok();
                js_sys::Reflect::set(
                    &options,
                    &"height".into(),
                    &JsValue::from_f64(*height as f64),
                )
                .ok();
                js_sys::Reflect::set(&options, &"depth".into(), &JsValue::from_f64(*depth as f64))
                    .ok();
                args.push(&options);
                args.push(&self.scene_handle);

                js_sys::Reflect::construct(
                    &js_sys::eval("BABYLON.MeshBuilder.CreateBox").unwrap(),
                    &args,
                )
                .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Cylinder {
                radius,
                height,
                segments,
            } => {
                let args = Array::new();
                args.push(&"cylinder".into());

                let options = js_sys::Object::new();
                js_sys::Reflect::set(
                    &options,
                    &"diameter".into(),
                    &JsValue::from_f64(*radius as f64 * 2.0),
                )
                .ok();
                js_sys::Reflect::set(
                    &options,
                    &"height".into(),
                    &JsValue::from_f64(*height as f64),
                )
                .ok();
                js_sys::Reflect::set(
                    &options,
                    &"tessellation".into(),
                    &JsValue::from_f64(*segments as f64),
                )
                .ok();
                args.push(&options);
                args.push(&self.scene_handle);

                js_sys::Reflect::construct(
                    &js_sys::eval("BABYLON.MeshBuilder.CreateCylinder").unwrap(),
                    &args,
                )
                .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Plane { width, height } => {
                let args = Array::new();
                args.push(&"plane".into());

                let options = js_sys::Object::new();
                js_sys::Reflect::set(&options, &"width".into(), &JsValue::from_f64(*width as f64))
                    .ok();
                js_sys::Reflect::set(
                    &options,
                    &"height".into(),
                    &JsValue::from_f64(*height as f64),
                )
                .ok();
                args.push(&options);
                args.push(&self.scene_handle);

                js_sys::Reflect::construct(
                    &js_sys::eval("BABYLON.MeshBuilder.CreatePlane").unwrap(),
                    &args,
                )
                .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Line { points } => {
                let points_array = Array::new();
                for point in points {
                    let vec = js_sys::eval(&format!(
                        "new BABYLON.Vector3({}, {}, {})",
                        point[0], point[1], point[2]
                    ))
                    .unwrap();
                    points_array.push(&vec);
                }

                let args = Array::new();
                args.push(&"lines".into());

                let options = js_sys::Object::new();
                js_sys::Reflect::set(&options, &"points".into(), &points_array).ok();
                args.push(&options);
                args.push(&self.scene_handle);

                js_sys::Reflect::construct(
                    &js_sys::eval("BABYLON.MeshBuilder.CreateLines").unwrap(),
                    &args,
                )
                .map_err(|_| RenderError::GeometryCreation)?
            }

            _ => {
                // Fallback for unsupported geometries
                return Err(RenderError::InvalidGeometry(
                    "Geometry type not supported by Babylon backend".into(),
                ));
            }
        };

        Ok(mesh)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_material_js(&self, material: &RenderMaterial) -> RenderResult<JsValue> {
        use js_sys::Array;

        let args = Array::new();
        args.push(&"material".into());
        args.push(&self.scene_handle);

        let mat =
            js_sys::Reflect::construct(&js_sys::eval("BABYLON.StandardMaterial").unwrap(), &args)
                .map_err(|_| RenderError::MaterialCreation)?;

        // Diffuse color
        let color = js_sys::eval(&format!(
            "new BABYLON.Color3({}, {}, {})",
            material.color[0], material.color[1], material.color[2]
        ))
        .unwrap();
        js_sys::Reflect::set(&mat, &"diffuseColor".into(), &color).ok();

        // Alpha
        js_sys::Reflect::set(
            &mat,
            &"alpha".into(),
            &JsValue::from_f64(material.opacity as f64),
        )
        .ok();

        // Metallic/roughness (approximated)
        js_sys::Reflect::set(
            &mat,
            &"specularPower".into(),
            &JsValue::from_f64((1.0 - material.roughness) as f64 * 100.0),
        )
        .ok();

        Ok(mat)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn log_stub(&self, message: &str) {
        log::info!("[Babylon Stub] {}", message);
    }
}

impl RenderBackend for BabylonBackend {
    #[cfg(target_arch = "wasm32")]
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        transform: RenderTransform,
        material: RenderMaterial,
    ) -> RenderResult<u64> {
        let mesh = self.create_mesh_js(&geometry)?;
        let mat = self.create_material_js(&material)?;

        // Apply material
        js_sys::Reflect::set(&mesh, &"material".into(), &mat).ok();

        // Apply transform
        self.apply_transform_js(&mesh, &transform)?;

        let id = self.objects.len() as u64 + 1;
        self.objects.insert(
            id,
            BabylonObject {
                js_mesh: mesh,
                visible: true,
                highlighted: false,
            },
        );

        Ok(id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        _transform: RenderTransform,
        _material: RenderMaterial,
    ) -> RenderResult<u64> {
        self.log_stub(&format!("create_object: {:?}", geometry));
        let id = self.objects.len() as u64 + 1;
        Ok(id)
    }

    #[cfg(target_arch = "wasm32")]
    fn update_transform(&mut self, id: u64, transform: RenderTransform) -> RenderResult<()> {
        let obj = self
            .objects
            .get(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        self.apply_transform_js(&obj.js_mesh, &transform)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn update_transform(&mut self, id: u64, _transform: RenderTransform) -> RenderResult<()> {
        self.log_stub(&format!("update_transform: {}", id));
        Ok(())
    }

    fn update_material(&mut self, id: u64, _material: RenderMaterial) -> RenderResult<()> {
        if !self.objects.contains_key(&id) {
            return Err(RenderError::ObjectNotFound(id));
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.log_stub(&format!("update_material: {}", id));
        Ok(())
    }

    fn update_geometry(&mut self, id: u64, _geometry: RenderGeometry) -> RenderResult<()> {
        if !self.objects.contains_key(&id) {
            return Err(RenderError::ObjectNotFound(id));
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.log_stub(&format!("update_geometry: {}", id));
        Ok(())
    }

    fn set_visible(&mut self, id: u64, visible: bool) -> RenderResult<()> {
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.visible = visible;

        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Reflect::set(&obj.js_mesh, &"isVisible".into(), &JsValue::from(visible)).ok();
        }

        Ok(())
    }

    fn set_highlighted(&mut self, id: u64, highlighted: bool) -> RenderResult<()> {
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        obj.highlighted = highlighted;

        #[cfg(target_arch = "wasm32")]
        {
            // Add glow layer when highlighted
            if highlighted {
                let glow_color = js_sys::eval("new BABYLON.Color3(0.2, 0.4, 1.0)").unwrap();
                js_sys::Reflect::set(&obj.js_mesh, &"outlineColor".into(), &glow_color).ok();
                js_sys::Reflect::set(
                    &obj.js_mesh,
                    &"outlineWidth".into(),
                    &JsValue::from_f64(0.1),
                )
                .ok();
            } else {
                js_sys::Reflect::set(
                    &obj.js_mesh,
                    &"outlineWidth".into(),
                    &JsValue::from_f64(0.0),
                )
                .ok();
            }
        }

        Ok(())
    }

    fn remove_object(&mut self, id: u64) -> RenderResult<()> {
        let _obj = self
            .objects
            .remove(&id)
            .ok_or(RenderError::ObjectNotFound(id))?;

        #[cfg(target_arch = "wasm32")]
        {
            let dispose = js_sys::Reflect::get(&_obj.js_mesh, &"dispose".into()).unwrap();
            js_sys::Reflect::apply(
                &dispose.dyn_into::<js_sys::Function>().unwrap(),
                &_obj.js_mesh,
                &js_sys::Array::new(),
            )
            .ok();
        }

        Ok(())
    }

    fn clear_scene(&mut self) -> RenderResult<()> {
        let ids: Vec<u64> = self.objects.keys().copied().collect();
        for id in ids {
            self.remove_object(id)?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "babylon.js"
    }
}

#[cfg(target_arch = "wasm32")]
impl BabylonBackend {
    fn apply_transform_js(&self, mesh: &JsValue, transform: &RenderTransform) -> RenderResult<()> {
        // Babylon uses separate position, rotation, scaling
        // We need to decompose the matrix

        let matrix = transform.matrix;

        // Extract position (last column)
        let position = js_sys::eval(&format!(
            "new BABYLON.Vector3({}, {}, {})",
            matrix[12], matrix[13], matrix[14]
        ))
        .unwrap();
        js_sys::Reflect::set(mesh, &"position".into(), &position).ok();

        // Extract scale
        let sx = (matrix[0] * matrix[0] + matrix[1] * matrix[1] + matrix[2] * matrix[2]).sqrt();
        let sy = (matrix[4] * matrix[4] + matrix[5] * matrix[5] + matrix[6] * matrix[6]).sqrt();
        let sz = (matrix[8] * matrix[8] + matrix[9] * matrix[9] + matrix[10] * matrix[10]).sqrt();

        let scaling =
            js_sys::eval(&format!("new BABYLON.Vector3({}, {}, {})", sx, sy, sz)).unwrap();
        js_sys::Reflect::set(mesh, &"scaling".into(), &scaling).ok();

        // For rotation, we'd need to convert the rotation matrix to Euler/Quaternion
        // This is simplified - production code would do full decomposition

        Ok(())
    }
}
