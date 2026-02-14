# Mathematical DSL Reference

## Scope
This document describes the mathematical extensions added to the DSL pipeline.

## Core constructs
- `function`: symbolic or numeric function definitions
- `curve`: explicit/implicit/parametric curves
- `surface`: explicit/implicit/parametric surfaces
- `vector_field`, `scalar_field`: field objects over a domain

## Expression forms
- Arithmetic: `+`, `-`, `*`, `/`, `^`
- Function call: `f(x)`, `sin(x)`, `log(x)`
- Derivative: `derivative(expr, var[, order])`
- Integral: `integral(expr, var[, lower, upper])`

## Domain and range
Domain restrictions should be provided when possible to support semantic checks.
Examples:
- `x in [0, 2*pi]`
- `x > 0` for `log(x)`

## Notes
Current parser and lowering support the MVP path for expressions and conversion to IR math expression payloads.
