// Single-variable calculus
function f(x) = sin(x) * exp(-x/10)
  domain: x in [0, 4*pi]

visualize {
  plot: f(x)
  tangent_line at: x = pi
  show_derivative: true
  show_integral: true
  riemann_sum: {
    method: midpoint
    partitions: 20
  }
}

// Multivariable calculus
function g(x,y) = x^2 - y^2
  domain: x in [-2, 2], y in [-2, 2]

visualize {
  surface: g(x,y)
  gradient_field: true
  critical_points: true
  level_curves: [-4, -2, 0, 2, 4]
}
