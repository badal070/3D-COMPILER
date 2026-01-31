"""
Force fields and field-based forces.

Support for gravitational, electromagnetic, and custom force fields
with configurable falloff functions.
"""

from dataclasses import dataclass
from typing import Callable
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
from mathlib.errors.math_errors import InvalidOperationError


@dataclass(frozen=True)
class ForceField:
    """Generic force field definition."""
    
    field_type: str
    strength: Scalar
    direction: Vector
    falloff: Callable[[float], float]  # distance -> field strength multiplier
    
    def __init__(
        self,
        field_type: str,
        strength: Scalar,
        direction: Vector = None,
        falloff: Callable[[float], float] = None
    ):
        valid_types = ['gravitational', 'electromagnetic', 'uniform', 'radial']
        if field_type not in valid_types:
            raise ValueError(f"field_type must be one of {valid_types}")
        
        if direction is not None and direction.dimension != 3:
            raise InvalidOperationError("force field", "direction must be 3D")
        
        # Default uniform field (no falloff)
        if falloff is None:
            falloff = lambda d: 1.0
        
        # Default downward direction for gravity
        if direction is None:
            direction = Vector([0, -1, 0])
        
        object.__setattr__(self, 'field_type', field_type)
        object.__setattr__(self, 'strength', strength)
        object.__setattr__(self, 'direction', direction.normalize())
        object.__setattr__(self, 'falloff', falloff)
    
    def force_at_point(self, point: Point, mass: Scalar = None) -> Vector:
        """Compute force at given point."""
        if self.field_type == 'uniform':
            force_magnitude = self.strength.value
            if mass is not None:
                force_magnitude *= mass.value
            return self.direction * force_magnitude
        
        elif self.field_type == 'gravitational':
            # F = m * g (uniform gravity)
            if mass is None:
                raise ValueError("Mass required for gravitational field")
            g = self.strength.value
            return self.direction * (mass.value * g)
        
        elif self.field_type == 'radial':
            # Field strength decreases with distance from origin
            distance = point.position.norm().value
            falloff_factor = self.falloff(distance)
            force_magnitude = self.strength.value * falloff_factor
            if mass is not None:
                force_magnitude *= mass.value
            
            # Direction is radial (toward or away from origin)
            if distance > 1e-10:
                radial_direction = point.position.normalize()
                return radial_direction * force_magnitude
            return Vector.zero(3)
        
        else:
            raise NotImplementedError(f"Force field type {self.field_type}")
    
    @staticmethod
    def gravitational_field(g: float = 9.81) -> 'ForceField':
        """Create uniform gravitational field."""
        return ForceField(
            'gravitational',
            Scalar(g),
            Vector([0, -1, 0])
        )
    
    @staticmethod
    def inverse_square_field(strength: float) -> 'ForceField':
        """Create inverse square law field (gravity, electrostatic)."""
        return ForceField(
            'radial',
            Scalar(strength),
            Vector([0, 0, 0]),  # Will be computed radially
            falloff=lambda d: 1.0 / (d * d + 1e-10)  # Prevent division by zero
        )
