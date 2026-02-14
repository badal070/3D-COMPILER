# Numerical Methods

## Implemented MVP
- Numerical derivative fallback (central difference)
- Numerical integral fallback (Simpson's rule)

## Runtime strategy
1. Try symbolic transformation first when available.
2. Fall back to numerical evaluation when symbolic form is unavailable.
3. Cache computed derivatives/integrals to avoid repeated cost.

## Configuration
See:
- `config/numerical_methods.toml`

## Accuracy notes
- Derivative fallback uses finite difference step size.
- Integral fallback uses fixed partition Simpson integration.
- These defaults are practical but not globally optimal.
