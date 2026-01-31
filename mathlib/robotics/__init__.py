"""
Robotics module for mathlib.

Forward/inverse kinematics, joint control, trajectory planning,
and workspace analysis for robotic manipulators.
"""

from mathlib.robotics.kinematics import (
    JointLimit,
    RevoluteJointKinematics,
    PrismaticJointKinematics,
    RobotLink,
    SerialManipulator
)
from mathlib.robotics.trajectories import (
    TrajectoryWaypoint,
    JointTrajectory,
    WorkspacePath
)

__all__ = [
    'JointLimit',
    'RevoluteJointKinematics',
    'PrismaticJointKinematics',
    'RobotLink',
    'SerialManipulator',
    'TrajectoryWaypoint',
    'JointTrajectory',
    'WorkspacePath'
]
