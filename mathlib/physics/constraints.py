"""
Physical constraints and systems.

Springs, pendulums, wave motion, and collision constraints
for physics simulations.
"""

from dataclasses import dataclass
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
from mathlib.errors.math_errors import InvalidOperationError
import math


@dataclass(frozen=True)
class Spring:
    """Spring constraint between two points."""
    
    rest_length: Scalar
    spring_constant: Scalar  # k
    damping_coefficient: Scalar  # c
    
    def __init__(self, rest_length: Scalar, spring_constant: Scalar, 
                 damping_coefficient: Scalar = None):
        if rest_length.value < 0:
            raise ValueError("Rest length must be non-negative")
        if spring_constant.value <= 0:
            raise ValueError("Spring constant must be positive")
        
        if damping_coefficient is None:
            damping_coefficient = Scalar(0.0)
        elif damping_coefficient.value < 0:
            raise ValueError("Damping coefficient must be non-negative")
        
        object.__setattr__(self, 'rest_length', rest_length)
        object.__setattr__(self, 'spring_constant', spring_constant)
        object.__setattr__(self, 'damping_coefficient', damping_coefficient)
    
    def force(self, point1: Point, point2: Point, 
              velocity1: Vector = None, velocity2: Vector = None) -> tuple:
        """
        Compute spring force on both points.
        
        Returns: (force_on_1, force_on_2)
        """
        # Spring force: F = -k * (x - x0)
        displacement_vector = point2.position - point1.position
        current_length = displacement_vector.norm()
        extension = current_length.value - self.rest_length.value
        
        if current_length.value < 1e-10:
            # Points coincident - no force
            return (Vector.zero(3), Vector.zero(3))
        
        direction = displacement_vector.normalize()
        spring_force_magnitude = -self.spring_constant.value * extension
        
        # Damping force: F_d = -c * v_relative
        damping_force = Vector.zero(3)
        if velocity1 is not None and velocity2 is not None:
            relative_velocity = velocity2 - velocity1
            # Project onto spring direction
            v_along_spring = relative_velocity.dot(direction).value
            damping_force_magnitude = -self.damping_coefficient.value * v_along_spring
            damping_force = direction * damping_force_magnitude
        
        total_force = direction * spring_force_magnitude + damping_force
        
        # Force on point1 is in direction of point2
        # Force on point2 is opposite (Newton's third law)
        return (total_force, -total_force)
    
    def potential_energy(self, point1: Point, point2: Point) -> Scalar:
        """Compute elastic potential energy: U = (1/2) * k * x^2."""
        displacement_vector = point2.position - point1.position
        current_length = displacement_vector.norm().value
        extension = current_length - self.rest_length.value
        
        pe = 0.5 * self.spring_constant.value * extension * extension
        return Scalar(pe, self.spring_constant.unit)


@dataclass(frozen=True)
class Pendulum:
    """Simple or compound pendulum system."""
    
    length: Scalar
    mass: Scalar
    gravity: Scalar
    damping: Scalar
    
    def __init__(self, length: Scalar, mass: Scalar, 
                 gravity: Scalar = None, damping: Scalar = None):
        if length.value <= 0:
            raise ValueError("Length must be positive")
        if mass.value <= 0:
            raise ValueError("Mass must be positive")
        
        if gravity is None:
            gravity = Scalar(9.81)
        if damping is None:
            damping = Scalar(0.0)
        
        object.__setattr__(self, 'length', length)
        object.__setattr__(self, 'mass', mass)
        object.__setattr__(self, 'gravity', gravity)
        object.__setattr__(self, 'damping', damping)
    
    def angular_acceleration(self, angle: Scalar, angular_velocity: Scalar) -> Scalar:
        """
        Compute angular acceleration for pendulum.
        
        Equation: α = -(g/L) * sin(θ) - (c/mL^2) * ω
        """
        # Restoring torque
        g = self.gravity.value
        L = self.length.value
        theta = angle.value
        
        restoring_term = -(g / L) * (theta if abs(theta) < 0.1 else 
                                     1.0 if theta > 0 else -1.0)  # Small angle approx
        
        # Damping torque
        omega = angular_velocity.value
        damping_term = -(self.damping.value / (self.mass.value * L * L)) * omega
        
        alpha = restoring_term + damping_term
        return Scalar(alpha, angle.unit)
    
    def period(self) -> Scalar:
        """Compute period for small oscillations: T = 2π * sqrt(L/g)."""
        T = 2 * math.pi * (self.length.value / self.gravity.value) ** 0.5
        return Scalar(T)
    
    def total_energy(self, angle: Scalar, angular_velocity: Scalar) -> Scalar:
        """Compute total mechanical energy."""
        # Kinetic energy: KE = (1/2) * I * ω^2, where I = mL^2
        I = self.mass.value * self.length.value ** 2
        ke = 0.5 * I * angular_velocity.value ** 2
        
        # Potential energy: PE = mgh = mgL(1 - cos(θ))
        pe = self.mass.value * self.gravity.value * self.length.value * \
             (1 - math.cos(angle.value))
        
        return Scalar(ke + pe)


