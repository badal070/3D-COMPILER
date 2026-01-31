# MathLib v2.0 - Extended Animation & Visualization Library

## Overview

MathLib v2.0 is a comprehensive mathematical library for creating complex educational animations in physics, chemistry, and mathematics. It extends the original strict, immutable, unit-aware foundation with powerful new modules for:

- **Hierarchical Motion Graphs** - Nested motions and complex animation sequences
- **Physics Simulation** - Realistic dynamics with forces, collisions, and constraints
- **Chemistry Visualization** - Molecular structures, bonds, and reactions
- **Mathematical Visualization** - Parametric surfaces, vector fields, and implicit surfaces

## New Features in v2.0

### 1. Animation System (`mathlib.animation.motion_graph`)

Create complex, hierarchical animations with nested motions and temporal dependencies.

#### Key Components:

- **MotionNode**: Individual motion primitive with timing, easing, and dependencies
- **MotionGraph**: Graph structure for composing multiple related motions
- **Easing Functions**: 13 built-in easing functions for smooth transitions
- **Motion Types**: Rotation, translation, scale, oscillation, orbital, parametric

#### Example: Nested Planetary Motion

```python
from mathlib.animation import MotionGraph, MotionNode, MotionType, EasingFunction
from mathlib.core.vector import Vector

# Create motion graph
graph = MotionGraph()

# Sun rotation (root motion)
sun_rotation = MotionNode(
    name="sun_spin",
    motion_type=MotionType.ROTATION,
    parameters={
        'axis': 'z',
        'angle': 2 * 3.14159,  # Full rotation
        'unit': RADIAN
    },
    duration=10.0,
    loop=True
)
graph.add_node(sun_rotation)

# Earth orbit around sun
earth_orbit = MotionNode(
    name="earth_orbit",
    motion_type=MotionType.ORBITAL,
    parameters={
        'radius': 5.0,
        'angular_speed': 2 * 3.14159,
        'axis': 'z',
        'center': Vector([0, 0, 0])
    },
    duration=20.0,
    easing=EasingFunction.LINEAR,
    loop=True
)
graph.add_nested_motion("sun_spin", earth_orbit)

# Moon orbit around earth (nested twice!)
moon_orbit = MotionNode(
    name="moon_orbit",
    motion_type=MotionType.ORBITAL,
    parameters={
        'radius': 1.0,
        'angular_speed': 4 * 3.14159,
        'axis': 'z',
        'center': Vector([0, 0, 0])  # Relative to earth
    },
    duration=5.0,
    loop=True
)
graph.add_nested_motion("earth_orbit", moon_orbit)

# Validate for circular dependencies
graph.validate()

# Generate animation frames
frames = graph.animate("moon_orbit", 0.0, 20.0, time_step=0.016)
```

#### Easing Functions

All standard easing functions supported:
- Linear
- Quadratic (ease-in, ease-out, ease-in-out)
- Cubic (ease-in, ease-out, ease-in-out)
- Sine (ease-in, ease-out, ease-in-out)
- Elastic
- Bounce

### 2. Physics Simulation (`mathlib.physics.simulation`)

Realistic physics-based motion with forces, collisions, and numerical integration.

#### Key Components:

- **PhysicsState**: Complete physical state (position, velocity, acceleration, mass)
- **Forces**: Constant, spring, damping, drag, central, periodic
- **PhysicsSimulator**: Numerical integrator (Euler, Verlet, RK4)
- **Collision**: Detection and response for spheres and planes

#### Example: Spring-Mass-Damper System

