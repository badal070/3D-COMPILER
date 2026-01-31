"""
Atomic structure and properties.

Representation of atoms with electronegativity, hybridization,
charge, and van der Waals/covalent radii.
"""

from dataclasses import dataclass
from mathlib.core.scalar import Scalar
from mathlib.geometry.point import Point


@dataclass(frozen=True)
class Atom:
    """Atomic representation with properties."""
    
    element: str
    atomic_number: int
    position: Point
    charge: float
    electronegativity: float
    hybridization: str
    
    def __init__(
        self,
        element: str,
        atomic_number: int,
        position: Point,
        charge: float = 0.0,
        electronegativity: float = None,
        hybridization: str = "sp3"
    ):
        if atomic_number < 1 or atomic_number > 118:
            raise ValueError("Atomic number must be between 1 and 118")
        
        if electronegativity is not None and not (0 <= electronegativity <= 4.5):
            raise ValueError("Electronegativity must be between 0 and 4.5")
        
        valid_hybridizations = ['s', 'sp', 'sp2', 'sp3', 'sp3d', 'sp3d2']
        if hybridization not in valid_hybridizations:
            raise ValueError(f"Hybridization must be one of {valid_hybridizations}")
        
        # Default electronegativity based on periodic trends
        if electronegativity is None:
            electronegativity = Atom._default_electronegativity(atomic_number)
        
        object.__setattr__(self, 'element', element)
        object.__setattr__(self, 'atomic_number', atomic_number)
        object.__setattr__(self, 'position', position)
        object.__setattr__(self, 'charge', charge)
        object.__setattr__(self, 'electronegativity', electronegativity)
        object.__setattr__(self, 'hybridization', hybridization)
    
    @staticmethod
    def _default_electronegativity(atomic_number: int) -> float:
        """Approximate electronegativity (Pauling scale)."""
        # Simplified periodic trends
        electronegativities = {
            1: 2.20,  # H
            6: 2.55,  # C
            7: 3.04,  # N
            8: 3.44,  # O
            9: 3.98,  # F
            15: 2.19, # P
            16: 2.58, # S
            17: 3.16, # Cl
        }
        return electronegativities.get(atomic_number, 2.0)
    
    def van_der_waals_radius(self) -> Scalar:
        """Get van der Waals radius for atom."""
        # Van der Waals radii in angstroms
        radii = {
            1: 1.20,   # H
            6: 1.70,   # C
            7: 1.55,   # N
            8: 1.52,   # O
            9: 1.47,   # F
            15: 1.80,  # P
            16: 1.80,  # S
            17: 1.75,  # Cl
        }
        radius = radii.get(self.atomic_number, 1.70)
        return Scalar(radius * 1e-10)  # Convert to meters
    
    def covalent_radius(self) -> Scalar:
        """Get covalent radius for atom."""
        # Covalent radii in angstroms
        radii = {
            1: 0.31,   # H
            6: 0.76,   # C
            7: 0.71,   # N
            8: 0.66,   # O
            9: 0.57,   # F
            15: 1.07,  # P
            16: 1.05,  # S
            17: 1.02,  # Cl
        }
        radius = radii.get(self.atomic_number, 0.77)
        return Scalar(radius * 1e-10)  # Convert to meters
