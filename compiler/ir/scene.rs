use crate::entity::Entity;
use crate::motion::{Motion, MotionKind};
use crate::timeline::Timeline;
use crate::constraint::Constraint;
use crate::ids::{EntityId, MotionId};

#[derive(Debug, Clone)]
pub struct Scene {
    pub entities: Vec<Entity>,
    pub motions: Vec<Motion>,
    pub timelines: Vec<Timeline>,
    pub constraints: Vec<Constraint>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            motions: Vec::new(),
            timelines: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn add_motion(&mut self, motion: Motion) {
        self.motions.push(motion);
    }

    pub fn add_timeline(&mut self, timeline: Timeline) {
        self.timelines.push(timeline);
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn get_motion(&self, id: MotionId) -> Option<&Motion> {
        self.motions.iter().find(|m| m.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate all motion targets exist
        for motion in &self.motions {
            if self.get_entity(motion.target).is_none() {
                return Err(format!("Motion {} targets non-existent entity {}", motion.id, motion.target));
            }
            for referenced in motion_references(&motion.kind) {
                if self.get_motion(referenced).is_none() {
                    return Err(format!(
                        "Motion {} references non-existent motion {}",
                        motion.id, referenced
                    ));
                }
            }
        }

        // Validate all timeline events reference existing motions
        for timeline in &self.timelines {
            for event in &timeline.events {
                if self.get_motion(event.motion).is_none() {
                    return Err(format!("Timeline {} references non-existent motion {}", timeline.id, event.motion));
                }
            }
        }

        // Validate all constraints reference existing entities
        for constraint in &self.constraints {
            match constraint {
                Constraint::GearRelation { driver, driven, .. } => {
                    if self.get_entity(*driver).is_none() {
                        return Err(format!("Constraint references non-existent driver entity {}", driver));
                    }
                    if self.get_entity(*driven).is_none() {
                        return Err(format!("Constraint references non-existent driven entity {}", driven));
                    }
                }
                Constraint::ParentChild { parent, child } => {
                    if self.get_entity(*parent).is_none() {
                        return Err(format!("Constraint references non-existent parent entity {}", parent));
                    }
                    if self.get_entity(*child).is_none() {
                        return Err(format!("Constraint references non-existent child entity {}", child));
                    }
                }
                Constraint::Distance { entity_a, entity_b, .. } => {
                    if self.get_entity(*entity_a).is_none() {
                        return Err(format!("Constraint references non-existent entity {}", entity_a));
                    }
                    if self.get_entity(*entity_b).is_none() {
                        return Err(format!("Constraint references non-existent entity {}", entity_b));
                    }
                }
                Constraint::Spring(spring) => {
                    if self.get_entity(spring.entity_a).is_none() {
                        return Err(format!(
                            "Spring constraint references non-existent entity {}",
                            spring.entity_a
                        ));
                    }
                    if self.get_entity(spring.entity_b).is_none() {
                        return Err(format!(
                            "Spring constraint references non-existent entity {}",
                            spring.entity_b
                        ));
                    }
                }
                Constraint::Pendulum(pendulum) => {
                    if self.get_entity(pendulum.entity).is_none() {
                        return Err(format!(
                            "Pendulum constraint references non-existent entity {}",
                            pendulum.entity
                        ));
                    }
                }
                Constraint::Collision(collision) => {
                    if self.get_entity(collision.entity_a).is_none() {
                        return Err(format!(
                            "Collision constraint references non-existent entity {}",
                            collision.entity_a
                        ));
                    }
                    if self.get_entity(collision.entity_b).is_none() {
                        return Err(format!(
                            "Collision constraint references non-existent entity {}",
                            collision.entity_b
                        ));
                    }
                }
                Constraint::ChemicalBond(bond) => {
                    if self.get_entity(bond.atom_a).is_none() {
                        return Err(format!(
                            "Chemical bond constraint references non-existent atom {}",
                            bond.atom_a
                        ));
                    }
                    if self.get_entity(bond.atom_b).is_none() {
                        return Err(format!(
                            "Chemical bond constraint references non-existent atom {}",
                            bond.atom_b
                        ));
                    }
                }
                Constraint::BondAngle(angle) => {
                    if self.get_entity(angle.atom_a).is_none() {
                        return Err(format!(
                            "Bond angle constraint references non-existent atom {}",
                            angle.atom_a
                        ));
                    }
                    if self.get_entity(angle.atom_b).is_none() {
                        return Err(format!(
                            "Bond angle constraint references non-existent atom {}",
                            angle.atom_b
                        ));
                    }
                    if self.get_entity(angle.atom_c).is_none() {
                        return Err(format!(
                            "Bond angle constraint references non-existent atom {}",
                            angle.atom_c
                        ));
                    }
                }
                Constraint::JointLimit(limit) => {
                    if self.get_entity(limit.joint).is_none() {
                        return Err(format!(
                            "Joint limit constraint references non-existent joint {}",
                            limit.joint
                        ));
                    }
                }
                Constraint::KinematicChain(chain) => {
                    for joint in &chain.joints {
                        if self.get_entity(*joint).is_none() {
                            return Err(format!(
                                "Kinematic chain constraint references non-existent joint {}",
                                joint
                            ));
                        }
                    }
                    if self.get_entity(chain.end_effector_target).is_none() {
                        return Err(format!(
                            "Kinematic chain constraint references non-existent target {}",
                            chain.end_effector_target
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn motion_references(kind: &MotionKind) -> Vec<MotionId> {
    match kind {
        MotionKind::Sequential(motion) => motion.motions.clone(),
        MotionKind::Parallel(motion) => motion.motions.clone(),
        MotionKind::Damped(motion) => vec![motion.base_motion],
        MotionKind::Periodic(motion) => vec![motion.base_motion],
        MotionKind::Eased(motion) => vec![motion.base_motion],
        MotionKind::Oscillation(_) => Vec::new(),
        MotionKind::Orbital(_) => Vec::new(),
        MotionKind::Parametric(_) => Vec::new(),
        MotionKind::Rotation { .. } => Vec::new(),
        MotionKind::Translation { .. } => Vec::new(),
        MotionKind::Scale { .. } => Vec::new(),
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
