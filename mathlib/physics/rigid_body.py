"""
Rigid body dynamics.

Support for rigid bodies with mass properties, inertia tensors,
force/torque application, and energy calculations.
"""

from dataclasses import dataclass
from mathlib.core.vector import Vector
from mathlib.core.matrix import Matrix
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
from mathlib.transforms.rotation import Rotation
from mathlib.errors.math_errors import InvalidOperationError


@dataclass(frozen=True)
class RigidBody:
    """Rigid body with mass properties."""
    
    mass: Scalar
    inertia_tensor: Matrix  # 3x3 moment of inertia matrix
    center_of_mass: Vector
    position: Point
    orientation: Rotation
    linear_velocity: Vector
    angular_velocity: Vector
    
    def __init__(
        self,
        mass: Scalar,
        inertia_tensor: Matrix,
        center_of_mass: Vector = None,
        position: Point = None,
        orientation: Rotation = None,
        linear_velocity: Vector = None,
        angular_velocity: Vector = None
    ):
        if mass.value <= 0:
            raise ValueError("Mass must be positive")
        
        if inertia_tensor.shape != (3, 3):
            raise InvalidOperationError("inertia tensor", "must be 3x3 matrix")
        
        # Default values
        if center_of_mass is None:
            center_of_mass = Vector.zero(3)
        if position is None:
            position = Point.origin(3)
        if orientation is None:
            from mathlib.core.units import RADIAN
            orientation = Rotation("x", Scalar(0.0, RADIAN))
        if linear_velocity is None:
            linear_velocity = Vector.zero(3)
        if angular_velocity is None:
            angular_velocity = Vector.zero(3)
        
        object.__setattr__(self, 'mass', mass)
        object.__setattr__(self, 'inertia_tensor', inertia_tensor)
        object.__setattr__(self, 'center_of_mass', center_of_mass)
        object.__setattr__(self, 'position', position)
        object.__setattr__(self, 'orientation', orientation)
        object.__setattr__(self, 'linear_velocity', linear_velocity)
        object.__setattr__(self, 'angular_velocity', angular_velocity)
    
    def apply_force(self, force: Vector, application_point: Vector = None) -> tuple:
        """
        Compute effect of applied force.
        
        Returns: (linear_acceleration, angular_acceleration)
        """
        # F = ma -> a = F/m
        linear_accel = force / self.mass.value
        
        if application_point is None:
            # Force through center of mass - no torque
            return (linear_accel, Vector.zero(3))
        
        # Torque = r × F
        r = application_point - self.center_of_mass
        torque = r.cross(force)
        
        # α = I^(-1) * τ
        angular_accel = self.inertia_tensor.inverse().apply(torque)
        
        return (linear_accel, angular_accel)
    
    def apply_torque(self, torque: Vector) -> Vector:
        """
        Compute angular acceleration from applied torque.
        
        Returns: angular_acceleration
        """
        return self.inertia_tensor.inverse().apply(torque)
    
    def kinetic_energy(self) -> Scalar:
        """Compute total kinetic energy (translational + rotational)."""
        # KE_trans = (1/2) * m * v^2
        ke_trans = 0.5 * self.mass.value * self.linear_velocity.norm().value ** 2
        
        # KE_rot = (1/2) * ω^T * I * ω
        I_omega = self.inertia_tensor.apply(self.angular_velocity)
        ke_rot = 0.5 * self.angular_velocity.dot(I_omega).value
        
        return Scalar(ke_trans + ke_rot, self.mass.unit)
    
    def angular_momentum(self) -> Vector:
        """Compute angular momentum L = I * ω."""
        return self.inertia_tensor.apply(self.angular_velocity)
