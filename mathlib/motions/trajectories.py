"""
Trajectory path representations.

Spline paths with control points and TCB (Tension-Continuity-Bias)
parameters for smooth animation curves.
"""

from dataclasses import dataclass
from typing import List, Tuple
from mathlib.geometry.point import Point
from mathlib.core.vector import Vector
from mathlib.errors.math_errors import InvalidOperationError
import math


@dataclass(frozen=True)
class SplinePath:
    """Smooth path through control points using splines."""
    
    control_points: Tuple[Point, ...]
    tension: float
    continuity: float
    bias: float
    
    def __init__(
        self,
        control_points: List[Point],
        tension: float = 0.0,
        continuity: float = 0.0,
        bias: float = 0.0
    ):
        if len(control_points) < 2:
            raise ValueError("Spline needs at least 2 control points")
        
        # Verify all points have same dimension
        dim = control_points[0].dimension
        for pt in control_points:
            if pt.dimension != dim:
                raise ValueError("All control points must have same dimension")
        
        # TCB parameters typically in [-1, 1]
        if not (-1 <= tension <= 1):
            raise ValueError("Tension must be in [-1, 1]")
        if not (-1 <= continuity <= 1):
            raise ValueError("Continuity must be in [-1, 1]")
        if not (-1 <= bias <= 1):
            raise ValueError("Bias must be in [-1, 1]")
        
        object.__setattr__(self, 'control_points', tuple(control_points))
        object.__setattr__(self, 'tension', tension)
        object.__setattr__(self, 'continuity', continuity)
        object.__setattr__(self, 'bias', bias)
    
    @property
    def num_segments(self) -> int:
        """Number of curve segments."""
        return len(self.control_points) - 1
    
    def evaluate(self, t: float) -> Point:
        """
        Evaluate spline at parameter t ∈ [0, 1].
        
        Uses Kochanek-Bartels (TCB) spline interpolation.
        """
        if not (0 <= t <= 1):
            raise ValueError("Parameter t must be in [0, 1]")
        
        # Map t to segment
        segment_t = t * self.num_segments
        segment_idx = min(int(segment_t), self.num_segments - 1)
        local_t = segment_t - segment_idx
        
        # Get four control points for cubic interpolation
        p0_idx = max(0, segment_idx - 1)
        p1_idx = segment_idx
        p2_idx = segment_idx + 1
        p3_idx = min(len(self.control_points) - 1, segment_idx + 2)
        
        p0 = self.control_points[p0_idx]
        p1 = self.control_points[p1_idx]
        p2 = self.control_points[p2_idx]
        p3 = self.control_points[p3_idx]
        
        # Compute tangents using TCB parameters
        t1 = self._compute_tangent(p0, p1, p2, at_start=False)
        t2 = self._compute_tangent(p1, p2, p3, at_start=True)
        
        # Hermite interpolation
        result = self._hermite_interpolate(p1, p2, t1, t2, local_t)
        
        return result
    
    def _compute_tangent(
        self,
        p_prev: Point,
        p_curr: Point,
        p_next: Point,
        at_start: bool
    ) -> Vector:
        """
        Compute TCB tangent at a control point.
        
        Args:
            p_prev, p_curr, p_next: Three consecutive control points
            at_start: True for incoming tangent, False for outgoing
        """
        # Tension, continuity, bias
        T = self.tension
        C = self.continuity
        B = self.bias
        
        # Compute direction vectors
        d_in = p_curr.position - p_prev.position
        d_out = p_next.position - p_curr.position
        
        if at_start:
            # Incoming tangent
            a = (1 - T) * (1 + C) * (1 + B) / 2
            b = (1 - T) * (1 - C) * (1 - B) / 2
            tangent = d_in * a + d_out * b
        else:
            # Outgoing tangent
            a = (1 - T) * (1 - C) * (1 + B) / 2
            b = (1 - T) * (1 + C) * (1 - B) / 2
            tangent = d_in * a + d_out * b
        
        return tangent
    
    def _hermite_interpolate(
        self,
        p1: Point,
        p2: Point,
        t1: Vector,
        t2: Vector,
        t: float
    ) -> Point:
        """
        Hermite interpolation between two points with tangents.
        
        H(t) = h00(t)*p1 + h10(t)*t1 + h01(t)*p2 + h11(t)*t2
        """
        # Hermite basis functions
        t2 = t * t
        t3 = t2 * t
        
        h00 = 2*t3 - 3*t2 + 1
        h10 = t3 - 2*t2 + t
        h01 = -2*t3 + 3*t2
        h11 = t3 - t2
        
        # Interpolate position
        result = (
            p1.position * h00 +
            t1 * h10 +
            p2.position * h01 +
            t2 * h11
        )
        
        return Point(list(result.components), result.unit)
    
    def arc_length(self, num_samples: int = 100) -> float:
        """
        Estimate arc length using numerical integration.
        
        Args:
            num_samples: Number of samples for approximation
        
        Returns:
            Approximate arc length
        """
        total_length = 0.0
        prev_point = self.evaluate(0.0)
        
        for i in range(1, num_samples + 1):
            t = i / num_samples
            curr_point = self.evaluate(t)
            
            segment_length = prev_point.distance_to(curr_point).value
            total_length += segment_length
            
            prev_point = curr_point
        
        return total_length
    
    @staticmethod
    def catmull_rom(control_points: List[Point]) -> 'SplinePath':
        """
        Create Catmull-Rom spline (T=0, C=0, B=0).
        
        This is a common default spline that passes through all control points.
        """
        return SplinePath(control_points, tension=0.0, continuity=0.0, bias=0.0)
    
    @staticmethod
    def cardinal_spline(control_points: List[Point], tension: float = 0.5) -> 'SplinePath':
        """
        Create cardinal spline (C=0, B=0, adjustable tension).
        
        Args:
            tension: Controls tightness of curve (0 = Catmull-Rom, 1 = straight lines)
        """
        return SplinePath(control_points, tension=tension, continuity=0.0, bias=0.0)
