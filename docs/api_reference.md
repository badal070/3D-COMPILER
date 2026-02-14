# API Reference

## Runtime math engine
Location:
- `compiler/runtime/src/math/runtime_engine.rs`

Key API:
- `RuntimeMathEngine::evaluate_expression`
- `RuntimeMathEngine::evaluate_derivative`
- `RuntimeMathEngine::evaluate_integral`
- `RuntimeMathEngine::invalidate_caches`

## Expression model
Location:
- `compiler/runtime/src/math/expression.rs`

Core types:
- `Expression`
- `UnaryOperator`
- `BinaryOperator`

## Renderer math MVP
Location:
- `compiler/renderer/mesh_generator.rs`

Core API:
- `generate_function_mesh_2d`
- `generate_surface_mesh_3d`
- `generate_parametric_surface_mesh`
