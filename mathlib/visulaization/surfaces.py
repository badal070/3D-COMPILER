"""
Mathematical Visualization - Tools for visualizing functions, surfaces, and fields.

Supports parametric surfaces, vector fields, implicit surfaces, and more.
"""

from dataclasses import dataclass
from typing import Callable, List, Tuple
import math

from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
from mathlib.calculus.curves import ParametricCurve


@dataclass
class ParametricSurface:
    """Parametric surface in 3D: S(u,v) = (x(u,v), y(u,v), z(u,v))."""
    
    function: Callable[[float, float], Point]
    u_range: Tuple[float, float]
    v_range: Tuple[float, float]
    
    def evaluate(self, u: float, v: float) -> Point:
        """Evaluate surface at parameters (u, v)."""
        return self.function(u, v)
    
    def sample_grid(self, u_steps: int, v_steps: int) -> List[List[Point]]:
        """Sample surface on a regular grid."""
        u_min, u_max = self.u_range
        v_min, v_max = self.v_range
        
        grid = []
        for i in range(u_steps):
            row = []
            u = u_min + (u_max - u_min) * i / (u_steps - 1)
            
            for j in range(v_steps):
                v = v_min + (v_max - v_min) * j / (v_steps - 1)
                point = self.evaluate(u, v)
                row.append(point)
            
            grid.append(row)
        
        return grid
    
    def normal_at(self, u: float, v: float, h: float = 1e-5) -> Vector:
        """Calculate normal vector at (u, v) using numerical derivatives."""
        p = self.evaluate(u, v)
        
        # Partial derivatives
        p_u_plus = self.evaluate(u + h, v)
        p_v_plus = self.evaluate(u, v + h)
        
        # Tangent vectors
        du = p_u_plus.position - p.position
        dv = p_v_plus.position - p.position
        
        # Normal is cross product
        normal = du.cross(dv)
        
        if not normal.is_zero():
            return normal.normalize()
        return Vector([0, 0, 1])


class CommonSurfaces:
    """Factory for common parametric surfaces."""
    
    @staticmethod
    def sphere(radius: float = 1.0, center: Point = None) -> ParametricSurface:
        """Create a sphere."""
        if center is None:
            center = Point([0, 0, 0])
        
        def sphere_func(u: float, v: float) -> Point:
            # u: [0, 2π], v: [0, π]
            x = center.x + radius * math.sin(v) * math.cos(u)
            y = center.y + radius * math.sin(v) * math.sin(u)
            z = center.z + radius * math.cos(v)
            return Point([x, y, z])
        
        return ParametricSurface(sphere_func, (0, 2*math.pi), (0, math.pi))
    
    @staticmethod
    def torus(major_radius: float = 2.0, minor_radius: float = 0.5) -> ParametricSurface:
        """Create a torus (donut shape)."""
        def torus_func(u: float, v: float) -> Point:
            # u, v: [0, 2π]
            x = (major_radius + minor_radius * math.cos(v)) * math.cos(u)
            y = (major_radius + minor_radius * math.cos(v)) * math.sin(u)
            z = minor_radius * math.sin(v)
            return Point([x, y, z])
        
        return ParametricSurface(torus_func, (0, 2*math.pi), (0, 2*math.pi))
    
    @staticmethod
    def mobius_strip(width: float = 1.0, radius: float = 2.0) -> ParametricSurface:
        """Create a Möbius strip."""
        def mobius_func(u: float, v: float) -> Point:
            # u: [0, 2π], v: [-width/2, width/2]
            x = (radius + v * math.cos(u/2)) * math.cos(u)
            y = (radius + v * math.cos(u/2)) * math.sin(u)
            z = v * math.sin(u/2)
            return Point([x, y, z])
        
        return ParametricSurface(mobius_func, (0, 2*math.pi), (-width/2, width/2))
    
    @staticmethod
    def klein_bottle(scale: float = 1.0) -> ParametricSurface:
        """Create a Klein bottle."""
        def klein_func(u: float, v: float) -> Point:
            # u, v: [0, 2π]
            r = 4 * (1 - math.cos(u) / 2)
            
            if u < math.pi:
                x = 6 * math.cos(u) * (1 + math.sin(u)) + r * math.cos(u) * math.cos(v)
                y = 16 * math.sin(u) + r * math.sin(u) * math.cos(v)
            else:
                x = 6 * math.cos(u) * (1 + math.sin(u)) + r * math.cos(v + math.pi)
                y = 16 * math.sin(u)
            
            z = r * math.sin(v)
            
            return Point([x * scale, y * scale, z * scale])
        
        return ParametricSurface(klein_func, (0, 2*math.pi), (0, 2*math.pi))
    
    @staticmethod
    def paraboloid(a: float = 1.0, b: float = 1.0) -> ParametricSurface:
        """Create an elliptic paraboloid z = x²/a² + y²/b²."""
        def paraboloid_func(u: float, v: float) -> Point:
            # u: radius, v: angle
            x = u * math.cos(v)
            y = u * math.sin(v)
            z = (x*x)/(a*a) + (y*y)/(b*b)
            return Point([x, y, z])
        
        return ParametricSurface(paraboloid_func, (0, 3), (0, 2*math.pi))
    
    @staticmethod
    def helicoid(pitch: float = 1.0, radius: float = 1.0) -> ParametricSurface:
        """Create a helicoid (spiral ramp)."""
        def helicoid_func(u: float, v: float) -> Point:
            # u: radial, v: angular
            x = u * math.cos(v)
            y = u * math.sin(v)
            z = pitch * v
            return Point([x, y, z])
        
        return ParametricSurface(helicoid_func, (0, radius), (0, 4*math.pi))


