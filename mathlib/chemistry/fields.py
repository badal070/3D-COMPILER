"""
Molecular force fields.

Coulombic electrostatics, Lennard-Jones potentials,
and electron orbital representations.
"""

from dataclasses import dataclass
from typing import Tuple
from mathlib.chemistry.atoms import Atom
from mathlib.core.vector import Vector
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point
import math


@dataclass(frozen=True)
class CoulombicField:
    """Electrostatic field for charged interactions."""
    
    permittivity: float  # Vacuum permittivity ε₀
    
    def __init__(self, permittivity: float = 8.854187817e-12):
        if permittivity <= 0:
            raise ValueError("Permittivity must be positive")
        
        object.__setattr__(self, 'permittivity', permittivity)
    
    def force(self, atom1: Atom, atom2: Atom) -> Tuple[Vector, Vector]:
        """
        Compute Coulomb force between two charged atoms.
        
        F = k * q1 * q2 / r^2 * r_hat
        where k = 1 / (4πε₀)
        """
        # Coulomb's constant
        k_e = 1.0 / (4 * math.pi * self.permittivity)
        
        q1 = atom1.charge * 1.602176634e-19  # Convert to Coulombs
        q2 = atom2.charge * 1.602176634e-19
        
        displacement = atom2.position.position - atom1.position.position
        distance = displacement.norm().value
        
        if distance < 1e-12:
            return (Vector.zero(3), Vector.zero(3))
        
        direction = displacement.normalize()
        force_magnitude = k_e * q1 * q2 / (distance * distance)
        
        force = direction * force_magnitude
        return (force, -force)
    
    def potential(self, atom1: Atom, atom2: Atom) -> Scalar:
        """
        Compute Coulomb potential energy: U = k * q1 * q2 / r
        """
        k_e = 1.0 / (4 * math.pi * self.permittivity)
        
        q1 = atom1.charge * 1.602176634e-19
        q2 = atom2.charge * 1.602176634e-19
        
        distance = atom1.position.distance_to(atom2.position).value
        
        if distance < 1e-12:
            return Scalar(float('inf'))
        
        pe = k_e * q1 * q2 / distance
        return Scalar(pe)


@dataclass(frozen=True)
class LennardJonesPotential:
    """Lennard-Jones 12-6 potential for van der Waals interactions."""
    
    epsilon: Scalar  # Depth of potential well
    sigma: Scalar    # Distance at which potential is zero
    
    def __init__(self, epsilon: Scalar, sigma: Scalar):
        if epsilon.value <= 0:
            raise ValueError("Epsilon must be positive")
        if sigma.value <= 0:
            raise ValueError("Sigma must be positive")
        
        object.__setattr__(self, 'epsilon', epsilon)
        object.__setattr__(self, 'sigma', sigma)
    
    def potential(self, atom1: Atom, atom2: Atom) -> Scalar:
        """
        Compute L-J potential: V = 4ε[(σ/r)^12 - (σ/r)^6]
        """
        distance = atom1.position.distance_to(atom2.position).value
        
        if distance < 1e-12:
            return Scalar(float('inf'))
        
        sigma_over_r = self.sigma.value / distance
        sr6 = sigma_over_r ** 6
        sr12 = sr6 * sr6
        
        potential = 4 * self.epsilon.value * (sr12 - sr6)
        return Scalar(potential)
    
    def force(self, atom1: Atom, atom2: Atom) -> Tuple[Vector, Vector]:
        """
        Compute L-J force: F = 24ε/r * [(σ/r)^6 - 2(σ/r)^12] * r_hat
        """
        displacement = atom2.position.position - atom1.position.position
        distance = displacement.norm().value
        
        if distance < 1e-12:
            return (Vector.zero(3), Vector.zero(3))
        
        direction = displacement.normalize()
        sigma_over_r = self.sigma.value / distance
        sr6 = sigma_over_r ** 6
        sr12 = sr6 * sr6
        
        force_magnitude = 24 * self.epsilon.value / distance * (sr6 - 2 * sr12)
        
        force = direction * force_magnitude
        return (force, -force)
    
    @staticmethod
    def from_atoms(atom1: Atom, atom2: Atom) -> 'LennardJonesPotential':
        """
        Create L-J potential from atom types using Lorentz-Berthelot rules.
        
        ε₁₂ = sqrt(ε₁ * ε₂)
        σ₁₂ = (σ₁ + σ₂) / 2
        """
        # Approximate epsilon values for common atoms (in J)
        epsilon_values = {
            1: 0.044e-21,   # H
            6: 0.439e-21,   # C
            7: 0.774e-21,   # N
            8: 0.878e-21,   # O
        }
        
        eps1 = epsilon_values.get(atom1.atomic_number, 0.5e-21)
        eps2 = epsilon_values.get(atom2.atomic_number, 0.5e-21)
        epsilon = Scalar((eps1 * eps2) ** 0.5)
        
        # Use van der Waals radii for sigma
        sigma1 = atom1.van_der_waals_radius().value
        sigma2 = atom2.van_der_waals_radius().value
        sigma = Scalar((sigma1 + sigma2) / 2)
        
        return LennardJonesPotential(epsilon, sigma)


@dataclass(frozen=True)
class ElectronOrbital:
    """Electron orbital representation."""
    
    orbital_type: str  # '1s', '2s', '2p', etc.
    energy_level: Scalar
    occupancy: int
    center: Point
    
    def __init__(
        self,
        orbital_type: str,
        energy_level: Scalar,
        occupancy: int,
        center: Point
    ):
        if occupancy < 0:
            raise ValueError("Occupancy must be non-negative")
        
        # Maximum occupancy based on orbital type
        max_occupancies = {
            's': 2,
            'p': 6,
            'd': 10,
            'f': 14
        }
        
        orbital_letter = orbital_type[-1]
        max_occ = max_occupancies.get(orbital_letter, 2)
        
        if occupancy > max_occ:
            raise ValueError(f"Occupancy {occupancy} exceeds maximum {max_occ} for {orbital_type}")
        
        object.__setattr__(self, 'orbital_type', orbital_type)
        object.__setattr__(self, 'energy_level', energy_level)
        object.__setattr__(self, 'occupancy', occupancy)
        object.__setattr__(self, 'center', center)
    
    def probability_density(self, point: Point) -> float:
        """
        Compute electron probability density at point.
        
        Simplified radial probability for visualization.
        """
        r = self.center.distance_to(point).value * 1e10  # Convert to angstroms
        
        # Simplified radial functions
        if self.orbital_type == '1s':
            psi = math.exp(-r)
        elif self.orbital_type == '2s':
            psi = (2 - r) * math.exp(-r/2)
        elif self.orbital_type == '2p':
            psi = r * math.exp(-r/2)
        else:
            # Generic falloff
            n = int(self.orbital_type[0])
            psi = (r ** (n-1)) * math.exp(-r/n)
        
        # Probability density is |ψ|^2
        return psi * psi
