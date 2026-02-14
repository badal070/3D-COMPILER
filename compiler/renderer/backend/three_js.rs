//! Three.js Backend
//!
//! Web-first, lightweight, WASM-friendly rendering.
//!
//! This backend communicates with JavaScript Three.js via WebAssembly.

use crate::renderer::{
    backend::{RenderBackend, RenderGeometry, RenderMaterial, RenderTransform},
    error::{RenderError, RenderResult},
};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Three.js rendering backend
///
/// Bridges Rust render commands to JavaScript Three.js library
pub struct ThreeJsBackend {
    objects: HashMap<u64, ThreeJsObject>,
    #[cfg(target_arch = "wasm32")]
    scene_handle: JsValue,
}

struct ThreeJsObject {
    #[cfg(target_arch = "wasm32")]
    js_object: JsValue,
    visible: bool,
    highlighted: bool,
}

impl ThreeJsBackend {
    /// Create new Three.js backend
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> RenderResult<Self> {
        let scene_handle = Self::init_scene()?;
        Ok(Self {
            objects: HashMap::new(),
            scene_handle,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> RenderResult<Self> {
        Ok(Self {
            objects: HashMap::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn init_scene() -> RenderResult<JsValue> {
        // Call JavaScript to initialize Three.js scene
        match js_sys::eval("window.initThreeScene()") {
            Ok(scene) => Ok(scene),
            Err(_) => Err(RenderError::BackendInit(
                "Failed to initialize Three.js scene".into(),
            )),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn create_geometry_js(&self, geometry: &RenderGeometry) -> RenderResult<JsValue> {
        use js_sys::{Array, Float32Array, Uint32Array};

        let geo = match geometry {
            RenderGeometry::Sphere { radius, segments } => {
                let args = Array::new();
                args.push(&JsValue::from_f64(*radius as f64));
                args.push(&JsValue::from_f64(*segments as f64));
                args.push(&JsValue::from_f64(*segments as f64));

                js_sys::Reflect::construct(&js_sys::eval("THREE.SphereGeometry").unwrap(), &args)
                    .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Box {
                width,
                height,
                depth,
            } => {
                let args = Array::new();
                args.push(&JsValue::from_f64(*width as f64));
                args.push(&JsValue::from_f64(*height as f64));
                args.push(&JsValue::from_f64(*depth as f64));

                js_sys::Reflect::construct(&js_sys::eval("THREE.BoxGeometry").unwrap(), &args)
                    .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Cylinder {
                radius,
                height,
                segments,
            } => {
                let args = Array::new();
                args.push(&JsValue::from_f64(*radius as f64));
                args.push(&JsValue::from_f64(*radius as f64));
                args.push(&JsValue::from_f64(*height as f64));
                args.push(&JsValue::from_f64(*segments as f64));

                js_sys::Reflect::construct(&js_sys::eval("THREE.CylinderGeometry").unwrap(), &args)
                    .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Cone {
                radius,
                height,
                segments,
            } => {
                let args = Array::new();
                args.push(&JsValue::from_f64(*radius as f64));
                args.push(&JsValue::from_f64(*height as f64));
                args.push(&JsValue::from_f64(*segments as f64));

                js_sys::Reflect::construct(&js_sys::eval("THREE.ConeGeometry").unwrap(), &args)
                    .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Plane { width, height } => {
                let args = Array::new();
                args.push(&JsValue::from_f64(*width as f64));
                args.push(&JsValue::from_f64(*height as f64));

                js_sys::Reflect::construct(&js_sys::eval("THREE.PlaneGeometry").unwrap(), &args)
                    .map_err(|_| RenderError::GeometryCreation)?
            }

            RenderGeometry::Line { points } => {
                let positions = Float32Array::new_with_length(points.len() as u32 * 3);
                for (i, point) in points.iter().enumerate() {
                    positions.set_index(i as u32 * 3, point[0]);
                    positions.set_index(i as u32 * 3 + 1, point[1]);
                    positions.set_index(i as u32 * 3 + 2, point[2]);
                }

                let geometry = js_sys::Reflect::construct(
                    &js_sys::eval("THREE.BufferGeometry").unwrap(),
                    &Array::new(),
                )
                .map_err(|_| RenderError::GeometryCreation)?;

                // Set positions attribute
                let attr_args = Array::new();
                attr_args.push(&positions);
                attr_args.push(&JsValue::from_f64(3.0));

                let attribute = js_sys::Reflect::construct(
                    &js_sys::eval("THREE.Float32BufferAttribute").unwrap(),
                    &attr_args,
                )
                .map_err(|_| RenderError::GeometryCreation)?;

                js_sys::Reflect::set(&geometry, &"position".into(), &attribute)
                    .map_err(|_| RenderError::GeometryCreation)?;

                geometry
            }

            RenderGeometry::Mesh { vertices, indices } => {
                let positions = Float32Array::new_with_length(vertices.len() as u32 * 3);
                for (i, vertex) in vertices.iter().enumerate() {
                    positions.set_index(i as u32 * 3, vertex[0]);
                    positions.set_index(i as u32 * 3 + 1, vertex[1]);
                    positions.set_index(i as u32 * 3 + 2, vertex[2]);
                }

                let idx = Uint32Array::from(indices.as_slice());

                let geometry = js_sys::Reflect::construct(
                    &js_sys::eval("THREE.BufferGeometry").unwrap(),
                    &Array::new(),
                )
                .map_err(|_| RenderError::GeometryCreation)?;

                // Set positions
                let pos_attr_args = Array::new();
                pos_attr_args.push(&positions);
                pos_attr_args.push(&JsValue::from_f64(3.0));

                let pos_attribute = js_sys::Reflect::construct(
                    &js_sys::eval("THREE.Float32BufferAttribute").unwrap(),
                    &pos_attr_args,
                )
                .map_err(|_| RenderError::GeometryCreation)?;

                js_sys::Reflect::set(&geometry, &"position".into(), &pos_attribute)
                    .map_err(|_| RenderError::GeometryCreation)?;

                // Set indices
                js_sys::Reflect::set(&geometry, &"index".into(), &idx)
                    .map_err(|_| RenderError::GeometryCreation)?;

                // Compute normals
                let compute_normals =
                    js_sys::Reflect::get(&geometry, &"computeVertexNormals".into())
                        .map_err(|_| RenderError::GeometryCreation)?;

                js_sys::Reflect::apply(
                    &compute_normals.dyn_into::<js_sys::Function>().unwrap(),
                    &geometry,
                    &Array::new(),
                )
                .map_err(|_| RenderError::GeometryCreation)?;

                geometry
            }
        };

        Ok(geo)
    }

    #[cfg(target_arch = "wasm32")]
    fn create_material_js(&self, material: &RenderMaterial) -> RenderResult<JsValue> {
        let mat_obj = js_sys::Object::new();

        // Color
        let color = js_sys::Number::from(
            ((material.color[0] * 255.0) as u32) << 16
                | ((material.color[1] * 255.0) as u32) << 8
                | ((material.color[2] * 255.0) as u32),
        );
        js_sys::Reflect::set(&mat_obj, &"color".into(), &color)
            .map_err(|_| RenderError::MaterialCreation)?;

        // Metalness
        js_sys::Reflect::set(
            &mat_obj,
            &"metalness".into(),
            &JsValue::from_f64(material.metallic as f64),
        )
        .map_err(|_| RenderError::MaterialCreation)?;

        // Roughness
        js_sys::Reflect::set(
            &mat_obj,
            &"roughness".into(),
            &JsValue::from_f64(material.roughness as f64),
        )
        .map_err(|_| RenderError::MaterialCreation)?;

        // Opacity
        js_sys::Reflect::set(
            &mat_obj,
            &"opacity".into(),
            &JsValue::from_f64(material.opacity as f64),
        )
        .map_err(|_| RenderError::MaterialCreation)?;

        // Transparent if opacity < 1
        if material.opacity < 1.0 {
            js_sys::Reflect::set(&mat_obj, &"transparent".into(), &JsValue::TRUE)
                .map_err(|_| RenderError::MaterialCreation)?;
        }

        // Create material
        let mat_args = Array::new();
        mat_args.push(&mat_obj);

        let mat = js_sys::Reflect::construct(
            &js_sys::eval("THREE.MeshStandardMaterial").unwrap(),
            &mat_args,
        )
        .map_err(|_| RenderError::MaterialCreation)?;

        Ok(mat)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn log_stub(&self, message: &str) {
        log::info!("[ThreeJS Stub] {}", message);
    }
}

impl RenderBackend for ThreeJsBackend {
    #[cfg(target_arch = "wasm32")]
    fn create_object(
        &mut self,
        geometry: RenderGeometry,
        transform: RenderTransform,
        material: RenderMaterial,
    ) -> RenderResult<u64> {
        let geo = self.create_geometry_js(&geometry)?;
        let mat = self.create_material_js(&material)?;

        // Create mesh
        let args = Array::new();
        args.push(&geo);
        args.push(&mat);

        let mesh = js_sys::Reflect::construct(&js_sys::eval("THREE.Mesh").unwrap(), &args)
            .map_err(|_| RenderError::ObjectCreation)?;

        // Apply transform
        self.apply_transform_js(&mesh, &transform)?;

        // Add to scene
        let add_fn = js_sys::Reflect::get(&self.scene_handle, &"add".into())
            .map_err(|_| RenderError::ObjectCreation)?;

        let add_args = Array::new();
        add_args.push(&mesh);

        js_sys::Reflect::apply(
            &add_fn.dyn_into::<js_sys::Function>().unwrap(),
            &self.scene_handle,
            &add_args,
        )
        .map_err(|_| RenderError::ObjectCreation)?;

        let id = self.objects.len() as u64 + 1;
        self.objects.insert(
            id,
            ThreeJsObject {
                js_object: mesh,
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

        self.apply_transform_js(&obj.js_object, &transform)?;
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
            js_sys::Reflect::set(&obj.js_object, &"visible".into(), &JsValue::from(visible))
                .map_err(|_| RenderError::UpdateFailed)?;
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
            // Add emissive glow when highlighted
            let material = js_sys::Reflect::get(&obj.js_object, &"material".into())
                .map_err(|_| RenderError::UpdateFailed)?;

            let emissive_value = if highlighted { 0x3366ff } else { 0x000000 };

            js_sys::Reflect::set(
                &material,
                &"emissive".into(),
                &js_sys::Number::from(emissive_value),
            )
            .map_err(|_| RenderError::UpdateFailed)?;
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
            let remove_fn = js_sys::Reflect::get(&self.scene_handle, &"remove".into())
                .map_err(|_| RenderError::UpdateFailed)?;

            let args = Array::new();
            args.push(&_obj.js_object);

            js_sys::Reflect::apply(
                &remove_fn.dyn_into::<js_sys::Function>().unwrap(),
                &self.scene_handle,
                &args,
            )
            .map_err(|_| RenderError::UpdateFailed)?;
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
        "three.js"
    }
}

#[cfg(target_arch = "wasm32")]
impl ThreeJsBackend {
    fn apply_transform_js(
        &self,
        object: &JsValue,
        transform: &RenderTransform,
    ) -> RenderResult<()> {
        let matrix = js_sys::Reflect::get(object, &"matrix".into())
            .map_err(|_| RenderError::UpdateFailed)?;

        // Set matrix elements
        let from_array = js_sys::Reflect::get(&matrix, &"fromArray".into())
            .map_err(|_| RenderError::UpdateFailed)?;

        let matrix_array = js_sys::Float32Array::from(transform.matrix.as_slice());
        let args = Array::new();
        args.push(&matrix_array);

        js_sys::Reflect::apply(
            &from_array.dyn_into::<js_sys::Function>().unwrap(),
            &matrix,
            &args,
        )
        .map_err(|_| RenderError::UpdateFailed)?;

        // Decompose matrix to update position/rotation/scale
        let decompose = js_sys::Reflect::get(object, &"matrixAutoUpdate".into())
            .map_err(|_| RenderError::UpdateFailed)?;

        js_sys::Reflect::set(object, &"matrixAutoUpdate".into(), &JsValue::FALSE)
            .map_err(|_| RenderError::UpdateFailed)?;

        Ok(())
    }
}