@dataclass
class VectorField:
    """Vector field F: R³ → R³."""
    
    function: Callable[[Point], Vector]
    
    def evaluate(self, point: Point) -> Vector:
        """Evaluate vector field at a point."""
        return self.function(point)
    
    def sample_grid(self, bounds: Tuple[float, float, float, float, float, float],
                    steps: Tuple[int, int, int]) -> List[Tuple[Point, Vector]]:
        """Sample vector field on a 3D grid.
        
        Args:
            bounds: (x_min, x_max, y_min, y_max, z_min, z_max)
            steps: (x_steps, y_steps, z_steps)
        """
        x_min, x_max, y_min, y_max, z_min, z_max = bounds
        x_steps, y_steps, z_steps = steps
        
        samples = []
        
        for i in range(x_steps):
            x = x_min + (x_max - x_min) * i / (x_steps - 1) if x_steps > 1 else x_min
            
            for j in range(y_steps):
                y = y_min + (y_max - y_min) * j / (y_steps - 1) if y_steps > 1 else y_min
                
                for k in range(z_steps):
                    z = z_min + (z_max - z_min) * k / (z_steps - 1) if z_steps > 1 else z_min
                    
                    point = Point([x, y, z])
                    vector = self.evaluate(point)
                    samples.append((point, vector))
        
        return samples
    
    def compute_streamline(self, start: Point, steps: int = 100, 
                          step_size: float = 0.1) -> List[Point]:
        """Compute a streamline starting from a point."""
        streamline = [start]
        current = start
        
        for _ in range(steps):
            # Evaluate field at current point
            velocity = self.evaluate(current)
            
            # Euler step
            displacement = velocity * step_size
            next_point = current.translate(displacement)
            
            streamline.append(next_point)
            current = next_point
        
        return streamline


class CommonVectorFields:
    """Factory for common vector fields."""
    
    @staticmethod
    def radial(strength: float = 1.0) -> VectorField:
        """Radial field pointing outward from origin."""
        def radial_func(p: Point) -> Vector:
            r = p.position.norm().value
            if r < 1e-10:
                return Vector.zero(3)
            return p.position * (strength / r)
        
        return VectorField(radial_func)
    
    @staticmethod
    def rotation(axis: str = 'z', strength: float = 1.0) -> VectorField:
        """Rotational field around an axis."""
        def rotation_func(p: Point) -> Vector:
            if axis == 'z':
                return Vector([-p.y, p.x, 0]) * strength
            elif axis == 'y':
                return Vector([-p.z, 0, p.x]) * strength
            else:  # x
                return Vector([0, -p.z, p.y]) * strength
        
        return VectorField(rotation_func)
    
    @staticmethod
    def gradient(scalar_field: Callable[[Point], float], h: float = 1e-5) -> VectorField:
        """Create gradient vector field from scalar field."""
        def gradient_func(p: Point) -> Vector:
            f0 = scalar_field(p)
            
            # Numerical partial derivatives
            p_dx = Point([p.x + h, p.y, p.z])
            p_dy = Point([p.x, p.y + h, p.z])
            p_dz = Point([p.x, p.y, p.z + h])
            
            df_dx = (scalar_field(p_dx) - f0) / h
            df_dy = (scalar_field(p_dy) - f0) / h
            df_dz = (scalar_field(p_dz) - f0) / h
            
            return Vector([df_dx, df_dy, df_dz])
        
        return VectorField(gradient_func)


