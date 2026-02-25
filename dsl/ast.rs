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
    pub math_objects: Vec<MathObjectNode>,
    pub compound_motions: Vec<AstCompoundMotion>,
    pub trajectories: Vec<AstTrajectory>,
    pub timelines: Vec<AstTimeline>,
    pub concept_ref: Option<ConceptAnnotation>,
    pub annotations: Vec<AnnotationNode>,
    pub highlight_schedule: Vec<HighlightScheduleEntry>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ConceptAnnotation {
    pub concept_id: String,
    pub section_id: String,
    pub step_index: usize,
}

#[derive(Debug, Clone)]
pub struct AnnotationNode {
    pub label_text: String,
    pub anchor_entity_id: String,
    pub position_offset: [f64; 3],
    pub equation_node_id: Option<String>,
    pub highlight_token: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct HighlightScheduleEntry {
    pub at_time: f64,
    pub highlight_token: String,
    pub entity_id: String,
    pub color_index: u8,
    pub span: SourceSpan,
}

/// Scene header
#[derive(Debug, Clone)]
pub struct AstScene {
    pub name: String,
    pub version: i64,
    pub ir_version: String,
    pub unit_system: String,
    pub domain: Option<String>,
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
        self.get_field("type").and_then(|f| f.value.as_identifier())
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
        self.get_field("type").and_then(|f| f.value.as_identifier())
    }

    pub fn motion_list(&self) -> Vec<String> {
        self.get_field("motions")
            .and_then(|f| f.value.as_list())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| match value {
                        AstValue::Identifier(id, _) => Some(id.clone()),
                        AstValue::String(val, _) => Some(val.clone()),
                        _ => None,
                    })
                    .collect()
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
        self.get_field("type").and_then(|f| f.value.as_identifier())
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
    Boolean(bool, SourceSpan),
    Identifier(String, SourceSpan),
    List(Vec<AstValue>, SourceSpan),
    Vector(Vec<f64>, SourceSpan),
    Matrix(Vec<Vec<f64>>, SourceSpan),
    MathExpression(AnnotatedExpr, SourceSpan),
}

