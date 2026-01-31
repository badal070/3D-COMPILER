"""
Compound motion compositions.

Sequential, parallel, oscillatory, damped, and conditional motions
for hierarchical animation control.
"""

from dataclasses import dataclass
from typing import List, Callable, Optional, Tuple
from mathlib.core.scalar import Scalar
from mathlib.core.vector import Vector
from mathlib.geometry.point import Point
from mathlib.transforms.rotation import Rotation
from mathlib.errors.math_errors import InvalidOperationError
import math


@dataclass(frozen=True)
class Motion:
    """Base motion primitive."""
    
    motion_id: str
    motion_type: str
    target_id: str
    parameters: dict
    
    def __init__(self, motion_id: str, motion_type: str, 
                 target_id: str, parameters: dict = None):
        if parameters is None:
            parameters = {}
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'motion_type', motion_type)
        object.__setattr__(self, 'target_id', target_id)
        object.__setattr__(self, 'parameters', dict(parameters))
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """
        Evaluate motion at given time.
        
        Returns: (position_offset, rotation)
        """
        raise NotImplementedError("Motion evaluation not implemented")


@dataclass(frozen=True)
class SequentialMotion:
    """Execute motions in sequence."""
    
    motion_id: str
    motions: Tuple[Motion, ...]
    durations: Tuple[float, ...]
    blend_time: float
    
    def __init__(
        self,
        motion_id: str,
        motions: List[Motion],
        durations: List[float] = None,
        blend_time: float = 0.0
    ):
        if len(motions) < 2:
            raise ValueError("Sequential motion needs at least 2 motions")
        
        if durations is None:
            durations = [1.0] * len(motions)
        elif len(durations) != len(motions):
            raise ValueError("Must have one duration per motion")
        
        for d in durations:
            if d <= 0:
                raise ValueError("Durations must be positive")
        
        if blend_time < 0:
            raise ValueError("Blend time must be non-negative")
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'motions', tuple(motions))
        object.__setattr__(self, 'durations', tuple(durations))
        object.__setattr__(self, 'blend_time', blend_time)
    
    @property
    def total_duration(self) -> float:
        """Total duration of sequential motion."""
        return sum(self.durations)
    
    def active_motion(self, time: float) -> Tuple[int, float]:
        """
        Get active motion index and local time.
        
        Returns: (motion_index, local_time)
        """
        if time < 0:
            return (0, 0.0)
        
        cumulative_time = 0.0
        for i, duration in enumerate(self.durations):
            if time < cumulative_time + duration:
                return (i, time - cumulative_time)
            cumulative_time += duration
        
        # After all motions
        return (len(self.motions) - 1, self.durations[-1])
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate sequential motion with optional blending."""
        motion_idx, local_time = self.active_motion(time)
        
        # No blending
        if self.blend_time == 0:
            return self.motions[motion_idx].evaluate(local_time, state)
        
        # Check if in blend region
        if motion_idx < len(self.motions) - 1:
            transition_time = self.durations[motion_idx]
            if local_time > transition_time - self.blend_time:
                # In blend zone
                blend_progress = (local_time - (transition_time - self.blend_time)) / self.blend_time
                
                pos1, rot1 = self.motions[motion_idx].evaluate(local_time, state)
                pos2, rot2 = self.motions[motion_idx + 1].evaluate(0.0, state)
                
                # Linear blend
                blended_pos = pos1 + (pos2 - pos1) * blend_progress
                # Rotation blending would need slerp (simplified here)
                
                return (blended_pos, rot1)
        
        return self.motions[motion_idx].evaluate(local_time, state)


@dataclass(frozen=True)
class ParallelMotion:
    """Execute multiple motions simultaneously."""
    
    motion_id: str
    motions: Tuple[Motion, ...]
    weights: Tuple[float, ...]
    
    def __init__(
        self,
        motion_id: str,
        motions: List[Motion],
        weights: List[float] = None
    ):
        if len(motions) < 2:
            raise ValueError("Parallel motion needs at least 2 motions")
        
        if weights is None:
            weights = [1.0 / len(motions)] * len(motions)
        elif len(weights) != len(motions):
            raise ValueError("Must have one weight per motion")
        
        # Normalize weights
        total_weight = sum(weights)
        if total_weight <= 0:
            raise ValueError("Total weight must be positive")
        
        normalized_weights = [w / total_weight for w in weights]
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'motions', tuple(motions))
        object.__setattr__(self, 'weights', tuple(normalized_weights))
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate all motions and combine with weights."""
        combined_pos = Vector.zero(3)
        
        # Combine positions with weights
        for motion, weight in zip(self.motions, self.weights):
            pos, rot = motion.evaluate(time, state)
            combined_pos = combined_pos + pos * weight
        
        # Rotation blending needs special handling (simplified)
        first_rot = self.motions[0].evaluate(time, state)[1]
        
        return (combined_pos, first_rot)