@dataclass
class ImplicitSurface:
    """Surface defined by f(x, y, z) = 0."""
    
    function: Callable[[float, float, float], float]
    
    def sample_marching_cubes(self, bounds: Tuple[float, float, float, float, float, float],
                              resolution: int = 50) -> List[Tuple[Point, Point, Point]]:
        """
        Sample surface using marching cubes algorithm (simplified).
        
        Returns list of triangles.
        """
        # This would implement marching cubes
        # For now, return empty list (complex algorithm)
        return []
    
    def sample_points(self, bounds: Tuple[float, float, float, float, float, float],
                     samples: int = 10000, threshold: float = 0.1) -> List[Point]:
        """Sample points near the surface."""
        x_min, x_max, y_min, y_max, z_min, z_max = bounds
        
        points = []
        attempts = 0
        max_attempts = samples * 10
        
        import random
        
        while len(points) < samples and attempts < max_attempts:
            x = random.uniform(x_min, x_max)
            y = random.uniform(y_min, y_max)
            z = random.uniform(z_min, z_max)
            
            value = abs(self.function(x, y, z))
            
            if value < threshold:
                points.append(Point([x, y, z]))
            
            attempts += 1
        
        return points


class CommonImplicitSurfaces:
    """Factory for common implicit surfaces."""
    
    @staticmethod
    def sphere(radius: float = 1.0, center: Tuple[float, float, float] = (0, 0, 0)) -> ImplicitSurface:
        """Sphere: (x-cx)² + (y-cy)² + (z-cz)² - r² = 0."""
        cx, cy, cz = center
        
        def sphere_func(x: float, y: float, z: float) -> float:
            return (x-cx)**2 + (y-cy)**2 + (z-cz)**2 - radius**2
        
        return ImplicitSurface(sphere_func)
    
    @staticmethod
    def torus(major_radius: float = 2.0, minor_radius: float = 0.5) -> ImplicitSurface:
        """Torus."""
        R, r = major_radius, minor_radius
        
        def torus_func(x: float, y: float, z: float) -> float:
            return (R - math.sqrt(x*x + y*y))**2 + z*z - r*r
        
        return ImplicitSurface(torus_func)
    
    @staticmethod
    def metaball(centers: List[Tuple[float, float, float]], 
                 radii: List[float], threshold: float = 1.0) -> ImplicitSurface:
        """Metaballs (blobby objects)."""
        def metaball_func(x: float, y: float, z: float) -> float:
            total = 0.0
            for (cx, cy, cz), r in zip(centers, radii):
                dist_sq = (x-cx)**2 + (y-cy)**2 + (z-cz)**2
                total += (r*r) / (dist_sq + 1e-10)
            return threshold - total
        
        return ImplicitSurface(metaball_func)


@dataclass
class FunctionGraph:
    """Graph of a function z = f(x, y)."""
    
    function: Callable[[float, float], float]
    x_range: Tuple[float, float]
    y_range: Tuple[float, float]
    
    def sample(self, x_steps: int, y_steps: int) -> List[List[Point]]:
        """Sample function on a grid."""
        x_min, x_max = self.x_range
        y_min, y_max = self.y_range
        
        grid = []
        for i in range(x_steps):
            row = []
            x = x_min + (x_max - x_min) * i / (x_steps - 1)
            
            for j in range(y_steps):
                y = y_min + (y_max - y_min) * j / (y_steps - 1)
                z = self.function(x, y)
                row.append(Point([x, y, z]))
            
            grid.append(row)
        
        return grid
    
    def find_critical_points(self, h: float = 1e-5) -> List[Tuple[Point, str]]:
        """Find critical points (simplified numerical method)."""
        # Would implement gradient descent/ascent
        # For now, return empty list
        return []
