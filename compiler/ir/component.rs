use crate::ids::EntityId;
use crate::values::{
    AtomicNumber, BondOrder, Electronegativity, Scalar, Vector3,
};

#[derive(Debug, Clone)]
pub enum Component {
    Transform(Transform),
    Geometry(Geometry),
    Physical(Physical),
    Material(Material),
    MotionComponent(MotionComponent),
    PhysicsComponent(PhysicsComponent),
    ChemistryComponent(ChemistryComponent),
    RoboticsComponent(RoboticsComponent),
}

// Euler ONLY at IR level. Quaternions are renderer-level math.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vector3,
    pub rotation: Vector3,  // Euler angles in radians
    pub scale: Vector3,
}

impl Transform {
    pub fn new(position: Vector3, rotation: Vector3, scale: Vector3) -> Self {
        Self { position, rotation, scale }
    }

    pub fn identity() -> Self {
        Self {
            position: Vector3::zero(),
            rotation: Vector3::zero(),
            scale: Vector3::one(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Geometry {
    Primitive(Primitive),
    Procedural(ProceduralShape),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Cube,
    Cylinder,
    Sphere,
}

// No meshes. Ever.
#[derive(Debug, Clone)]
pub struct ProceduralShape {
    pub name: String,
    pub parameters: Vec<Scalar>,
}

impl ProceduralShape {
    pub fn new(name: String, parameters: Vec<Scalar>) -> Self {
        Self { name, parameters }
    }
}

// Optional because not all concepts are physical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Physical {
    pub mass: Option<f64>,
    pub rigid: bool,
}

impl Physical {
    pub fn new(mass: Option<f64>, rigid: bool) -> Self {
        Self { mass, rigid }
    }

    pub fn rigid_body(mass: f64) -> Self {
        Self {
            mass: Some(mass),
            rigid: true,
        }
    }

    pub fn kinematic() -> Self {
        Self {
            mass: None,
            rigid: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub color: Vector3,
    pub metallic: f64,
    pub roughness: f64,
    pub opacity: f64,
    pub emissive: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionComponent {
    pub velocity: Vector3,
    pub acceleration: Vector3,
    pub angular_velocity: Vector3,
    pub angular_acceleration: Vector3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsComponent {
    pub rigid_body: Option<RigidBodyData>,
    pub spring: Option<SpringData>,
    pub pendulum: Option<PendulumData>,
    pub collision_data: Option<CollisionData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChemistryComponent {
    pub atom: Option<AtomData>,
    pub bond: Option<BondData>,
    pub molecule: Option<MoleculeData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoboticsComponent {
    pub joint: Option<JointData>,
    pub link: Option<LinkData>,
    pub kinematic_chain: Option<KinematicChainData>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidBodyData {
    pub mass: f64,
    pub is_kinematic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringData {
    pub rest_length: f64,
    pub spring_constant: f64,
    pub damping: f64,
    pub point_a: Vector3,
    pub point_b: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendulumData {
    pub length: f64,
    pub pivot_point: Vector3,
    pub gravity_direction: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionData {
    pub restitution: f64,
    pub friction: f64,
    pub collision_normal: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtomData {
    pub atomic_number: AtomicNumber,
    pub position: Vector3,
    pub electronegativity: Option<Electronegativity>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondData {
    pub atom_a: EntityId,
    pub atom_b: EntityId,
    pub bond_order: BondOrder,
    pub equilibrium_length: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoleculeData {
    pub atoms: Vec<EntityId>,
    pub bonds: Vec<BondData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointData {
    pub name: String,
    pub min_position: f64,
    pub max_position: f64,
    pub max_velocity: f64,
    pub max_effort: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub name: String,
    pub length: f64,
    pub mass: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KinematicChainData {
    pub joints: Vec<EntityId>,
    pub end_effector: EntityId,
}