@dataclass(frozen=True)
class OscillatoryMotion:
    """Wrap motion with oscillation."""
    
    motion_id: str
    base_motion: Motion
    frequency: float
    amplitude: float
    phase_offset: float
    axis: Vector
    
    def __init__(
        self,
        motion_id: str,
        base_motion: Motion,
        frequency: float,
        amplitude: float = 1.0,
        phase_offset: float = 0.0,
        axis: Vector = None
    ):
        if frequency <= 0:
            raise ValueError("Frequency must be positive")
        if amplitude < 0:
            raise ValueError("Amplitude must be non-negative")
        
        if axis is None:
            axis = Vector([0, 1, 0])
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'base_motion', base_motion)
        object.__setattr__(self, 'frequency', frequency)
        object.__setattr__(self, 'amplitude', amplitude)
        object.__setattr__(self, 'phase_offset', phase_offset)
        object.__setattr__(self, 'axis', axis.normalize())
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate base motion with oscillation."""
        base_pos, base_rot = self.base_motion.evaluate(time, state)
        
        # Oscillation: A * sin(2πft + φ)
        omega = 2 * math.pi * self.frequency
        oscillation = self.amplitude * math.sin(omega * time + self.phase_offset)
        
        # Add oscillation along axis
        osc_offset = self.axis * oscillation
        final_pos = base_pos + osc_offset
        
        return (final_pos, base_rot)
    
    def period(self) -> float:
        """Get oscillation period: T = 1/f."""
        return 1.0 / self.frequency


@dataclass(frozen=True)
class DampedMotion:
    """Apply exponential damping to motion."""
    
    motion_id: str
    base_motion: Motion
    damping_coefficient: float
    
    def __init__(
        self,
        motion_id: str,
        base_motion: Motion,
        damping_coefficient: float
    ):
        if damping_coefficient < 0:
            raise ValueError("Damping coefficient must be non-negative")
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'base_motion', base_motion)
        object.__setattr__(self, 'damping_coefficient', damping_coefficient)
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate motion with exponential damping."""
        base_pos, base_rot = self.base_motion.evaluate(time, state)
        
        # Damping factor: e^(-ct)
        damping_factor = math.exp(-self.damping_coefficient * time)
        
        damped_pos = base_pos * damping_factor
        
        return (damped_pos, base_rot)


