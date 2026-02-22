"""MathLib core math surface."""

__version__ = "2.0.0"

# Core
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
from mathlib.geometry.intersections import intersect, IntersectionResult, EmptySet

# Transforms
from mathlib.transforms.rotation import Rotation
from mathlib.transforms.translation import Translation
from mathlib.transforms.scale import Scale
from mathlib.transforms.affine import AffineTransform
from mathlib.transforms.homogeneous import HomogeneousMatrix

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
from mathlib.errors.math_errors import MathLibError, DimensionError, UnitError, AngleUnitError
from mathlib.errors.validation_errors import ValidationError, DomainError, InvariantError

__all__ = [
    "Scalar",
    "Vector",
    "Matrix",
    "Tensor",
    "Unit",
    "Length",
    "Angle",
    "Time",
    "Mass",
    "Point",
    "Line",
    "Plane",
    "Polygon",
    "Polyhedron",
    "intersect",
    "IntersectionResult",
    "EmptySet",
    "Rotation",
    "Translation",
    "Scale",
    "AffineTransform",
    "HomogeneousMatrix",
    "limit",
    "derivative",
    "gradient",
    "integrate",
    "Curve",
    "ParametricCurve",
    "Expression",
    "Variable",
    "Equation",
    "solve",
    "Polynomial",
    "check_dimensions",
    "check_domain",
    "validate_invariants",
    "MathLibError",
    "DimensionError",
    "UnitError",
    "AngleUnitError",
    "ValidationError",
    "DomainError",
    "InvariantError",
]
