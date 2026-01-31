"""
Robot kinematics.

Forward kinematics, Jacobian computation, and inverse kinematics
for serial manipulators.
"""

from dataclasses import dataclass
from typing import List, Optional
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.core.matrix import Matrix
from mathlib.geometry.point import Point
from mathlib.transforms.rotation import Rotation
from mathlib.transforms.affine import AffineTransform
from mathlib.errors.math_errors import InvalidOperationError
import math


@dataclass(frozen=True)
class JointLimit:
    """Joint position and velocity limits."""
    
    min_position: float
    max_position: float
    max_velocity: float
    max_effort: float
    
    def __init__(
        self,
        min_position: float,
        max_position: float,
        max_velocity: float,
        max_effort: float
    ):
        if min_position >= max_position:
            raise ValueError("min_position must be less than max_position")
        if max_velocity <= 0:
            raise ValueError("max_velocity must be positive")
        if max_effort <= 0:
            raise ValueError("max_effort must be positive")
        
        object.__setattr__(self, 'min_position', min_position)
        object.__setattr__(self, 'max_position', max_position)
        object.__setattr__(self, 'max_velocity', max_velocity)
        object.__setattr__(self, 'max_effort', max_effort)
    
    def clamp_position(self, position: float) -> float:
        """Clamp position to limits."""
        return max(self.min_position, min(self.max_position, position))
    
    def is_within_limits(self, position: float, velocity: float = 0.0) -> bool:
        """Check if position and velocity are within limits."""
        pos_ok = self.min_position <= position <= self.max_position
        vel_ok = abs(velocity) <= self.max_velocity
        return pos_ok and vel_ok


@dataclass(frozen=True)
class RevoluteJointKinematics:
    """Revolute (rotational) joint kinematics."""
    
    joint_id: str
    axis: Vector  # Rotation axis in parent frame
    limits: JointLimit
    parent_link: str
    child_link: str
    origin: AffineTransform
    
    def __init__(
        self,
        joint_id: str,
        axis: Vector,
        limits: JointLimit,
        parent_link: str,
        child_link: str,
        origin: AffineTransform = None
    ):
        if axis.dimension != 3:
            raise InvalidOperationError("revolute joint", "axis must be 3D")
        
        if origin is None:
            origin = AffineTransform(Matrix.identity(3), Vector.zero(3))
        
        object.__setattr__(self, 'joint_id', joint_id)
        object.__setattr__(self, 'axis', axis.normalize())
        object.__setattr__(self, 'limits', limits)
        object.__setattr__(self, 'parent_link', parent_link)
        object.__setattr__(self, 'child_link', child_link)
        object.__setattr__(self, 'origin', origin)
    
    def transform(self, angle: float) -> AffineTransform:
        """Compute transformation for given joint angle."""
        from mathlib.core.units import RADIAN
        
        # Create rotation around axis
        # For standard axes, use built-in rotations
        if abs(self.axis[0] - 1.0) < 1e-9:
            rotation = Rotation("x", Scalar(angle, RADIAN))
        elif abs(self.axis[1] - 1.0) < 1e-9:
            rotation = Rotation("y", Scalar(angle, RADIAN))
        elif abs(self.axis[2] - 1.0) < 1e-9:
            rotation = Rotation("z", Scalar(angle, RADIAN))
        else:
            rotation = Rotation(self.axis, Scalar(angle, RADIAN))
        
        # Combine with origin transform
        rot_transform = AffineTransform(rotation.as_matrix(), Vector.zero(3))
        return self.origin @ rot_transform
    
    def jacobian_column(self, joint_position: Point, end_effector_position: Point) -> tuple:
        """
        Compute Jacobian column for this joint.
        
        Returns: (linear_velocity_contribution, angular_velocity_contribution)
        """
        # For revolute joint:
        # J_v = axis × (p_ee - p_joint)
        # J_ω = axis
        
        r = end_effector_position.position - joint_position.position
        j_v = self.axis.cross(r)
        j_omega = self.axis
        
        return (j_v, j_omega)