@dataclass(frozen=True)
class PeriodicMotion:
    """Repeat motion periodically."""
    
    motion_id: str
    base_motion: Motion
    period: float
    repeat_count: Optional[int]
    
    def __init__(
        self,
        motion_id: str,
        base_motion: Motion,
        period: float,
        repeat_count: int = None
    ):
        if period <= 0:
            raise ValueError("Period must be positive")
        
        if repeat_count is not None and repeat_count < 1:
            raise ValueError("Repeat count must be at least 1")
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'base_motion', base_motion)
        object.__setattr__(self, 'period', period)
        object.__setattr__(self, 'repeat_count', repeat_count)
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate motion with periodic repetition."""
        # Map time to one period
        local_time = time % self.period
        
        # Check if beyond repeat limit
        if self.repeat_count is not None:
            cycle = int(time / self.period)
            if cycle >= self.repeat_count:
                # Hold last position
                local_time = self.period
        
        return self.base_motion.evaluate(local_time, state)


@dataclass(frozen=True)
class EasedMotion:
    """Apply easing function to motion."""
    
    motion_id: str
    base_motion: Motion
    easing_function: Callable[[float], float]
    
    def __init__(
        self,
        motion_id: str,
        base_motion: Motion,
        easing_function: Callable[[float], float] = None
    ):
        if easing_function is None:
            easing_function = EasedMotion.ease_in_out_cubic
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'base_motion', base_motion)
        object.__setattr__(self, 'easing_function', easing_function)
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate motion with easing."""
        # Apply easing to time parameter
        eased_time = self.easing_function(time)
        return self.base_motion.evaluate(eased_time, state)
    
    @staticmethod
    def ease_in_out_cubic(t: float) -> float:
        """Cubic ease-in-out: slow start and end."""
        if t < 0.5:
            return 4 * t * t * t
        else:
            return 1 - pow(-2 * t + 2, 3) / 2
    
    @staticmethod
    def ease_in_quad(t: float) -> float:
        """Quadratic ease-in: slow start."""
        return t * t
    
    @staticmethod
    def ease_out_quad(t: float) -> float:
        """Quadratic ease-out: slow end."""
        return 1 - (1 - t) * (1 - t)
    
    @staticmethod
    def ease_elastic(t: float) -> float:
        """Elastic easing: overshoot and settle."""
        if t == 0 or t == 1:
            return t
        
        c4 = (2 * math.pi) / 3
        return pow(2, -10 * t) * math.sin((t * 10 - 0.75) * c4) + 1


@dataclass(frozen=True)
class ConditionalMotion:
    """Choose motion based on condition."""
    
    motion_id: str
    condition: Callable[[dict], bool]
    motion_true: Motion
    motion_false: Motion
    
    def __init__(
        self,
        motion_id: str,
        condition: Callable[[dict], bool],
        motion_true: Motion,
        motion_false: Motion
    ):
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'condition', condition)
        object.__setattr__(self, 'motion_true', motion_true)
        object.__setattr__(self, 'motion_false', motion_false)
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate motion based on condition."""
        if self.condition(state):
            return self.motion_true.evaluate(time, state)
        else:
            return self.motion_false.evaluate(time, state)


@dataclass(frozen=True)
class MotionBlender:
    """Blend multiple motions with time-varying weights."""
    
    motion_id: str
    motions: Tuple[Motion, ...]
    blend_weights: Callable[[float], List[float]]
    
    def __init__(
        self,
        motion_id: str,
        motions: List[Motion],
        blend_weights: Callable[[float], List[float]]
    ):
        if len(motions) < 2:
            raise ValueError("Blender needs at least 2 motions")
        
        object.__setattr__(self, 'motion_id', motion_id)
        object.__setattr__(self, 'motions', tuple(motions))
        object.__setattr__(self, 'blend_weights', blend_weights)
    
    def evaluate(self, time: float, state: dict) -> Tuple[Vector, Rotation]:
        """Evaluate motions with time-varying blend."""
        weights = self.blend_weights(time)
        
        if len(weights) != len(self.motions):
            raise ValueError("Blend weights must match number of motions")
        
        # Normalize weights
        total_weight = sum(weights)
        if total_weight <= 0:
            total_weight = 1.0
        
        weights = [w / total_weight for w in weights]
        
        # Blend positions
        combined_pos = Vector.zero(3)
        for motion, weight in zip(self.motions, weights):
            pos, _ = motion.evaluate(time, state)
            combined_pos = combined_pos + pos * weight
        
        # Use rotation from dominant motion
        dominant_idx = weights.index(max(weights))
        _, dominant_rot = self.motions[dominant_idx].evaluate(time, state)
        
        return (combined_pos, dominant_rot)
