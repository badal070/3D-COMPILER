"""
Compound motions module for mathlib.

Hierarchical motion composition, easing functions, and trajectory blending
for complex animations.
"""

from mathlib.motions.compound import (
    Motion,
    SequentialMotion,
    ParallelMotion,
    OscillatoryMotion,
    DampedMotion,
    PeriodicMotion,
    EasedMotion,
    ConditionalMotion,
    MotionBlender
)
from mathlib.motions.trajectories import SplinePath

__all__ = [
    'Motion',
    'SequentialMotion',
    'ParallelMotion',
    'OscillatoryMotion',
    'DampedMotion',
    'PeriodicMotion',
    'EasedMotion',
    'ConditionalMotion',
    'MotionBlender',
    'SplinePath'
]
