matrix A = [
  [2, 1],
  [1, 3]
]

transformation T = linear_transform(A)

visualize {
  original_grid: unit_square
  transformed_grid: apply(T, unit_square)
  show_eigenvectors: true
  show_eigenvalues: true
  animate_transformation: {
    from: identity
    to: T
    duration: 2s
  }
}
