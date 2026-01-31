"""
Chemical bonds and molecular structure.

Bonds, bond angles, and molecular vibrations for
molecular dynamics and visualization.
"""

from dataclasses import dataclass
from typing import List, Tuple
from mathlib.chemistry.atoms import Atom
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
import math


@dataclass(frozen=True)
class Bond:
    """Chemical bond between atoms."""
    
    atom1: Atom
    atom2: Atom
    bond_order: int  # 1 = single, 2 = double, 3 = triple
    equilibrium_length: Scalar
    spring_constant: Scalar
    
    def __init__(
        self,
        atom1: Atom,
        atom2: Atom,
        bond_order: int = 1,
        equilibrium_length: Scalar = None,
        spring_constant: Scalar = None
    ):
        if bond_order not in [1, 2, 3]:
            raise ValueError("Bond order must be 1, 2, or 3")
        
        # Default equilibrium length: sum of covalent radii
        if equilibrium_length is None:
            r1 = atom1.covalent_radius().value
            r2 = atom2.covalent_radius().value
            # Bond order affects length: double bonds ~shorter by 10-15%
            length_factor = 1.0 - (bond_order - 1) * 0.12
            equilibrium_length = Scalar((r1 + r2) * length_factor)
        
        # Default spring constant based on bond strength
        if spring_constant is None:
            # Stronger bonds (higher order) have higher spring constants
            base_k = 500.0  # N/m
            spring_constant = Scalar(base_k * bond_order)
        
        object.__setattr__(self, 'atom1', atom1)
        object.__setattr__(self, 'atom2', atom2)
        object.__setattr__(self, 'bond_order', bond_order)
        object.__setattr__(self, 'equilibrium_length', equilibrium_length)
        object.__setattr__(self, 'spring_constant', spring_constant)
    
    def current_length(self) -> Scalar:
        """Compute current bond length."""
        return self.atom1.position.distance_to(self.atom2.position)
    
    def force(self) -> Tuple[Vector, Vector]:
        """
        Compute harmonic force on both atoms.
        
        F = -k * (r - r0) * r_hat
        
        Returns: (force_on_atom1, force_on_atom2)
        """
        displacement = self.atom2.position.position - self.atom1.position.position
        current_length = displacement.norm().value
        
        if current_length < 1e-12:
            return (Vector.zero(3), Vector.zero(3))
        
        direction = displacement.normalize()
        extension = current_length - self.equilibrium_length.value
        force_magnitude = -self.spring_constant.value * extension
        
        force = direction * force_magnitude
        return (force, -force)  # Newton's third law
    
    def potential_energy(self) -> Scalar:
        """Compute harmonic potential energy: U = (1/2) * k * (r - r0)^2."""
        current = self.current_length().value
        equilibrium = self.equilibrium_length.value
        extension = current - equilibrium
        
        pe = 0.5 * self.spring_constant.value * extension * extension
        return Scalar(pe)
    
    def bond_dipole(self) -> Vector:
        """
        Compute bond dipole moment.
        
        μ = q * r, where q is partial charge difference
        """
        # Electronegativity difference determines charge separation
        delta_en = abs(self.atom2.electronegativity - self.atom1.electronegativity)
        
        # Empirical relation: partial charge ≈ 0.16 * ΔEN + 0.035 * ΔEN^2
        partial_charge = 0.16 * delta_en + 0.035 * delta_en * delta_en
        
        # Direction: from less to more electronegative
        displacement = self.atom2.position.position - self.atom1.position.position
        if self.atom2.electronegativity < self.atom1.electronegativity:
            displacement = -displacement
        
        # Dipole moment magnitude
        dipole_magnitude = partial_charge * displacement.norm().value
        direction = displacement.normalize()
        
        return direction * dipole_magnitude