```python
from mathlib.physics import (
    PhysicsState, PhysicsSimulator, SpringForce, DampingForce
)
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.core.units import METER, KILOGRAM

# Create mass
state = PhysicsState(
    position=Vector([2.0, 0.0, 0.0], unit=METER),
    velocity=Vector.zero(3, METER),
    acceleration=Vector.zero(3, METER),
    mass=Scalar(1.0, KILOGRAM)
)

# Create simulator with RK4 integration
sim = PhysicsSimulator(integration_method='rk4')
obj_idx = sim.add_object(state)

# Add spring force (anchor at origin)
spring = SpringForce(
    anchor=Vector.zero(3),
    spring_constant=10.0,
    rest_length=0.0
)
sim.add_force(spring, obj_idx)

# Add damping
damping = DampingForce(damping_coefficient=0.5)
sim.add_force(damping, obj_idx)

# Simulate
positions = []
for i in range(1000):
    positions.append(state.position.components[0])
    sim.step(dt=0.01)

# Plot to see damped oscillation
```

#### Available Forces:

1. **ConstantForce**: Gravity, electric fields
2. **SpringForce**: Hooke's law springs
3. **DampingForce**: Velocity-dependent damping
4. **DragForce**: Quadratic air resistance
5. **CentralForce**: Gravitational/electric (1/r² or custom)
6. **PeriodicForce**: Driven oscillations

### 3. Chemistry Module (`mathlib.chemistry.molecules`)

Create and animate molecular structures with proper geometry and bonding.

#### Key Components:

- **Atom**: 3D atom with element, position, charge, hybridization
- **Bond**: Chemical bonds (single, double, triple, aromatic, hydrogen)
- **Molecule**: Collection of atoms and bonds
- **MolecularGeometry**: Factory for standard geometries
- **Reaction**: Reaction animation with interpolation

#### Example: Water Molecule (H₂O)

```python
from mathlib.chemistry import Atom, Molecule, BondType, HybridizationType
from mathlib.geometry.point import Point
import math

# Create water molecule
water = Molecule("H2O")

# Oxygen (sp3 hybridized, bent geometry)
oxygen = Atom(
    element='O',
    position=Point([0, 0, 0]),
    atomic_number=8,
    mass=15.999,
    hybridization=HybridizationType.SP3
)
o_idx = water.add_atom(oxygen)

# Hydrogens (104.5° angle)
angle = 104.5 * math.pi / 180 / 2  # Half angle
bond_length = 0.96  # Angstroms

h1 = Atom(
    element='H',
    position=Point([
        bond_length * math.cos(angle),
        bond_length * math.sin(angle),
        0
    ]),
    atomic_number=1,
    mass=1.008
)
h1_idx = water.add_atom(h1)

h2 = Atom(
    element='H',
    position=Point([
        bond_length * math.cos(angle),
        -bond_length * math.sin(angle),
        0
    ]),
    atomic_number=1,
    mass=1.008
)
h2_idx = water.add_atom(h2)

# Add O-H bonds
water.add_bond(o_idx, h1_idx, BondType.SINGLE)
water.add_bond(o_idx, h2_idx, BondType.SINGLE)

# Calculate bond angle
angle_rad = water.get_bond_angle(h1_idx, o_idx, h2_idx)
print(f"H-O-H angle: {math.degrees(angle_rad):.1f}°")
```

#### Pre-built Geometries:

```python
from mathlib.chemistry import MolecularGeometry
from mathlib.geometry.point import Point

# Create benzene ring
benzene = MolecularGeometry.create_benzene_ring(Point([0, 0, 0]))

# Or use standard geometries
positions = MolecularGeometry.create_tetrahedral(Point([0, 0, 0]), 1.0)
# Returns: [center, vertex1, vertex2, vertex3, vertex4]
```

Available geometries:
- Linear (180°)
- Trigonal planar (120°)
- Tetrahedral (109.5°)
- Octahedral (90°)
- Benzene ring (aromatic)

### 4. Mathematical Visualization (`mathlib.visualization.surfaces`)

Tools for visualizing mathematical concepts in 3D.

#### Key Components:

