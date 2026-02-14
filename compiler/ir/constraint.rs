use crate::ids::EntityId;
use crate::values::{Angle, BondOrder, Vector3};

#[derive(Debug, Clone)]
pub enum Constraint {
    FixedAxis {
        axis: Vector3,
    },
    GearRelation {
        driver: EntityId,
        driven: EntityId,
        ratio: f64,
    },
    ParentChild {
        parent: EntityId,
        child: EntityId,
    },
    Distance {
        entity_a: EntityId,
        entity_b: EntityId,
        distance: f64,
    },
    Spring(SpringConstraint),
    Pendulum(PendulumConstraint),
    Collision(CollisionConstraint),
    ChemicalBond(ChemicalBondConstraint),
    BondAngle(BondAngleConstraint),
    JointLimit(JointLimitConstraint),
    KinematicChain(KinematicChainConstraint),
}

impl Constraint {
    pub fn fixed_axis(axis: Vector3) -> Self {
        Constraint::FixedAxis { axis }
    }

    pub fn gear_relation(driver: EntityId, driven: EntityId, ratio: f64) -> Self {
        Constraint::GearRelation { driver, driven, ratio }
    }

    pub fn parent_child(parent: EntityId, child: EntityId) -> Self {
        Constraint::ParentChild { parent, child }
    }

    pub fn distance(entity_a: EntityId, entity_b: EntityId, distance: f64) -> Self {
        Constraint::Distance { entity_a, entity_b, distance }
    }

    pub fn references_entity(&self, entity_id: EntityId) -> bool {
        match self {
            Constraint::FixedAxis { .. } => false,
            Constraint::GearRelation { driver, driven, .. } => {
                *driver == entity_id || *driven == entity_id
            }
            Constraint::ParentChild { parent, child } => {
                *parent == entity_id || *child == entity_id
            }
            Constraint::Distance { entity_a, entity_b, .. } => {
                *entity_a == entity_id || *entity_b == entity_id
            }
            Constraint::Spring(spring) => {
                spring.entity_a == entity_id || spring.entity_b == entity_id
            }
            Constraint::Pendulum(pendulum) => pendulum.entity == entity_id,
            Constraint::Collision(collision) => {
                collision.entity_a == entity_id || collision.entity_b == entity_id
            }
            Constraint::ChemicalBond(bond) => {
                bond.atom_a == entity_id || bond.atom_b == entity_id
            }
            Constraint::BondAngle(angle) => {
                angle.atom_a == entity_id || angle.atom_b == entity_id || angle.atom_c == entity_id
            }
            Constraint::JointLimit(limit) => limit.joint == entity_id,
            Constraint::KinematicChain(chain) => {
                chain.joints.iter().any(|joint| *joint == entity_id)
                    || chain.end_effector_target == entity_id
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringConstraint {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    pub rest_length: f64,
    pub spring_constant: f64,
    pub damping: f64,
    pub point_a: Vector3,
    pub point_b: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendulumConstraint {
    pub entity: EntityId,
    pub length: f64,
    pub pivot_point: Vector3,
    pub gravity_direction: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionConstraint {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    pub restitution: f64,
    pub friction: f64,
    pub collision_normal: Vector3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChemicalBondConstraint {
    pub atom_a: EntityId,
    pub atom_b: EntityId,
    pub bond_order: BondOrder,
    pub equilibrium_length: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondAngleConstraint {
    pub atom_a: EntityId,
    pub atom_b: EntityId,
    pub atom_c: EntityId,
    pub equilibrium_angle: Angle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointLimitConstraint {
    pub joint: EntityId,
    pub min_position: f64,
    pub max_position: f64,
    pub max_velocity: f64,
    pub max_effort: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KinematicChainConstraint {
    pub joints: Vec<EntityId>,
    pub end_effector_target: EntityId,
}
