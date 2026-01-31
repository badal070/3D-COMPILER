"""
Physics module for mathlib.

Advanced physics simulation primitives including rigid body dynamics,
force fields, springs, pendulums, and wave motion.
"""

from mathlib.physics.rigid_body import RigidBody
from mathlib.physics.forces import ForceField
from mathlib.physics.constraints import Spring, Pendulum, WaveMotion, CollisionConstraint

__all__ = [
    'RigidBody',
    'ForceField',
    'Spring',
    'Pendulum',
    'WaveMotion',
    'CollisionConstraint'
]
