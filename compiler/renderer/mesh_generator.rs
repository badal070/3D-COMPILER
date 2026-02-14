//! Mesh generation helpers for mathematical visualization.
//! MVP scope:
//! - 2D function plots as line geometry
//! - z=f(x,y) surfaces as triangle meshes
//! - parametric surfaces as triangle meshes

use crate::renderer::error::{RenderError, RenderResult};
use crate::renderer::GeometryType;

pub fn generate_function_mesh_2d<F>(
    f: F,
    domain: (f64, f64),
    resolution: usize,
) -> RenderResult<GeometryType>
where
    F: Fn(f64) -> f64,
{
    let (x0, x1) = domain;
    if resolution < 2 {
        return Err(RenderError::InvalidGeometry(
            "Function plot resolution must be at least 2".to_string(),
        ));
    }
    if !x0.is_finite() || !x1.is_finite() || x0 >= x1 {
        return Err(RenderError::InvalidGeometry(
            "Function plot domain must satisfy finite x_min < x_max".to_string(),
        ));
    }

    let mut points = Vec::with_capacity(resolution);
    let step = (x1 - x0) / (resolution - 1) as f64;

    for i in 0..resolution {
        let x = x0 + i as f64 * step;
        let y = f(x);
        if y.is_finite() {
            points.push([x, y, 0.0]);
        }
    }

    if points.len() < 2 {
        return Err(RenderError::InvalidGeometry(
            "Function plot produced fewer than two finite points".to_string(),
        ));
    }

    Ok(GeometryType::Line { points })
}

pub fn generate_surface_mesh_3d<F>(
    f: F,
    domain: ((f64, f64), (f64, f64)),
    resolution: (usize, usize),
) -> RenderResult<GeometryType>
where
    F: Fn(f64, f64) -> f64,
{
    generate_parametric_surface_mesh(|x, y| [x, y, f(x, y)], domain, resolution)
}

pub fn generate_parametric_surface_mesh<F>(
    f: F,
    domain: ((f64, f64), (f64, f64)),
    resolution: (usize, usize),
) -> RenderResult<GeometryType>
where
    F: Fn(f64, f64) -> [f64; 3],
{
    let ((u0, u1), (v0, v1)) = domain;
    let (nu, nv) = resolution;

    if nu < 2 || nv < 2 {
        return Err(RenderError::InvalidGeometry(
            "Surface resolution must be at least 2x2".to_string(),
        ));
    }
    if !u0.is_finite()
        || !u1.is_finite()
        || !v0.is_finite()
        || !v1.is_finite()
        || u0 >= u1
        || v0 >= v1
    {
        return Err(RenderError::InvalidGeometry(
            "Surface domain must satisfy finite min < max on both axes".to_string(),
        ));
    }

    let mut vertices = Vec::with_capacity(nu * nv);
    let du = (u1 - u0) / (nu - 1) as f64;
    let dv = (v1 - v0) / (nv - 1) as f64;

    for i in 0..nu {
        let u = u0 + i as f64 * du;
        for j in 0..nv {
            let v = v0 + j as f64 * dv;
            let p = f(u, v);
            if p[0].is_finite() && p[1].is_finite() && p[2].is_finite() {
                vertices.push(p);
            } else {
                vertices.push([0.0, 0.0, 0.0]);
            }
        }
    }

    let mut indices = Vec::with_capacity((nu - 1) * (nv - 1) * 6);
    for i in 0..(nu - 1) {
        for j in 0..(nv - 1) {
            let a = (i * nv + j) as u32;
            let b = ((i + 1) * nv + j) as u32;
            let c = (i * nv + (j + 1)) as u32;
            let d = ((i + 1) * nv + (j + 1)) as u32;

            // Two triangles per quad: (a,b,c) and (b,d,c)
            indices.push(a);
            indices.push(b);
            indices.push(c);
            indices.push(b);
            indices.push(d);
            indices.push(c);
        }
    }

    Ok(GeometryType::Mesh { vertices, indices })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_mesh_generation() {
        let geom = generate_function_mesh_2d(|x| x * x, (-1.0, 1.0), 16).unwrap();
        match geom {
            GeometryType::Line { points } => assert_eq!(points.len(), 16),
            _ => panic!("expected line geometry"),
        }
    }

    #[test]
    fn test_surface_mesh_generation() {
        let geom =
            generate_surface_mesh_3d(|x, y| x * x + y * y, ((-1.0, 1.0), (-1.0, 1.0)), (8, 8))
                .unwrap();
        match geom {
            GeometryType::Mesh { vertices, indices } => {
                assert_eq!(vertices.len(), 64);
                assert_eq!(indices.len(), (8 - 1) * (8 - 1) * 6);
            }
            _ => panic!("expected mesh geometry"),
        }
    }
}
