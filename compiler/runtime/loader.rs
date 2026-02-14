// compiler/runtime/loader.rs
use crate::error::RuntimeResult;
use crate::state::world_state::{ActiveConstraint, ConstraintKind};
use crate::state::{
    ObjectKind, ObjectState, Parameter, ParameterKind, ParameterState, Quaternion, RuntimeState,
    TimeState, Vector3, WorldState,
};
/// Loads IR into executable runtime state
use dsl_compiler::lower_to_ir::{IrComponent, IrConstraint, IrEntity, IrScene, IrValue};

pub struct SceneLoader;

impl SceneLoader {
    /// Load complete IR scene into runtime state
    pub fn load_scene(ir: &IrScene) -> RuntimeResult<RuntimeState> {
        let mut world = WorldState::new();
        let time = TimeState::new();

        // Load entities
        for entity in &ir.entities {
            let object = Self::load_entity(entity)?;
            world.add_object(entity.id.clone(), object)?;
        }

        // Load parameters from metadata and constraints
        let mut params = ParameterState::new();
        if let Some(time_param) = Self::create_time_parameter() {
            params.add(time_param)?;
        }
        // Constraint-specific scalar parameters (e.g. gear ratio, distance)
        for constraint in &ir.constraints {
            Self::add_constraint_parameters(constraint, &mut params);
        }
        world.parameters = params;

        // Load constraints from IR into runtime world state
        for constraint in &ir.constraints {
            if let Some(active) = Self::load_constraint(constraint) {
                world.add_constraint(active);
            }
        }

        Ok(RuntimeState::new(world, time))
    }

    fn load_entity(entity: &IrEntity) -> RuntimeResult<ObjectState> {
        // Determine object kind
        let kind = match entity.kind.as_str() {
            "solid" => Self::infer_kind_from_geometry(entity),
            _ => ObjectKind::Custom,
        };

        let mut obj = ObjectState::new(entity.id.clone(), kind);

        // Load transform component
        if let Some(transform) = entity.components.get("transform") {
            obj = Self::load_transform(obj, transform)?;
        }

        // Load physical component
        if let Some(physical) = entity.components.get("physical") {
            obj = Self::load_physical(obj, physical)?;
        }

        obj.visible = true;
        Ok(obj)
    }

    fn infer_kind_from_geometry(entity: &IrEntity) -> ObjectKind {
        if let Some(geom) = entity.components.get("geometry") {
            if let Some(IrValue::Identifier(prim)) = geom.properties.get("primitive") {
                return match prim.as_str() {
                    "cube" => ObjectKind::Box,
                    "sphere" => ObjectKind::Sphere,
                    "cylinder" => ObjectKind::Cylinder,
                    "plane" => ObjectKind::Plane,
                    _ => ObjectKind::Custom,
                };
            }
        }
        ObjectKind::Custom
    }

    fn load_transform(mut obj: ObjectState, transform: &IrComponent) -> RuntimeResult<ObjectState> {
        // Load position
        if let Some(IrValue::Vector3(pos)) = transform.properties.get("position") {
            obj.position = Vector3::new(pos[0], pos[1], pos[2]);
        }

        // Load rotation (Euler angles in radians -> Quaternion)
        if let Some(IrValue::Vector3(rot)) = transform.properties.get("rotation") {
            obj.orientation = Self::euler_to_quaternion(rot[0], rot[1], rot[2]);
        }

        // Load scale
        if let Some(IrValue::Vector3(scale)) = transform.properties.get("scale") {
            obj.scale = Vector3::new(scale[0], scale[1], scale[2]);
        }

        Ok(obj)
    }

    fn load_physical(mut obj: ObjectState, physical: &IrComponent) -> RuntimeResult<ObjectState> {
        // Check if rigid/static
        if let Some(IrValue::Boolean(rigid)) = physical.properties.get("rigid") {
            if *rigid {
                obj = obj.make_static();
            }
        }

        Ok(obj)
    }

    /// Create parameter entries for constraint scalars so the solver can
    /// read them from `WorldState.parameters`.
    fn add_constraint_parameters(ir: &IrConstraint, params: &mut ParameterState) {
        match ir.constraint_type.as_str() {
            // Gear relation: scalar ratio
            "gear_relation" => {
                if let Some(IrValue::Number(ratio)) = ir.parameters.get("ratio") {
                    let id = format!("{}.ratio", ir.id);
                    let param = Parameter::new(id, *ratio).with_kind(ParameterKind::Scalar);
                    let _ = params.add(param);
                }
            }
            // Distance constraint: target distance
            "distance_constraint" => {
                if let Some(IrValue::Number(distance)) = ir.parameters.get("distance") {
                    let id = format!("{}.distance", ir.id);
                    let param = Parameter::new(id, *distance).with_kind(ParameterKind::Length);
                    let _ = params.add(param);
                }
            }
            _ => {}
        }
    }