impl AstValue {
    pub fn span(&self) -> SourceSpan {
        match self {
            AstValue::Number(_, span)
            | AstValue::String(_, span)
            | AstValue::Boolean(_, span)
            | AstValue::Identifier(_, span)
            | AstValue::List(_, span)
            | AstValue::Vector(_, span)
            | AstValue::Matrix(_, span)
            | AstValue::MathExpression(_, span) => *span,
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

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            AstValue::Boolean(v, _) => Some(*v),
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

    pub fn as_list(&self) -> Option<&[AstValue]> {
        match self {
            AstValue::List(values, _) => Some(values),
            _ => None,
        }
    }

    pub fn as_matrix(&self) -> Option<&[Vec<f64>]> {
        match self {
            AstValue::Matrix(matrix, _) => Some(matrix),
            _ => None,
        }
    }
}

/// Mathematical object AST nodes (Step 4: AST extension before parser wiring)
#[derive(Debug, Clone)]
pub enum MathObjectNode {
    Function(FunctionNode),
    Curve(CurveNode),
    Surface(SurfaceNode),
    VectorField(VectorFieldNode),
    ScalarField(ScalarFieldNode),
    Transformation(TransformationNode),
    DifferentialEquation(DifferentialEquationNode),
    MatrixDefinition(MatrixDefinitionNode),
}

#[derive(Debug, Clone)]
pub struct AnnotatedExpr {
    pub node_id: String,
    pub highlight_token: Option<String>,
    pub expr: MathExpression,
}

#[derive(Debug, Clone)]
pub enum MathExpression {
    Variable(String),
    Constant(MathConstant),
    Number(f64),
    ComplexNumber {
        real: f64,
        imag: f64,
    },
    BinaryOp(Box<AnnotatedExpr>, MathBinaryOperator, Box<AnnotatedExpr>),
    UnaryOp(MathUnaryOperator, Box<AnnotatedExpr>),
    FunctionCall(String, Vec<AnnotatedExpr>),
    Derivative {
        expression: Box<AnnotatedExpr>,
        variable: String,
        order: usize,
    },
    Integral {
        expression: Box<AnnotatedExpr>,
        variable: String,
        bounds: Option<Box<IntervalConstraint>>,
    },
    Limit {
        expression: Box<AnnotatedExpr>,
        variable: String,
        approach: f64,
    },
    Summation {
        expression: Box<AnnotatedExpr>,
        variable: String,
        bounds: IntervalConstraint,
    },
    Product {
        expression: Box<AnnotatedExpr>,
        variable: String,
        bounds: IntervalConstraint,
    },
    Piecewise(Vec<(MathCondition, AnnotatedExpr)>),
    MatrixExpr(Vec<Vec<AnnotatedExpr>>),
    VectorExpr(Vec<AnnotatedExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathConstant {
    Pi,
    Euler,
    ImaginaryUnit,
    Infinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Dot,
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathUnaryOperator {
    Negate,
    Gradient,
    Divergence,
    Curl,
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: AnnotatedExpr,
    pub domain: DomainConstraint,
    pub range: Option<RangeConstraint>,
    pub properties: FunctionProperties,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct CurveNode {
    pub name: String,
    pub curve_type: CurveType,
    pub definition: AnnotatedExpr,
    pub parameter: Option<String>,
    pub domain: DomainConstraint,
    pub properties: CurveProperties,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct SurfaceNode {
    pub name: String,
    pub surface_type: SurfaceType,
    pub definition: AnnotatedExpr,
    pub parameters: Option<(String, String)>,
    pub domain: DomainConstraint,
    pub properties: SurfaceProperties,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct VectorFieldNode {
    pub name: String,
    pub components: Vec<AnnotatedExpr>,
    pub dimension: usize,
    pub domain: DomainConstraint,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ScalarFieldNode {
    pub name: String,
    pub expression: AnnotatedExpr,
    pub domain: DomainConstraint,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct TransformationNode {
    pub name: String,
    pub transform_type: TransformationType,
    pub expression: AnnotatedExpr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct DifferentialEquationNode {
    pub name: String,
    pub equation_type: DifferentialEquationType,
    pub order: usize,
    pub equation: AnnotatedExpr,
    pub initial_conditions: Vec<MathCondition>,
    pub boundary_conditions: Vec<MathCondition>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct MatrixDefinitionNode {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub elements: Vec<Vec<AnnotatedExpr>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    Explicit,
    Implicit,
    Parametric,
    Polar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceType {
    Explicit,
    Implicit,
    Parametric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformationType {
    Linear,
    Affine,
    NonLinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialEquationType {
    Ode,
    Pde,
}

#[derive(Debug, Clone)]
pub struct FunctionProperties {
    pub continuous: bool,
    pub differentiable_order: Option<usize>,
    pub periodic: Option<f64>,
    pub symmetric: Option<SymmetryType>,
    pub monotonic: Option<MonotonicityType>,
}

#[derive(Debug, Clone)]
pub struct CurveProperties {
    pub closed: bool,
    pub smooth: bool,
}

#[derive(Debug, Clone)]
pub struct SurfaceProperties {
    pub orientable: bool,
    pub closed: bool,
    pub genus: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetryType {
    Even,
    Odd,
    Axis,
    Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicityType {
    Increasing,
    Decreasing,
    NonMonotonic,
}

#[derive(Debug, Clone)]
pub struct DomainConstraint {
    pub variables: Vec<String>,
    pub constraints: Vec<MathConstraint>,
    pub space: MathSpace,
}

#[derive(Debug, Clone)]
pub struct RangeConstraint {
    pub constraints: Vec<MathConstraint>,
}

#[derive(Debug, Clone)]
pub struct MathConstraint {
    pub expression: AnnotatedExpr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathSpace {
    Real,
    Real2,
    Real3,
    Complex,
}

#[derive(Debug, Clone)]
pub struct IntervalConstraint {
    pub lower: Box<AnnotatedExpr>,
    pub upper: Box<AnnotatedExpr>,
    pub lower_inclusive: bool,
    pub upper_inclusive: bool,
}

#[derive(Debug, Clone)]
pub struct MathCondition {
    pub expression: AnnotatedExpr,
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
        self.get_field("type").and_then(|f| f.value.as_identifier())
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
        self.get_field("type").and_then(|f| f.value.as_identifier())
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
        self.get_field("start").and_then(|f| f.value.as_number())
    }

    pub fn duration(&self) -> Option<f64> {
        self.get_field("duration").and_then(|f| f.value.as_number())
    }
}

/// Helper trait for field lookup
pub trait HasFields {
    fn get_field(&self, name: &str) -> Option<&AstField>;

    fn get_string_field(&self, name: &str) -> Option<&str> {
        self.get_field(name).and_then(|f| f.value.as_string())
    }

    fn get_number_field(&self, name: &str) -> Option<f64> {
        self.get_field(name).and_then(|f| f.value.as_number())
    }

    fn get_identifier_field(&self, name: &str) -> Option<&str> {
        self.get_field(name).and_then(|f| f.value.as_identifier())
    }

    fn get_vector_field(&self, name: &str) -> Option<&[f64]> {
        self.get_field(name).and_then(|f| f.value.as_vector())
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