- **ParametricSurface**: Surfaces defined by S(u,v)
- **VectorField**: 3D vector fields F: R³ → R³
- **ImplicitSurface**: Surfaces defined by f(x,y,z) = 0
- **FunctionGraph**: Graphs of z = f(x,y)

#### Example: Klein Bottle

```python
from mathlib.visualization import CommonSurfaces

# Create Klein bottle
klein = CommonSurfaces.klein_bottle(scale=1.0)

# Sample on grid
grid = klein.sample_grid(u_steps=50, v_steps=50)

# Each point in grid is a Point object
# grid[i][j] represents point at (u_i, v_j)

# Calculate normals for lighting
normals = []
for i in range(50):
    row_normals = []
    for j in range(50):
        u = i / 49.0 * 2 * math.pi
        v = j / 49.0 * 2 * math.pi
        normal = klein.normal_at(u, v)
        row_normals.append(normal)
    normals.append(row_normals)
```

#### Available Surfaces:

```python
from mathlib.visualization import CommonSurfaces

# Sphere
sphere = CommonSurfaces.sphere(radius=1.0)

# Torus (donut)
torus = CommonSurfaces.torus(major_radius=2.0, minor_radius=0.5)

# Möbius strip
mobius = CommonSurfaces.mobius_strip(width=1.0, radius=2.0)

# Klein bottle
klein = CommonSurfaces.klein_bottle(scale=1.0)

# Paraboloid
paraboloid = CommonSurfaces.paraboloid(a=1.0, b=1.0)

# Helicoid
helicoid = CommonSurfaces.helicoid(pitch=1.0, radius=1.0)
```

#### Vector Fields Example:

```python
from mathlib.visualization import CommonVectorFields, VectorField
from mathlib.geometry.point import Point

# Create rotational field around z-axis
rotation_field = CommonVectorFields.rotation(axis='z', strength=1.0)

# Sample on grid
samples = rotation_field.sample_grid(
    bounds=(-3, 3, -3, 3, -1, 1),  # x, y, z ranges
    steps=(10, 10, 3)               # samples per dimension
)

# Each sample is (point, vector) tuple
for point, vector in samples:
    # point: where field is evaluated
    # vector: field value at that point
    pass

# Compute streamline
start = Point([1, 0, 0])
streamline = rotation_field.compute_streamline(start, steps=100, step_size=0.1)
# Returns list of points following the field
```

#### Create Custom Vector Field:

```python
def my_field_function(p: Point) -> Vector:
    """Custom vector field: F(x,y,z) = (y, -x, z)."""
    return Vector([p.y, -p.x, p.z])

custom_field = VectorField(my_field_function)
```

## Complete Example: Pendulum with Chemistry

Combining multiple modules for a complex animation:

```python
from mathlib.animation import MotionGraph, MotionNode, MotionType
from mathlib.physics import create_pendulum_simulation
from mathlib.chemistry import Molecule, Atom, BondType
from mathlib.geometry.point import Point
from mathlib.core.vector import Vector
import math

# 1. Create molecular pendulum bob (diatomic molecule)
molecule = Molecule("O2")
o1 = Atom('O', Point([0, 0, 0]), 8, 15.999)
o2 = Atom('O', Point([1.21, 0, 0]), 8, 15.999)
molecule.add_atom(o1)
molecule.add_atom(o2)
molecule.add_bond(0, 1, BondType.DOUBLE)

# 2. Create pendulum physics
pendulum_sim = create_pendulum_simulation(
    length=2.0,
    mass=32.0,  # Total mass of O2
    initial_angle=math.pi / 4,  # 45 degrees
    gravity=9.81
)

# 3. Create motion graph for visualization
graph = MotionGraph()

# Pendulum swing motion
swing = MotionNode(
    name="pendulum_swing",
    motion_type=MotionType.PARAMETRIC,
    parameters={
        'function': lambda t, params: get_pendulum_transform(pendulum_sim, t)
    },
    duration=10.0
)
graph.add_node(swing)

# Molecule rotation (nested)
rotation = MotionNode(
    name="molecule_spin",
    motion_type=MotionType.ROTATION,
    parameters={
        'axis': Vector([1, 0, 0]),  # Rotate around bond axis
        'angle': 2 * math.pi,
        'unit': RADIAN
    },
    duration=2.0,
    loop=True
)
graph.add_nested_motion("pendulum_swing", rotation)

# 4. Animate
frames = graph.animate("molecule_spin", 0.0, 10.0, time_step=1/60)

# Each frame contains the complete hierarchical transformation
# Apply to molecule atoms for rendering
```

