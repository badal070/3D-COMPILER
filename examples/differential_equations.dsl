// Simple harmonic oscillator
ode {
  equation: d²x/dt² + ω²*x = 0
  initial_conditions: [x(0) = 1, dx/dt(0) = 0]
  parameters: ω = 2*pi
  time_span: [0, 4]
}

visualize {
  solution_curve: true
  phase_portrait: true
  direction_field: true
}

// Lotka-Volterra (predator-prey)
ode_system {
  equations: [
    dx/dt = αx - βxy,
    dy/dt = δxy - γy
  ]
  initial_conditions: [x(0) = 10, y(0) = 5]
  parameters: [α = 1.5, β = 0.1, δ = 0.075, γ = 1.5]
  time_span: [0, 30]
}

visualize {
  phase_portrait: true
  trajectories: multiple
  equilibrium_points: true
  nullclines: true
}
