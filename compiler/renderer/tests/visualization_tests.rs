use renderer::{
    generate_function_mesh_2d, generate_parametric_surface_mesh, generate_surface_mesh_3d,
    GeometryType,
};

#[test]
fn function_plot_generation() {
    let geom = generate_function_mesh_2d(|x| x.sin(), (0.0, std::f64::consts::PI), 64).unwrap();
    match geom {
        GeometryType::Line { points } => assert_eq!(points.len(), 64),
        _ => panic!("expected line geometry"),
    }
}

#[test]
fn surface_generation() {
    let geom = generate_surface_mesh_3d(|x, y| x * x - y * y, ((-1.0, 1.0), (-1.0, 1.0)), (12, 10))
        .unwrap();

    match geom {
        GeometryType::Mesh { vertices, indices } => {
            assert_eq!(vertices.len(), 12 * 10);
            assert_eq!(indices.len(), (12 - 1) * (10 - 1) * 6);
        }
        _ => panic!("expected mesh geometry"),
    }
}

#[test]
fn parametric_surface_generation() {
    let geom =
        generate_parametric_surface_mesh(|u, v| [u, v, u * v], ((0.0, 1.0), (0.0, 1.0)), (8, 8))
            .unwrap();

    match geom {
        GeometryType::Mesh { vertices, .. } => assert_eq!(vertices.len(), 64),
        _ => panic!("expected mesh geometry"),
    }
}