    /// Map a DSL-level IR constraint into a runtime ActiveConstraint.
    ///
    /// This provides domain-specific wiring for known constraint types:
    /// - gear_relation: keeps two objects in a fixed rotational ratio
    /// - fixed_joint: parent/child rigid attachment
    /// - distance_constraint: maintains a fixed distance between two objects
    ///
    /// Unknown constraint types are ignored at runtime (they may still be
    /// used by offline tools or other backends).
    fn load_constraint(ir: &IrConstraint) -> Option<ActiveConstraint> {
        use std::collections::HashMap;

        // Helper to extract a string parameter
        fn get_str(params: &HashMap<String, IrValue>, key: &str) -> Option<String> {
            match params.get(key) {
                Some(IrValue::Identifier(s)) | Some(IrValue::String(s)) => Some(s.clone()),
                _ => None,
            }
        }

        match ir.constraint_type.as_str() {
            // Gear relation between two rotating entities
            "gear_relation" => {
                let driver = get_str(&ir.parameters, "driver")?;
                let driven = get_str(&ir.parameters, "driven")?;
                // Parameter id where the ratio was stored
                let ratio_param = format!("{}.ratio", ir.id);

                Some(ActiveConstraint {
                    id: ir.id.clone(),
                    kind: ConstraintKind::Angle,
                    objects: vec![driver, driven],
                    // Parameter names are referenced by the equation string below.
                    parameters: vec![ratio_param],
                    // Simple symbolic equation: angle(driven) - ratio * angle(driver) = 0
                    // The actual evaluation is implemented in the constraint solver.
                    equation: "angle(driven) - ratio * angle(driver)".to_string(),
                    priority: 0,
                    enabled: true,
                })
            }

            // Fixed joint between parent and child (no relative motion)
            "fixed_joint" => {
                let parent = get_str(&ir.parameters, "parent")?;
                let child = get_str(&ir.parameters, "child")?;

                Some(ActiveConstraint {
                    id: ir.id.clone(),
                    kind: ConstraintKind::Equality,
                    objects: vec![parent, child],
                    parameters: Vec::new(),
                    // Enforce zero relative transform: position/rotation of child
                    // matches parent in the chosen frame.
                    equation: "relative_transform(parent, child) = 0".to_string(),
                    priority: 0,
                    enabled: true,
                })
            }

            // Distance constraint between two objects
            "distance_constraint" => {
                let a = get_str(&ir.parameters, "entity_a")?;
                let b = get_str(&ir.parameters, "entity_b")?;
                // Parameter id where the distance was stored
                let distance_param = format!("{}.distance", ir.id);

                Some(ActiveConstraint {
                    id: ir.id.clone(),
                    kind: ConstraintKind::Distance,
                    objects: vec![a, b],
                    parameters: vec![distance_param],
                    // Enforce ||p_a - p_b|| - distance = 0
                    equation: "norm(pos(entity_a) - pos(entity_b)) - distance".to_string(),
                    priority: 0,
                    enabled: true,
                })
            }

            // Unknown or unsupported constraint type: ignore at runtime
            _ => None,
        }
    }

    fn euler_to_quaternion(roll: f64, pitch: f64, yaw: f64) -> Quaternion {
        let cr = (roll * 0.5).cos();
        let sr = (roll * 0.5).sin();
        let cp = (pitch * 0.5).cos();
        let sp = (pitch * 0.5).sin();
        let cy = (yaw * 0.5).cos();
        let sy = (yaw * 0.5).sin();

        Quaternion::new(
            cr * cp * cy + sr * sp * sy, // w
            sr * cp * cy - cr * sp * sy, // x
            cr * sp * cy + sr * cp * sy, // y
            cr * cp * sy - sr * sp * cy, // z
        )
    }

    fn create_time_parameter() -> Option<Parameter> {
        Some(Parameter::new("time".to_string(), 0.0).with_kind(crate::state::ParameterKind::Time))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_load_simple_entity() {
        // Test loading a basic cube entity
    }
}