@dataclass(frozen=True)
class PrismaticJointKinematics:
    """Prismatic (sliding) joint kinematics."""
    
    joint_id: str
    axis: Vector  # Translation axis in parent frame
    limits: JointLimit
    parent_link: str
    child_link: str
    origin: AffineTransform
    
    def __init__(
        self,
        joint_id: str,
        axis: Vector,
        limits: JointLimit,
        parent_link: str,
        child_link: str,
        origin: AffineTransform = None
    ):
        if axis.dimension != 3:
            raise InvalidOperationError("prismatic joint", "axis must be 3D")
        
        if origin is None:
            origin = AffineTransform(Matrix.identity(3), Vector.zero(3))
        
        object.__setattr__(self, 'joint_id', joint_id)
        object.__setattr__(self, 'axis', axis.normalize())
        object.__setattr__(self, 'limits', limits)
        object.__setattr__(self, 'parent_link', parent_link)
        object.__setattr__(self, 'child_link', child_link)
        object.__setattr__(self, 'origin', origin)
    
    def transform(self, distance: float) -> AffineTransform:
        """Compute transformation for given joint displacement."""
        # Translation along axis
        translation = self.axis * distance
        trans_transform = AffineTransform(Matrix.identity(3), translation)
        
        # Combine with origin transform
        return self.origin @ trans_transform
    
    def jacobian_column(self) -> tuple:
        """
        Compute Jacobian column for this joint.
        
        Returns: (linear_velocity_contribution, angular_velocity_contribution)
        """
        # For prismatic joint:
        # J_v = axis
        # J_ω = 0
        
        j_v = self.axis
        j_omega = Vector.zero(3)
        
        return (j_v, j_omega)


@dataclass(frozen=True)
class RobotLink:
    """Robot link properties."""
    
    link_id: str
    length: Scalar
    mass: Scalar
    inertia_tensor: Matrix
    center_of_mass: Vector
    parent_joint: Optional[str]
    
    def __init__(
        self,
        link_id: str,
        length: Scalar,
        mass: Scalar,
        inertia_tensor: Matrix = None,
        center_of_mass: Vector = None,
        parent_joint: str = None
    ):
        if length.value < 0:
            raise ValueError("Link length must be non-negative")
        if mass.value <= 0:
            raise ValueError("Link mass must be positive")
        
        # Default inertia: point mass at end of link
        if inertia_tensor is None:
            I = mass.value * length.value ** 2
            inertia_tensor = Matrix([[I, 0, 0], [0, I, 0], [0, 0, I]])
        
        if center_of_mass is None:
            # Center of mass at link midpoint
            center_of_mass = Vector([length.value / 2, 0, 0])
        
        object.__setattr__(self, 'link_id', link_id)
        object.__setattr__(self, 'length', length)
        object.__setattr__(self, 'mass', mass)
        object.__setattr__(self, 'inertia_tensor', inertia_tensor)
        object.__setattr__(self, 'center_of_mass', center_of_mass)
        object.__setattr__(self, 'parent_joint', parent_joint)


