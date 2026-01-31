"""
Motion Graph - Hierarchical motion composition system.

Supports nested motions, temporal dependencies, and complex animation sequences.
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Callable, Set
from enum import Enum
import math

from mathlib.core.scalar import Scalar
from mathlib.transforms.affine import AffineTransform
from mathlib.geometry.point import Point
from mathlib.errors.validation_errors import CircularDependencyError


class MotionType(Enum):
    """Types of motion primitives."""
    ROTATION = "rotation"
    TRANSLATION = "translation"
    SCALE = "scale"
    OSCILLATION = "oscillation"
    ORBITAL = "orbital"
    PARAMETRIC = "parametric"
    COMPOSITE = "composite"


class EasingFunction(Enum):
    """Easing functions for smooth interpolation."""
    LINEAR = "linear"
    EASE_IN_QUAD = "ease_in_quad"
    EASE_OUT_QUAD = "ease_out_quad"
    EASE_IN_OUT_QUAD = "ease_in_out_quad"
    EASE_IN_CUBIC = "ease_in_cubic"
    EASE_OUT_CUBIC = "ease_out_cubic"
    EASE_IN_OUT_CUBIC = "ease_in_out_cubic"
    EASE_IN_SINE = "ease_in_sine"
    EASE_OUT_SINE = "ease_out_sine"
    EASE_IN_OUT_SINE = "ease_in_out_sine"
    ELASTIC = "elastic"
    BOUNCE = "bounce"


def apply_easing(t: float, easing: EasingFunction) -> float:
    """Apply easing function to normalized time [0, 1]."""
    t = max(0.0, min(1.0, t))  # Clamp to [0, 1]
    
    if easing == EasingFunction.LINEAR:
        return t
    elif easing == EasingFunction.EASE_IN_QUAD:
        return t * t
    elif easing == EasingFunction.EASE_OUT_QUAD:
        return 1 - (1 - t) * (1 - t)
    elif easing == EasingFunction.EASE_IN_OUT_QUAD:
        return 2 * t * t if t < 0.5 else 1 - (-2 * t + 2) ** 2 / 2
    elif easing == EasingFunction.EASE_IN_CUBIC:
        return t * t * t
    elif easing == EasingFunction.EASE_OUT_CUBIC:
        return 1 - (1 - t) ** 3
    elif easing == EasingFunction.EASE_IN_OUT_CUBIC:
        return 4 * t * t * t if t < 0.5 else 1 - (-2 * t + 2) ** 3 / 2
    elif easing == EasingFunction.EASE_IN_SINE:
        return 1 - math.cos(t * math.pi / 2)
    elif easing == EasingFunction.EASE_OUT_SINE:
        return math.sin(t * math.pi / 2)
    elif easing == EasingFunction.EASE_IN_OUT_SINE:
        return -(math.cos(math.pi * t) - 1) / 2
    elif easing == EasingFunction.ELASTIC:
        if t == 0 or t == 1:
            return t
        return -(2 ** (10 * (t - 1))) * math.sin((t - 1.1) * 5 * math.pi)
    elif easing == EasingFunction.BOUNCE:
        if t < 1 / 2.75:
            return 7.5625 * t * t
        elif t < 2 / 2.75:
            t -= 1.5 / 2.75
            return 7.5625 * t * t + 0.75
        elif t < 2.5 / 2.75:
            t -= 2.25 / 2.75
            return 7.5625 * t * t + 0.9375
        else:
            t -= 2.625 / 2.75
            return 7.5625 * t * t + 0.984375
    
    return t


@dataclass
class MotionNode:
    """Node in the motion graph representing a single motion or transformation."""
    
    name: str
    motion_type: MotionType
    parameters: Dict[str, any]
    start_time: float = 0.0
    duration: float = 1.0
    easing: EasingFunction = EasingFunction.LINEAR
    parent: Optional['MotionNode'] = None
    children: List['MotionNode'] = field(default_factory=list)
    dependencies: List['MotionNode'] = field(default_factory=list)
    enabled: bool = True
    loop: bool = False
    loop_count: int = -1  # -1 for infinite
    
    def add_child(self, child: 'MotionNode'):
        """Add a child motion that moves relative to this motion."""
        child.parent = self
        self.children.append(child)
    
    def add_dependency(self, dependency: 'MotionNode'):
        """Add a motion that must complete before this one starts."""
        self.dependencies.append(dependency)
    
    def get_normalized_time(self, global_time: float) -> float:
        """Get normalized time [0, 1] for this motion at global time."""
        if not self.enabled:
            return 0.0
        
        # Account for dependencies
        effective_start = self.start_time
        for dep in self.dependencies:
            dep_end = dep.start_time + dep.duration
            if dep_end > effective_start:
                effective_start = dep_end
        
        # Handle looping
        if self.loop:
            if self.loop_count > 0:
                # Finite loops
                total_duration = self.duration * self.loop_count
                if global_time < effective_start or global_time > effective_start + total_duration:
                    return 1.0 if global_time > effective_start else 0.0
                local_time = (global_time - effective_start) % self.duration
            else:
                # Infinite loop
                if global_time < effective_start:
                    return 0.0
                local_time = (global_time - effective_start) % self.duration
        else:
            # No looping
            if global_time < effective_start:
                return 0.0
            if global_time > effective_start + self.duration:
                return 1.0
            local_time = global_time - effective_start
        
        t = local_time / self.duration if self.duration > 0 else 1.0
        return apply_easing(t, self.easing)
    
    def evaluate(self, global_time: float) -> AffineTransform:
        """Evaluate the motion at a given global time."""
        from mathlib.transforms.rotation import Rotation
        from mathlib.transforms.translation import Translation
        from mathlib.transforms.scale import Scale
        from mathlib.core.vector import Vector
        from mathlib.core.matrix import Matrix
        
        t = self.get_normalized_time(global_time)
        
        if self.motion_type == MotionType.ROTATION:
            axis = self.parameters.get('axis', 'z')
            angle_total = self.parameters.get('angle', 0.0)
            angle_current = Scalar(angle_total * t, self.parameters.get('unit', RADIAN))
            rotation = Rotation(axis, angle_current)
            return AffineTransform(rotation.as_matrix())
        
        elif self.motion_type == MotionType.TRANSLATION:
            direction = self.parameters.get('direction', Vector([0, 0, 0]))
            distance_total = self.parameters.get('distance', 0.0)
            offset = direction * (distance_total * t)
            return AffineTransform(Matrix.identity(3), offset)
        
        elif self.motion_type == MotionType.SCALE:
            scale_start = self.parameters.get('scale_start', [1.0, 1.0, 1.0])
            scale_end = self.parameters.get('scale_end', [1.0, 1.0, 1.0])
            scale_current = [
                scale_start[i] + (scale_end[i] - scale_start[i]) * t
                for i in range(3)
            ]
            scale_transform = Scale(scale_current)
            return AffineTransform(scale_transform.as_matrix())
        
        elif self.motion_type == MotionType.OSCILLATION:
            # Simple harmonic motion
            amplitude = self.parameters.get('amplitude', 1.0)
            frequency = self.parameters.get('frequency', 1.0)
            direction = self.parameters.get('direction', Vector([0, 1, 0]))
            phase = self.parameters.get('phase', 0.0)
            
            displacement = amplitude * math.sin(2 * math.pi * frequency * t + phase)
            offset = direction * displacement
            return AffineTransform(Matrix.identity(3), offset)
        
        elif self.motion_type == MotionType.ORBITAL:
            # Orbital motion around a point
            radius = self.parameters.get('radius', 1.0)
            angular_speed = self.parameters.get('angular_speed', 1.0)
            axis = self.parameters.get('axis', 'z')
            center = self.parameters.get('center', Vector([0, 0, 0]))
            
            angle = angular_speed * t
            if axis == 'z':
                x = radius * math.cos(angle)
                y = radius * math.sin(angle)
                offset = Vector([x, y, 0]) + center
            elif axis == 'y':
                x = radius * math.cos(angle)
                z = radius * math.sin(angle)
                offset = Vector([x, 0, z]) + center
            else:  # x
                y = radius * math.cos(angle)
                z = radius * math.sin(angle)
                offset = Vector([0, y, z]) + center
            
            return AffineTransform(Matrix.identity(3), offset)
        
        elif self.motion_type == MotionType.PARAMETRIC:
            # Custom parametric function
            func = self.parameters.get('function')
            if func and callable(func):
                transform = func(t, self.parameters)
                return transform if isinstance(transform, AffineTransform) else AffineTransform(Matrix.identity(3))
        
        # Default: identity transform
        return AffineTransform(Matrix.identity(3))


@dataclass
class MotionGraph:
    """Graph of interconnected motions forming a complex animation."""
    
    nodes: Dict[str, MotionNode] = field(default_factory=dict)
    root_nodes: List[MotionNode] = field(default_factory=list)
    
    def add_node(self, node: MotionNode):
        """Add a motion node to the graph."""
        self.nodes[node.name] = node
        if node.parent is None:
            self.root_nodes.append(node)
    
    def add_nested_motion(self, parent_name: str, child: MotionNode):
        """Add a nested motion under a parent."""
        if parent_name not in self.nodes:
            raise ValueError(f"Parent node '{parent_name}' not found")
        
        parent = self.nodes[parent_name]
        parent.add_child(child)
        self.add_node(child)
    
    def validate(self):
        """Validate the motion graph for cycles and consistency."""
        # Check for circular dependencies
        visited = set()
        rec_stack = set()
        
        for node in self.nodes.values():
            if node.name not in visited:
                if self._has_cycle(node, visited, rec_stack):
                    raise CircularDependencyError(list(rec_stack))
    
    def _has_cycle(self, node: MotionNode, visited: Set[str], rec_stack: Set[str]) -> bool:
        """DFS-based cycle detection."""
        visited.add(node.name)
        rec_stack.add(node.name)
        
        # Check dependencies
        for dep in node.dependencies:
            if dep.name not in visited:
                if self._has_cycle(dep, visited, rec_stack):
                    return True
            elif dep.name in rec_stack:
                return True
        
        # Check children (parent-child is not a cycle, but we track it)
        for child in node.children:
            if child.name not in visited:
                if self._has_cycle(child, visited, rec_stack):
                    return True
        
        rec_stack.remove(node.name)
        return False
    
    def evaluate_node(self, node_name: str, global_time: float) -> AffineTransform:
        """Evaluate a specific node and its ancestors at global time."""
        if node_name not in self.nodes:
            raise ValueError(f"Node '{node_name}' not found")
        
        node = self.nodes[node_name]
        
        # Build transform chain from root to this node
        transforms = []
        current = node
        while current is not None:
            transforms.insert(0, current.evaluate(global_time))
            current = current.parent
        
        # Compose transforms
        if not transforms:
            from mathlib.core.matrix import Matrix
            return AffineTransform(Matrix.identity(3))
        
        result = transforms[0]
        for t in transforms[1:]:
            result = result @ t
        
        return result
    
    def get_transform_at_time(self, node_name: str, global_time: float) -> AffineTransform:
        """Get the complete transform for a node at a specific time."""
        return self.evaluate_node(node_name, global_time)
    
    def animate(self, node_name: str, start_time: float, end_time: float, 
                time_step: float = 0.016) -> List[tuple]:
        """Generate animation frames for a node over a time range.
        
        Returns list of (time, transform) tuples.
        """
        frames = []
        t = start_time
        while t <= end_time:
            transform = self.evaluate_node(node_name, t)
            frames.append((t, transform))
            t += time_step
        
        return frames
