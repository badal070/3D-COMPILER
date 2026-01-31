"""
Trajectory planning and execution.

Joint-space and workspace trajectory planning with
interpolation methods.
"""

from dataclasses import dataclass
from typing import List, Optional
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
from mathlib.transforms.rotation import Rotation
from mathlib.errors.math_errors import InvalidOperationError
import math


@dataclass(frozen=True)
class TrajectoryWaypoint:
    """Single waypoint in a trajectory."""
    
    time: float
    joint_positions: List[float]
    joint_velocities: Optional[List[float]]
    joint_accelerations: Optional[List[float]]
    
    def __init__(
        self,
        time: float,
        joint_positions: List[float],
        joint_velocities: List[float] = None,
        joint_accelerations: List[float] = None
    ):
        if time < 0:
            raise ValueError("Time must be non-negative")
        
        dof = len(joint_positions)
        
        if joint_velocities is not None and len(joint_velocities) != dof:
            raise ValueError("Joint velocities must have same DOF as positions")
        
        if joint_accelerations is not None and len(joint_accelerations) != dof:
            raise ValueError("Joint accelerations must have same DOF as positions")
        
        object.__setattr__(self, 'time', time)
        object.__setattr__(self, 'joint_positions', tuple(joint_positions))
        object.__setattr__(self, 'joint_velocities', 
                          tuple(joint_velocities) if joint_velocities else None)
        object.__setattr__(self, 'joint_accelerations',
                          tuple(joint_accelerations) if joint_accelerations else None)
    
    @property
    def dof(self) -> int:
        """Degrees of freedom."""
        return len(self.joint_positions)


@dataclass(frozen=True)
class JointTrajectory:
    """Trajectory defined by waypoints in joint space."""
    
    waypoints: List[TrajectoryWaypoint]
    interpolation_method: str
    
    def __init__(
        self,
        waypoints: List[TrajectoryWaypoint],
        interpolation_method: str = "cubic_spline"
    ):
        if len(waypoints) < 2:
            raise ValueError("Trajectory must have at least 2 waypoints")
        
        # Verify all waypoints have same DOF
        dof = waypoints[0].dof
        for wp in waypoints:
            if wp.dof != dof:
                raise ValueError("All waypoints must have same DOF")
        
        # Verify waypoints are time-ordered
        for i in range(len(waypoints) - 1):
            if waypoints[i].time >= waypoints[i + 1].time:
                raise ValueError("Waypoints must be strictly increasing in time")
        
        valid_methods = ['linear', 'cubic_spline', 'quintic_spline']
        if interpolation_method not in valid_methods:
            raise ValueError(f"Interpolation method must be one of {valid_methods}")
        
        object.__setattr__(self, 'waypoints', tuple(waypoints))
        object.__setattr__(self, 'interpolation_method', interpolation_method)
    
    @property
    def duration(self) -> float:
        """Total trajectory duration."""
        return self.waypoints[-1].time - self.waypoints[0].time
    
    @property
    def dof(self) -> int:
        """Degrees of freedom."""
        return self.waypoints[0].dof
    
    def sample(self, time: float) -> TrajectoryWaypoint:
        """
        Sample trajectory at given time.
        
        Returns:
            Interpolated waypoint at specified time
        """
        if time < self.waypoints[0].time or time > self.waypoints[-1].time:
            raise ValueError(f"Time {time} outside trajectory range")
        
        # Find surrounding waypoints
        i = 0
        while i < len(self.waypoints) - 1 and self.waypoints[i + 1].time <= time:
            i += 1
        
        if i == len(self.waypoints) - 1:
            return self.waypoints[-1]
        
        wp0 = self.waypoints[i]
        wp1 = self.waypoints[i + 1]
        
        # Interpolate based on method
        if self.interpolation_method == 'linear':
            return self._interpolate_linear(wp0, wp1, time)
        elif self.interpolation_method == 'cubic_spline':
            return self._interpolate_cubic(wp0, wp1, time)
        elif self.interpolation_method == 'quintic_spline':
            return self._interpolate_quintic(wp0, wp1, time)
        
        raise NotImplementedError(f"Interpolation method {self.interpolation_method}")
    
    def _interpolate_linear(
        self,
        wp0: TrajectoryWaypoint,
        wp1: TrajectoryWaypoint,
        time: float
    ) -> TrajectoryWaypoint:
        """Linear interpolation between waypoints."""
        t0 = wp0.time
        t1 = wp1.time
        
        # Normalized time parameter [0, 1]
        s = (time - t0) / (t1 - t0)
        
        # Linear interpolation
        positions = []
        for i in range(self.dof):
            q = wp0.joint_positions[i] + s * (wp1.joint_positions[i] - wp0.joint_positions[i])
            positions.append(q)
        
        # Constant velocity
        velocities = []
        for i in range(self.dof):
            v = (wp1.joint_positions[i] - wp0.joint_positions[i]) / (t1 - t0)
            velocities.append(v)
        
        return TrajectoryWaypoint(time, positions, velocities)
    
    def _interpolate_cubic(
        self,
        wp0: TrajectoryWaypoint,
        wp1: TrajectoryWaypoint,
        time: float
    ) -> TrajectoryWaypoint:
        """
        Cubic spline interpolation (Hermite).
        
        Uses position and velocity at endpoints.
        """
        t0 = wp0.time
        t1 = wp1.time
        dt = t1 - t0
        
        # Normalized time [0, 1]
        s = (time - t0) / dt
        
        # Hermite basis functions
        h00 = 2*s**3 - 3*s**2 + 1
        h10 = s**3 - 2*s**2 + s
        h01 = -2*s**3 + 3*s**2
        h11 = s**3 - s**2
        
        # Derivatives of basis functions
        dh00 = 6*s**2 - 6*s
        dh10 = 3*s**2 - 4*s + 1
        dh01 = -6*s**2 + 6*s
        dh11 = 3*s**2 - 2*s
        
        positions = []
        velocities = []
        
        for i in range(self.dof):
            p0 = wp0.joint_positions[i]
            p1 = wp1.joint_positions[i]
            
            # Use provided velocities or estimate
            if wp0.joint_velocities is not None:
                v0 = wp0.joint_velocities[i] * dt
            else:
                v0 = (p1 - p0)  # Simple estimate
            
            if wp1.joint_velocities is not None:
                v1 = wp1.joint_velocities[i] * dt
            else:
                v1 = (p1 - p0)
            
            # Position interpolation
            q = h00 * p0 + h10 * v0 + h01 * p1 + h11 * v1
            positions.append(q)
            
            # Velocity interpolation
            qd = (dh00 * p0 + dh10 * v0 + dh01 * p1 + dh11 * v1) / dt
            velocities.append(qd)
        
        return TrajectoryWaypoint(time, positions, velocities)
    
    def _interpolate_quintic(
        self,
        wp0: TrajectoryWaypoint,
        wp1: TrajectoryWaypoint,
        time: float
    ) -> TrajectoryWaypoint:
        """
        Quintic spline interpolation.
        
        Uses position, velocity, and acceleration at endpoints.
        """
        # Simplified quintic - uses cubic as fallback for now
        return self._interpolate_cubic(wp0, wp1, time)


