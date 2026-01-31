"""
MathLib - Deterministic Math Kernel for 3D Educational Concepts

A strict, immutable, unit-aware mathematical foundation for educational
3D rendering, DSL validation, and exact geometric computation.

Core Principles:
- Immutability (objects never change)
- Explicit units (degrees ≠ radians ≠ meters)
- No side effects (pure functions only)
- Symbolic + Numeric separation
- Fail loudly (raise errors, don't "fix" input)
- Human-readable math (education > performance)

Extended Features (v2.0):
- Hierarchical motion graphs for complex animations
- Physics simulation for realistic motion
- Chemistry module for molecular visualization
- Mathematical visualization tools
"""

__version__ = "2.0.0"

# Core primitives
from mathlib.core.scalar import Scalar
from mathlib.core.vector import Vector
from mathlib.core.matrix import Matrix
from mathlib.core.tensor import Tensor
from mathlib.core.units import Unit, Length, Angle, Time, Mass

# Geometry
from mathlib.geometry.point import Point
from mathlib.geometry.line import Line
from mathlib.geometry.plane import Plane
from mathlib.geometry.polygon import Polygon
from mathlib.geometry.polyhedron import Polyhedron
from mathlib.geometry.intersections import (
    intersect,
    IntersectionResult,
    EmptySet
)

# Transforms
from mathlib.transforms.rotation import Rotation
from mathlib.transforms.translation import Translation
from mathlib.transforms.scale import Scale
from mathlib.transforms.affine import AffineTransform
from mathlib.transforms.homogeneous import HomogeneousMatrix

# Kinematics
from mathlib.kinematics.frames import Frame
from mathlib.kinematics.joints import Joint, RevoluteJoint, PrismaticJoint
from mathlib.kinematics.chains import KinematicChain
from mathlib.kinematics.constraints import Constraint

# Animation (NEW)
from mathlib.animation.motion_graph import (
    MotionNode,
    MotionGraph,
    MotionType,
    EasingFunction
)

# Physics (NEW)
from mathlib.physics.simulation import (
    PhysicsState,
    PhysicsSimulator,
    Force,
    SpringForce,
    DampingForce,
    Collision,
    create_pendulum_simulation
)

# Chemistry (NEW)
from mathlib.chemistry.molecules import (
    Atom,
    Bond,
    BondType,
    Molecule,
    MolecularGeometry,
    Reaction
)

# Visualization (NEW)
from mathlib.visualization.surfaces import (
    ParametricSurface,
    CommonSurfaces,
    VectorField,
    CommonVectorFields,
    ImplicitSurface,
    FunctionGraph
)

# Calculus
from mathlib.calculus.limits import limit
from mathlib.calculus.derivatives import derivative, gradient
from mathlib.calculus.integrals import integrate
from mathlib.calculus.curves import Curve, ParametricCurve

# Algebra
from mathlib.algebra.expressions import Expression, Variable
from mathlib.algebra.equations import Equation
from mathlib.algebra.solvers import solve
from mathlib.algebra.polynomials import Polynomial

# Validation
from mathlib.validation.dimension_check import check_dimensions
from mathlib.validation.domain_check import check_domain
from mathlib.validation.invariants import validate_invariants

# Errors
from mathlib.errors.math_errors import (
    MathLibError,
    DimensionError,
    UnitError,
    AngleUnitError
)
from mathlib.errors.validation_errors import (
    ValidationError,
    DomainError,
    InvariantError
)

__all__ = [
    # Core
    'Scalar', 'Vector', 'Matrix', 'Tensor',
    'Unit', 'Length', 'Angle', 'Time', 'Mass',
    
    # Geometry
    'Point', 'Line', 'Plane', 'Polygon', 'Polyhedron',
    'intersect', 'IntersectionResult', 'EmptySet',
    
    # Transforms
    'Rotation', 'Translation', 'Scale', 'AffineTransform', 'HomogeneousMatrix',
    
    # Kinematics
    'Frame', 'Joint', 'RevoluteJoint', 'PrismaticJoint',
    'KinematicChain', 'Constraint',
    
    # Animation
    'MotionNode', 'MotionGraph', 'MotionType', 'EasingFunction',
    
    # Physics
    'PhysicsState', 'PhysicsSimulator', 'Force', 'SpringForce',
    'DampingForce', 'Collision', 'create_pendulum_simulation',
    
    # Chemistry
    'Atom', 'Bond', 'BondType', 'Molecule', 'MolecularGeometry', 'Reaction',
    
    # Visualization
    'ParametricSurface', 'CommonSurfaces', 'VectorField', 'CommonVectorFields',
    'ImplicitSurface', 'FunctionGraph',
    
    # Calculus
    'limit', 'derivative', 'gradient', 'integrate',
    'Curve', 'ParametricCurve',
    
    # Algebra
    'Expression', 'Variable', 'Equation', 'solve', 'Polynomial',
    
    # Validation
    'check_dimensions', 'check_domain', 'validate_invariants',
    
    # Errors
    'MathLibError', 'DimensionError', 'UnitError', 'AngleUnitError',
    'ValidationError', 'DomainError', 'InvariantError',
]
# Physics 

from mathlib.physics import ( 

    RigidBody, ForceField, Spring, Pendulum, 

    WaveMotion, CollisionConstraint 

) 

 

# Chemistry 

from mathlib.chemistry import ( 

    Atom, Bond, BondAngle, MolecularVibration, 

    CoulombicField, LennardJonesPotential, ElectronOrbital 

) 

 

# Robotics 

from mathlib.robotics import ( 

    JointLimit, RevoluteJointKinematics, PrismaticJointKinematics, 

    RobotLink, SerialManipulator, JointTrajectory, WorkspacePath 

) 

 

# Compound Motions 

from mathlib.motions import ( 

    SequentialMotion, ParallelMotion, OscillatoryMotion, 

    DampedMotion, PeriodicMotion, EasedMotion, SplinePath 

) 