@dataclass(frozen=True)
class SerialManipulator:
    """Serial manipulator with multiple joints."""
    
    name: str
    joints: List  # List of RevoluteJointKinematics or PrismaticJointKinematics
    links: List[RobotLink]
    base_transform: AffineTransform
    
    def __init__(
        self,
        name: str,
        joints: List,
        links: List[RobotLink],
        base_transform: AffineTransform = None
    ):
        if len(joints) != len(links):
            raise ValueError("Number of joints must equal number of links")
        
        if base_transform is None:
            base_transform = AffineTransform(Matrix.identity(3), Vector.zero(3))
        
        object.__setattr__(self, 'name', name)
        object.__setattr__(self, 'joints', tuple(joints))
        object.__setattr__(self, 'links', tuple(links))
        object.__setattr__(self, 'base_transform', base_transform)
    
    @property
    def dof(self) -> int:
        """Degrees of freedom."""
        return len(self.joints)
    
    def forward_kinematics(self, joint_positions: List[float]) -> Point:
        """
        Compute end-effector position for given joint configuration.
        
        Args:
            joint_positions: List of joint positions (angles for revolute, distances for prismatic)
        
        Returns:
            End-effector position
        """
        if len(joint_positions) != self.dof:
            raise ValueError(f"Expected {self.dof} joint positions, got {len(joint_positions)}")
        
        # Start from base
        current_transform = self.base_transform
        
        # Apply each joint transformation
        for joint, position in zip(self.joints, joint_positions):
            joint_transform = joint.transform(position)
            current_transform = current_transform @ joint_transform
        
        # Extract position from final transform
        end_position = current_transform.translation
        return Point(list(end_position.components), end_position.unit)
    
    def jacobian(self, joint_positions: List[float]) -> Matrix:
        """
        Compute 6×n Jacobian matrix at given configuration.
        
        Returns:
            Jacobian matrix [J_v; J_ω] where J_v is linear velocity Jacobian
            and J_ω is angular velocity Jacobian
        """
        if len(joint_positions) != self.dof:
            raise ValueError(f"Expected {self.dof} joint positions")
        
        # Compute end-effector position
        ee_pos = self.forward_kinematics(joint_positions)
        
        # Build Jacobian column by column
        jacobian_columns = []
        current_transform = self.base_transform
        
        for i, (joint, position) in enumerate(zip(self.joints, joint_positions)):
            # Joint position in world frame
            joint_pos = Point(list(current_transform.translation.components),
                            current_transform.translation.unit)
            
            # Get Jacobian column contribution
            if isinstance(joint, RevoluteJointKinematics):
                j_v, j_omega = joint.jacobian_column(joint_pos, ee_pos)
            else:  # PrismaticJointKinematics
                j_v, j_omega = joint.jacobian_column()
            
            # Stack into 6D column
            column = [j_v[0], j_v[1], j_v[2], j_omega[0], j_omega[1], j_omega[2]]
            jacobian_columns.append(column)
            
            # Update transform for next joint
            joint_transform = joint.transform(position)
            current_transform = current_transform @ joint_transform
        
        # Convert to matrix (6 × dof)
        jacobian_elements = []
        for row_idx in range(6):
            row = [col[row_idx] for col in jacobian_columns]
            jacobian_elements.append(row)
        
        return Matrix(jacobian_elements)
    
    def inverse_kinematics_numeric(
        self,
        target_position: Point,
        initial_guess: List[float] = None,
        tolerance: float = 1e-6,
        max_iterations: int = 100
    ) -> Optional[List[float]]:
        """
        Solve inverse kinematics numerically using Newton-Raphson.
        
        Args:
            target_position: Desired end-effector position
            initial_guess: Initial joint configuration
            tolerance: Convergence tolerance
            max_iterations: Maximum iterations
        
        Returns:
            Joint positions or None if no solution found
        """
        if initial_guess is None:
            # Start from zero configuration
            initial_guess = [0.0] * self.dof
        
        q = list(initial_guess)
        
        for iteration in range(max_iterations):
            # Current end-effector position
            current_pos = self.forward_kinematics(q)
            
            # Error vector
            error = target_position.position - current_pos.position
            error_magnitude = error.norm().value
            
            if error_magnitude < tolerance:
                return q  # Converged
            
            # Compute Jacobian (only use position part, first 3 rows)
            J_full = self.jacobian(q)
            J_pos = Matrix([J_full.elements[i][:self.dof] for i in range(3)])
            
            # Pseudo-inverse using transpose (damped least squares)
            alpha = 0.5  # Damping factor
            J_T = J_pos.transpose()
            
            # Δq = α * J^T * error
            delta_q = []
            for i in range(self.dof):
                dq_i = 0.0
                for j in range(3):
                    dq_i += alpha * J_T[i, j] * error[j]
                delta_q.append(dq_i)
            
            # Update configuration
            q = [q[i] + delta_q[i] for i in range(self.dof)]
            
            # Clamp to joint limits
            for i, joint in enumerate(self.joints):
                q[i] = joint.limits.clamp_position(q[i])
        
        # Did not converge
        return None