@dataclass(frozen=True)
class WorkspacePath:
    """Trajectory defined in workspace (Cartesian) coordinates."""
    
    waypoints: List[Point]
    orientations: Optional[List[Rotation]]
    
    def __init__(
        self,
        waypoints: List[Point],
        orientations: List[Rotation] = None
    ):
        if len(waypoints) < 2:
            raise ValueError("Path must have at least 2 waypoints")
        
        # Verify all waypoints have same dimension
        dim = waypoints[0].dimension
        for wp in waypoints:
            if wp.dimension != dim:
                raise ValueError("All waypoints must have same dimension")
        
        if orientations is not None and len(orientations) != len(waypoints):
            raise ValueError("Number of orientations must match number of waypoints")
        
        object.__setattr__(self, 'waypoints', tuple(waypoints))
        object.__setattr__(self, 'orientations', 
                          tuple(orientations) if orientations else None)
    
    @property
    def num_waypoints(self) -> int:
        """Number of waypoints."""
        return len(self.waypoints)
    
    def to_joint_trajectory(
        self,
        manipulator,
        times: List[float] = None
    ) -> Optional[JointTrajectory]:
        """
        Convert workspace path to joint trajectory using IK.
        
        Args:
            manipulator: SerialManipulator instance
            times: Time at each waypoint (if None, uses unit spacing)
        
        Returns:
            JointTrajectory or None if IK fails
        """
        if times is None:
            times = [float(i) for i in range(len(self.waypoints))]
        
        if len(times) != len(self.waypoints):
            raise ValueError("Number of times must match number of waypoints")
        
        trajectory_waypoints = []
        current_config = None
        
        for i, (point, time) in enumerate(zip(self.waypoints, times)):
            # Solve IK for this waypoint
            joint_config = manipulator.inverse_kinematics_numeric(
                point,
                initial_guess=current_config
            )
            
            if joint_config is None:
                return None  # IK failed
            
            trajectory_waypoints.append(
                TrajectoryWaypoint(time, joint_config)
            )
            current_config = joint_config
        
        return JointTrajectory(trajectory_waypoints)