@dataclass(frozen=True)
class WaveMotion:
    """Wave propagation parameters."""
    
    amplitude: Scalar
    frequency: Scalar
    wavelength: Scalar
    phase_velocity: Scalar
    direction: Vector
    
    def __init__(self, amplitude: Scalar, frequency: Scalar, 
                 wavelength: Scalar, direction: Vector = None):
        if amplitude.value < 0:
            raise ValueError("Amplitude must be non-negative")
        if frequency.value <= 0:
            raise ValueError("Frequency must be positive")
        if wavelength.value <= 0:
            raise ValueError("Wavelength must be positive")
        
        if direction is None:
            direction = Vector([1, 0, 0])
        
        # Compute phase velocity: v = f * λ
        phase_velocity = Scalar(frequency.value * wavelength.value)
        
        object.__setattr__(self, 'amplitude', amplitude)
        object.__setattr__(self, 'frequency', frequency)
        object.__setattr__(self, 'wavelength', wavelength)
        object.__setattr__(self, 'phase_velocity', phase_velocity)
        object.__setattr__(self, 'direction', direction.normalize())
    
    def displacement(self, position: Point, time: float) -> Scalar:
        """
        Compute wave displacement at position and time.
        
        y = A * sin(k*x - ω*t + φ)
        where k = 2π/λ (wave number) and ω = 2πf (angular frequency)
        """
        # Project position onto wave direction
        x = position.position.dot(self.direction).value
        
        k = 2 * math.pi / self.wavelength.value  # wave number
        omega = 2 * math.pi * self.frequency.value  # angular frequency
        
        phase = k * x - omega * time
        displacement = self.amplitude.value * math.sin(phase)
        
        return Scalar(displacement, self.amplitude.unit)
    
    def velocity(self, position: Point, time: float) -> Scalar:
        """Compute particle velocity at position and time."""
        x = position.position.dot(self.direction).value
        k = 2 * math.pi / self.wavelength.value
        omega = 2 * math.pi * self.frequency.value
        
        phase = k * x - omega * time
        velocity = -self.amplitude.value * omega * math.cos(phase)
        
        return Scalar(velocity)


@dataclass(frozen=True)
class CollisionConstraint:
    """Collision parameters for rigid body interactions."""
    
    restitution: float  # Coefficient of restitution (0 = inelastic, 1 = elastic)
    friction: float  # Coefficient of friction
    collision_normal: Vector
    
    def __init__(self, restitution: float = 0.8, friction: float = 0.5,
                 collision_normal: Vector = None):
        if not (0 <= restitution <= 1):
            raise ValueError("Restitution must be between 0 and 1")
        if friction < 0:
            raise ValueError("Friction must be non-negative")
        
        if collision_normal is None:
            collision_normal = Vector([0, 1, 0])
        
        object.__setattr__(self, 'restitution', restitution)
        object.__setattr__(self, 'friction', friction)
        object.__setattr__(self, 'collision_normal', collision_normal.normalize())
    
    def resolve_collision(self, velocity1: Vector, velocity2: Vector,
                         mass1: float, mass2: float) -> tuple:
        """
        Compute post-collision velocities for two rigid bodies.
        
        Returns: (new_velocity1, new_velocity2)
        """
        n = self.collision_normal
        v1 = velocity1
        v2 = velocity2
        
        # Relative velocity along collision normal
        v_rel = v1 - v2
        v_rel_n = v_rel.dot(n).value
        
        # Don't resolve if velocities separating
        if v_rel_n > 0:
            return (v1, v2)
        
        # Impulse magnitude: j = -(1 + e) * v_rel_n / (1/m1 + 1/m2)
        e = self.restitution
        j = -(1 + e) * v_rel_n / (1/mass1 + 1/mass2)
        
        # Apply impulse
        impulse = n * j
        v1_new = v1 + impulse / mass1
        v2_new = v2 - impulse / mass2
        
        return (v1_new, v2_new)
