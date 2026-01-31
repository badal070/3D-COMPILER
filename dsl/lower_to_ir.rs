/// Lowering pass from validated AST to IR.
/// This is where DSL constructs become IR constructs.
/// 1:1 mapping, no semantic interpretation.
/// Pure function - validated AST in, IR out.

use crate::ast::*;
use crate::errors::{DslError, DslResult, ErrorCode};
use serde::Serialize;
use std::collections::HashMap;

/// Intermediate Representation - this would typically be defined in a separate IR module
/// For now, we define a minimal IR structure to demonstrate the lowering process

#[derive(Serialize)]
pub struct IrScene {
    pub metadata: IrMetadata,
    pub entities: Vec<IrEntity>,
    pub constraints: Vec<IrConstraint>,
    pub motions: Vec<IrMotion>,
    pub timelines: Vec<IrTimeline>,
     
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMetadata {
    pub name: String,
    pub version: i64,
    pub ir_version: String,
    pub unit_system: String,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEntity {
    pub id: String,
    pub kind: String,
    pub components: HashMap<String, IrComponent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrComponent {
    pub component_type: String,
    pub properties: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub enum IrValue {
    Number(f64),
    String(String),
    Identifier(String),
    Vector3([f64; 3]),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize)]
pub struct IrConstraint {
    pub id: String,
    pub constraint_type: String,
    pub parameters: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMotion {
    pub id: String,
    pub motion_type: String,
    pub target_entity: String,
    pub parameters: HashMap<String, IrValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrTimeline {
    pub id: String,
    pub events: Vec<IrEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrEvent {
    pub motion_id: String,
    pub start_time: f64,
    pub duration: f64,
}

/// Lowers a validated AST to IR
pub struct IrLowering;

impl IrLowering {
    pub fn lower(ast: AstFile) -> DslResult<IrScene> {
        let metadata = Self::lower_metadata(&ast.scene, &ast.library_imports);
        let entities = Self::lower_entities(ast.entities)?;
        let constraints = Self::lower_constraints(ast.constraints)?;
        let motions = Self::lower_motions(ast.motions)?;
        let timelines = Self::lower_timelines(ast.timelines)?;

        Ok(IrScene {
            metadata,
            entities,
            constraints,
            motions,
            timelines,
        })
    }

    fn lower_metadata(scene: &AstScene, imports: &AstLibraryImports) -> IrMetadata {
        IrMetadata {
            name: scene.name.clone(),
            version: scene.version,
            ir_version: scene.ir_version.clone(),
            unit_system: scene.unit_system.clone(),
            libraries: imports.imports.iter().map(|i| i.library_name.clone()).collect(),
        }
    }

    fn lower_entities(entities: Vec<AstEntity>) -> DslResult<Vec<IrEntity>> {
        entities
            .into_iter()
            .map(|entity| {
                let mut components_map = HashMap::new();

                for component in entity.components {
                    let mut properties = HashMap::new();

                    for field in component.fields {
                        let value = Self::lower_value(field.value)?;
                        properties.insert(field.name, value);
                    }

                    let ir_component = IrComponent {
                        component_type: component.name.clone(),
                        properties,
                    };

                    components_map.insert(component.name, ir_component);
                }

                Ok(IrEntity {
                    id: entity.name,
                    kind: entity.kind,
                    components: components_map,
                })
            })
            .collect()
    }

    fn lower_constraints(constraints: Vec<AstConstraint>) -> DslResult<Vec<IrConstraint>> {
        constraints
            .into_iter()
            .map(|constraint| {
                let mut parameters = HashMap::new();
                let mut constraint_type = String::new();

                for field in constraint.fields {
                    if field.name == "type" {
                        if let AstValue::Identifier(type_name, _) = field.value {
                            constraint_type = type_name;
                        }
                    } else {
                        let value = Self::lower_value(field.value)?;
                        parameters.insert(field.name, value);
                    }
                }

                Ok(IrConstraint {
                    id: constraint.name,
                    constraint_type,
                    parameters,
                })
            })
            .collect()
    }

    fn lower_motions(motions: Vec<AstMotion>) -> DslResult<Vec<IrMotion>> {
        motions
            .into_iter()
            .map(|motion| {
                let mut parameters = HashMap::new();
                let mut motion_type = String::new();
                let mut target_entity = String::new();

                for field in motion.fields {
                    match field.name.as_str() {
                        "type" => {
                            if let AstValue::Identifier(type_name, _) = field.value {
                                motion_type = type_name;
                            }
                        }
                        "target" => {
                            if let AstValue::Identifier(target, _) = field.value {
                                target_entity = target;
                            }
                        }
                        _ => {
                            let value = Self::lower_value(field.value)?;
                            parameters.insert(field.name, value);
                        }
                    }
                }

                Ok(IrMotion {
                    id: motion.name,
                    motion_type,
                    target_entity,
                    parameters,
                })
            })
            .collect()
    }

    fn lower_timelines(timelines: Vec<AstTimeline>) -> DslResult<Vec<IrTimeline>> {
        timelines
            .into_iter()
            .map(|timeline| {
                let events = timeline
                    .events
                    .into_iter()
                    .filter_map(|event| {
                        let motion_id = event.motion()?.to_string();
                        let start_time = event.start()?;
                        let duration = event.duration()?;

                        Some(IrEvent {
                            motion_id,
                            start_time,
                            duration,
                        })
                    })
                    .collect();

                Ok(IrTimeline {
                    id: timeline.name,
                    events,
                })
            })
            .collect()
    }

    fn lower_value(value: AstValue) -> DslResult<IrValue> {
        match value {
            AstValue::Number(n, _) => Ok(IrValue::Number(n)),
            AstValue::String(s, _) => Ok(IrValue::String(s)),
            AstValue::Identifier(id, _) => {
                // Handle boolean identifiers
                match id.as_str() {
                    "true" => Ok(IrValue::Boolean(true)),
                    "false" => Ok(IrValue::Boolean(false)),
                    _ => Ok(IrValue::Identifier(id)),
                }
            }
            AstValue::Vector(vec, span) => {
                if vec.len() != 3 {
                    return Err(DslError::new(
                        ErrorCode::InvalidVectorLength,
                        format!("Expected 3D vector, found {} components", vec.len()),
                        span,
                        std::path::PathBuf::from("lowering"),
                    ));
                }
                Ok(IrValue::Vector3([vec[0], vec[1], vec[2]]))
            }
        }
    }
}

/// IR serialization helper (for debugging/output)
impl IrScene {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "metadata": {
                "name": self.metadata.name,
                "version": self.metadata.version,
                "ir_version": self.metadata.ir_version,
                "unit_system": self.metadata.unit_system,
                "libraries": self.metadata.libraries,
            },
            "entities": self.entities.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "kind": e.kind,
                    "components": e.components.iter().map(|(name, comp)| {
                        (name.clone(), serde_json::json!({
                            "type": comp.component_type,
                            "properties": comp.properties.iter().map(|(k, v)| {
                                (k.clone(), Self::value_to_json(v))
                            }).collect::<serde_json::Map<_, _>>(),
                        }))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "constraints": self.constraints.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "type": c.constraint_type,
                    "parameters": c.parameters.iter().map(|(k, v)| {
                        (k.clone(), Self::value_to_json(v))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "motions": self.motions.iter().map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "type": m.motion_type,
                    "target": m.target_entity,
                    "parameters": m.parameters.iter().map(|(k, v)| {
                        (k.clone(), Self::value_to_json(v))
                    }).collect::<serde_json::Map<_, _>>(),
                })
            }).collect::<Vec<_>>(),
            "timelines": self.timelines.iter().map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "events": t.events.iter().map(|e| {
                        serde_json::json!({
                            "motion": e.motion_id,
                            "start": e.start_time,
                            "duration": e.duration,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn value_to_json(value: &IrValue) -> serde_json::Value {
        match value {
            IrValue::Number(n) => serde_json::json!(n),
            IrValue::String(s) => serde_json::json!(s),
            IrValue::Identifier(id) => serde_json::json!(id),
            IrValue::Vector3(v) => serde_json::json!(v),
            IrValue::Boolean(b) => serde_json::json!(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::SourceSpan;

    #[test]
    fn test_value_lowering() {
        let span = SourceSpan::single_point(1, 1, 0);
        
        let num = AstValue::Number(42.0, span);
        let ir_num = IrLowering::lower_value(num).unwrap();
        assert!(matches!(ir_num, IrValue::Number(42.0)));
        
        let vec = AstValue::Vector(vec![1.0, 2.0, 3.0], span);
        let ir_vec = IrLowering::lower_value(vec).unwrap();
        assert!(matches!(ir_vec, IrValue::Vector3([1.0, 2.0, 3.0])));
        
        let bool_true = AstValue::Identifier("true".to_string(), span);
        let ir_bool = IrLowering::lower_value(bool_true).unwrap();
        assert!(matches!(ir_bool, IrValue::Boolean(true)));
    }

    #[test]
    fn test_invalid_vector_length() {
        let span = SourceSpan::single_point(1, 1, 0);
        let vec = AstValue::Vector(vec![1.0, 2.0], span);
        let result = IrLowering::lower_value(vec);
        assert!(result.is_err());
    }
}