## Architecture

### Module Organization

```
mathlib/
├── animation/
│   ├── __init__.py
│   └── motion_graph.py          # Hierarchical motion system
├── physics/
│   ├── __init__.py
│   └── simulation.py            # Physics engine
├── chemistry/
│   ├── __init__.py
│   └── molecules.py             # Molecular structures
├── visualization/
│   ├── __init__.py
│   └── surfaces.py              # Mathematical visualization
├── core/                         # Original modules
├── geometry/                     # Original modules
├── transforms/                   # Original modules
├── kinematics/                   # Original modules
├── calculus/                     # Original modules
├── algebra/                      # Original modules
└── validation/                   # Original modules
```

### Design Principles (Maintained)

1. **Immutability**: All objects are frozen dataclasses
2. **Explicit Units**: No silent unit conversions
3. **Type Safety**: Strong typing with validation
4. **Educational Focus**: Clear, readable code over performance
5. **Fail Loudly**: Errors with context, not silent failures

### Integration with DSL

The extended mathlib integrates seamlessly with the existing DSL compiler:

```dsl
motion spinning_molecule {
  target: benzene
  type: nested_rotation
  primary_axis: [0, 0, 1]
  primary_speed: 1.0
  secondary_axis: [1, 0, 0]
  secondary_speed: 2.0
  easing: ease_in_out_cubic
}
```

## Performance Considerations

### Physics Simulation

- **Euler**: Fastest, least accurate, good for prototyping
- **Verlet**: Balanced, good for oscillations
- **RK4**: Most accurate, 4x slower than Euler

### Large Molecules

- Use simplified representations for >1000 atoms
- Implement LOD (Level of Detail) for distant molecules
- Cache bond calculations

### Vector Fields

- Pre-compute on grid for static fields
- Use adaptive sampling for complex fields
- Streamline integration can be expensive

## Future Enhancements

Planned for v2.1:

- [ ] Quantum mechanics visualization (wavefunction collapse)
- [ ] Thermodynamics (particle systems, entropy visualization)
- [ ] Electromagnetic fields (Maxwell's equations)
- [ ] Fluid dynamics (Navier-Stokes, simplified)
- [ ] Sound waves and acoustics
- [ ] Relativity visualization (spacetime curvature)

## Testing

Run tests for new modules:

```python
# Test motion graph
from mathlib.animation import MotionGraph, MotionNode, MotionType

graph = MotionGraph()
# ... build graph ...
graph.validate()  # Check for cycles

# Test physics
from mathlib.physics import create_pendulum_simulation

sim = create_pendulum_simulation(1.0, 1.0, 0.1)
for _ in range(100):
    sim.step(0.01)
# Verify energy conservation (within numerical error)

# Test chemistry
from mathlib.chemistry import MolecularGeometry

benzene = MolecularGeometry.create_benzene_ring(Point([0, 0, 0]))
assert benzene.num_vertices == 12  # 6 carbons + 6 hydrogens
```

## License

Same as original MathLib (MIT/Apache-2.0)

## Contributors

Extended modules by Claude (v2.0 animation, physics, chemistry, visualization)

Original MathLib foundation by project team

---

**Version**: 2.0.0  
**Last Updated**: 2026-01-28  
**Status**: Production Ready
