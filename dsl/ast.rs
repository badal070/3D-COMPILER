/// Abstract Syntax Tree definitions.
/// Mirrors DSL structure exactly - no semantic interpretation.
/// Preserves source spans for excellent error reporting.

use crate::errors::SourceSpan;

/// Complete DSL file representation
#[derive(Debug, Clone)]
pub struct AstFile {
    pub scene: AstScene,
    pub library_imports: AstLibraryImports,
    pub materials: Vec<AstMaterial>,
    pub fields: Vec<AstFieldDef>,
    pub entities: Vec<AstEntity>,
    pub constraints: Vec<AstConstraint>,
    pub motions: Vec<AstMotion>,
    pub compound_motions: Vec<AstCompoundMotion>,
    pub trajectories: Vec<AstTrajectory>,
    pub timelines: Vec<AstTimeline>,
    pub span: SourceSpan,
}

/// Scene header
#[derive(Debug, Clone)]
pub struct AstScene {
    pub name: String,
    pub version: i64,
    pub ir_version: String,
    pub unit_system: String,
    pub span: SourceSpan,
}

/// Library imports section
#[derive(Debug, Clone)]
pub struct AstLibraryImports {
    pub imports: Vec<AstImport>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct AstImport {
    pub alias: String,
    pub library_name: String,
    pub span: SourceSpan,
}

/// Material definition (NEW)
#[derive(Debug, Clone)]
pub struct AstMaterial {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstMaterial {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// Field definition (NEW)
#[derive(Debug, Clone)]
pub struct AstFieldDef {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstFieldDef {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn field_type(&self) -> Option<&str> {
        self.get_field("type")
            .and_then(|f| f.value.as_identifier())
    }
}

/// Compound motion definition (NEW)
#[derive(Debug, Clone)]
pub struct AstCompoundMotion {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstCompoundMotion {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn motion_type(&self) -> Option<&str> {
        self.get_field("type")
            .and_then(|f| f.value.as_identifier())
    }

    pub fn motion_list(&self) -> Vec<String> {
        self.get_field("motions")
            .and_then(|f| {
                if let AstValue::String(s, _) = &f.value {
                    Some(s.split(',').map(|m| m.trim().to_string()).collect())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }
}

/// Trajectory definition (NEW)
#[derive(Debug, Clone)]
pub struct AstTrajectory {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstTrajectory {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn path_type(&self) -> Option<&str> {
        self.get_field("type")
            .and_then(|f| f.value.as_identifier())
    }

    pub fn target(&self) -> Option<&str> {
        self.get_field("target")
            .and_then(|f| f.value.as_identifier())
    }
}

/// Entity definition
#[derive(Debug, Clone)]
pub struct AstEntity {
    pub name: String,
    pub kind: String,
    pub components: Vec<AstComponent>,
    pub span: SourceSpan,
}

/// Component within an entity
#[derive(Debug, Clone)]
pub struct AstComponent {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

/// Field within a component or other block
#[derive(Debug, Clone)]
pub struct AstField {
    pub name: String,
    pub value: AstValue,
    pub span: SourceSpan,
}

/// Value types
#[derive(Debug, Clone)]
pub enum AstValue {
    Number(f64, SourceSpan),
    String(String, SourceSpan),
    Identifier(String, SourceSpan),
    Vector(Vec<f64>, SourceSpan),
}

impl AstValue {
    pub fn span(&self) -> SourceSpan {
        match self {
            AstValue::Number(_, span)
            | AstValue::String(_, span)
            | AstValue::Identifier(_, span)
            | AstValue::Vector(_, span) => *span,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            AstValue::Number(n, _) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            AstValue::String(s, _) => Some(s),
            _ => None,
        }
    }

    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            AstValue::Identifier(id, _) => Some(id),
            _ => None,
        }
    }

    pub fn as_vector(&self) -> Option<&[f64]> {
        match self {
            AstValue::Vector(vec, _) => Some(vec),
            _ => None,
        }
    }
}

/// Constraint definition
#[derive(Debug, Clone)]
pub struct AstConstraint {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstConstraint {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn constraint_type(&self) -> Option<&str> {
        self.get_field("type")
            .and_then(|f| f.value.as_identifier())
    }
}

/// Motion definition
#[derive(Debug, Clone)]
pub struct AstMotion {
    pub name: String,
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstMotion {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn target(&self) -> Option<&str> {
        self.get_field("target")
            .and_then(|f| f.value.as_identifier())
    }

    pub fn motion_type(&self) -> Option<&str> {
        self.get_field("type")
            .and_then(|f| f.value.as_identifier())
    }
}

/// Timeline definition
#[derive(Debug, Clone)]
pub struct AstTimeline {
    pub name: String,
    pub events: Vec<AstEvent>,
    pub span: SourceSpan,
}

/// Event within a timeline
#[derive(Debug, Clone)]
pub struct AstEvent {
    pub fields: Vec<AstField>,
    pub span: SourceSpan,
}

impl AstEvent {
    pub fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn motion(&self) -> Option<&str> {
        self.get_field("motion")
            .and_then(|f| f.value.as_identifier())
    }

    pub fn start(&self) -> Option<f64> {
        self.get_field("start")
            .and_then(|f| f.value.as_number())
    }

    pub fn duration(&self) -> Option<f64> {
        self.get_field("duration")
            .and_then(|f| f.value.as_number())
    }
}

/// Helper trait for field lookup
pub trait HasFields {
    fn get_field(&self, name: &str) -> Option<&AstField>;
    
    fn get_string_field(&self, name: &str) -> Option<&str> {
        self.get_field(name)
            .and_then(|f| f.value.as_string())
    }
    
    fn get_number_field(&self, name: &str) -> Option<f64> {
        self.get_field(name)
            .and_then(|f| f.value.as_number())
    }
    
    fn get_identifier_field(&self, name: &str) -> Option<&str> {
        self.get_field(name)
            .and_then(|f| f.value.as_identifier())
    }
    
    fn get_vector_field(&self, name: &str) -> Option<&[f64]> {
        self.get_field(name)
            .and_then(|f| f.value.as_vector())
    }
}

impl HasFields for AstComponent {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstConstraint {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstMotion {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstEvent {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstMaterial {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstFieldDef {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstCompoundMotion {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl HasFields for AstTrajectory {
    fn get_field(&self, name: &str) -> Option<&AstField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_accessors() {
        let span = SourceSpan::single_point(1, 1, 0);
        
        let num_val = AstValue::Number(42.0, span);
        assert_eq!(num_val.as_number(), Some(42.0));
        assert_eq!(num_val.as_string(), None);
        
        let str_val = AstValue::String("test".to_string(), span);
        assert_eq!(str_val.as_string(), Some("test"));
        assert_eq!(str_val.as_number(), None);
        
        let id_val = AstValue::Identifier("cube".to_string(), span);
        assert_eq!(id_val.as_identifier(), Some("cube"));
        
        let vec_val = AstValue::Vector(vec![1.0, 2.0, 3.0], span);
        assert_eq!(vec_val.as_vector(), Some(&[1.0, 2.0, 3.0][..]));
    }
}