@dataclass(frozen=True)
class BondAngle:
    """Angle constraint between three bonded atoms."""
    
    atom1: Atom
    atom2: Atom  # Central atom
    atom3: Atom
    equilibrium_angle: Scalar  # In radians
    spring_constant: Scalar
    
    def __init__(
        self,
        atom1: Atom,
        atom2: Atom,
        atom3: Atom,
        equilibrium_angle: Scalar,
        spring_constant: Scalar = None
    ):
        if not (0 < equilibrium_angle.value < math.pi):
            raise ValueError("Equilibrium angle must be between 0 and π radians")
        
        if spring_constant is None:
            # Default angle spring constant
            spring_constant = Scalar(50.0)  # (energy unit) / rad^2
        
        object.__setattr__(self, 'atom1', atom1)
        object.__setattr__(self, 'atom2', atom2)
        object.__setattr__(self, 'atom3', atom3)
        object.__setattr__(self, 'equilibrium_angle', equilibrium_angle)
        object.__setattr__(self, 'spring_constant', spring_constant)
    
    def current_angle(self) -> Scalar:
        """Compute current angle between three atoms."""
        # Vectors from central atom to outer atoms
        v1 = self.atom1.position.position - self.atom2.position.position
        v2 = self.atom3.position.position - self.atom2.position.position
        
        # Angle from dot product
        cos_angle = v1.dot(v2).value / (v1.norm().value * v2.norm().value)
        cos_angle = max(-1.0, min(1.0, cos_angle))  # Clamp to [-1, 1]
        
        angle = math.acos(cos_angle)
        return Scalar(angle)
    
    def potential_energy(self) -> Scalar:
        """Compute angle bending potential: U = (1/2) * k * (θ - θ0)^2."""
        current = self.current_angle().value
        equilibrium = self.equilibrium_angle.value
        deviation = current - equilibrium
        
        pe = 0.5 * self.spring_constant.value * deviation * deviation
        return Scalar(pe)


@dataclass(frozen=True)
class MolecularVibration:
    """Vibrational mode of a molecule."""
    
    mode_type: str  # 'stretch', 'bend', 'torsion', etc.
    frequency: Scalar  # In Hz
    amplitude: Scalar
    atoms: List[Atom]
    displacement_vectors: List[Vector]
    
    def __init__(
        self,
        mode_type: str,
        frequency: Scalar,
        amplitude: Scalar,
        atoms: List[Atom],
        displacement_vectors: List[Vector]
    ):
        if frequency.value <= 0:
            raise ValueError("Frequency must be positive")
        if amplitude.value < 0:
            raise ValueError("Amplitude must be non-negative")
        
        if len(atoms) != len(displacement_vectors):
            raise ValueError("Must have one displacement vector per atom")
        
        valid_modes = ['symmetric_stretch', 'asymmetric_stretch', 'bending',
                      'scissoring', 'rocking', 'wagging', 'twisting', 'torsion']
        if mode_type not in valid_modes:
            raise ValueError(f"Mode type must be one of {valid_modes}")
        
        object.__setattr__(self, 'mode_type', mode_type)
        object.__setattr__(self, 'frequency', frequency)
        object.__setattr__(self, 'amplitude', amplitude)
        object.__setattr__(self, 'atoms', tuple(atoms))
        object.__setattr__(self, 'displacement_vectors', tuple(displacement_vectors))
    
    def displacement_at_time(self, atom_index: int, time: float) -> Vector:
        """
        Compute displacement for atom at given time.
        
        Δr = A * sin(2πft) * d_hat
        """
        if atom_index < 0 or atom_index >= len(self.atoms):
            raise IndexError("Atom index out of range")
        
        omega = 2 * math.pi * self.frequency.value
        phase = omega * time
        magnitude = self.amplitude.value * math.sin(phase)
        
        displacement_direction = self.displacement_vectors[atom_index]
        return displacement_direction * magnitude
    
    def energy(self) -> Scalar:
        """
        Compute vibrational energy: E = hf
        
        where h is Planck's constant
        """
        h = 6.62607015e-34  # J⋅s
        energy = h * self.frequency.value
        return Scalar(energy)
