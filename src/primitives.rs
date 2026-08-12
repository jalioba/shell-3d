use glam::Vec3;
use crate::mesh::{Mesh, Triangle};

/// Generates a procedural 3D Cube (-1.0 to 1.0)
pub fn create_cube() -> Mesh {
    let vertices = [
        Vec3::new(-1.0, -1.0, -1.0), // 0
        Vec3::new( 1.0, -1.0, -1.0), // 1
        Vec3::new( 1.0,  1.0, -1.0), // 2
        Vec3::new(-1.0,  1.0, -1.0), // 3
        Vec3::new(-1.0, -1.0,  1.0), // 4
        Vec3::new( 1.0, -1.0,  1.0), // 5
        Vec3::new( 1.0,  1.0,  1.0), // 6
        Vec3::new(-1.0,  1.0,  1.0), // 7
    ];

    let indices = [
        // Front
        [4, 5, 6], [4, 6, 7],
        // Back
        [1, 0, 3], [1, 3, 2],
        // Top
        [7, 6, 2], [7, 2, 3],
        // Bottom
        [4, 0, 1], [4, 1, 5],
        // Right
        [5, 1, 2], [5, 2, 6],
        // Left
        [0, 4, 7], [0, 7, 3],
    ];

    let mut triangles = Vec::new();
    for idx in indices {
        triangles.push(Triangle::new(vertices[idx[0]], vertices[idx[1]], vertices[idx[2]]));
    }

    Mesh::new_with_color("Cube", triangles, (100, 200, 255)) // Cyber Blue
}

/// Generates a procedural 3D Pyramid
pub fn create_pyramid() -> Mesh {
    let top = Vec3::new(0.0, 1.0, 0.0);
    let v0 = Vec3::new(-1.0, -1.0, -1.0);
    let v1 = Vec3::new( 1.0, -1.0, -1.0);
    let v2 = Vec3::new( 1.0, -1.0,  1.0);
    let v3 = Vec3::new(-1.0, -1.0,  1.0);

    let triangles = vec![
        // Sides
        Triangle::new(v0, v1, top),
        Triangle::new(v1, v2, top),
        Triangle::new(v2, v3, top),
        Triangle::new(v3, v0, top),
        // Base
        Triangle::new(v0, v3, v2),
        Triangle::new(v0, v2, v1),
    ];

    Mesh::new_with_color("Pyramid", triangles, (255, 180, 40)) // Gold / Amber
}

/// Generates a procedural UV Sphere
pub fn create_sphere(stacks: usize, slices: usize) -> Mesh {
    let mut vertices = Vec::new();
    let radius = 1.0f32;

    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI * (i as f32) / (stacks as f32); // 0 to PI
        let xy = radius * stack_angle.sin();
        let z = radius * stack_angle.cos();

        for j in 0..=slices {
            let slice_angle = 2.0 * std::f32::consts::PI * (j as f32) / (slices as f32); // 0 to 2PI
            let x = xy * slice_angle.cos();
            let y = xy * slice_angle.sin();
            vertices.push(Vec3::new(x, y, z));
        }
    }

    let mut triangles = Vec::new();
    for i in 0..stacks {
        let k1 = i * (slices + 1);
        let k2 = k1 + slices + 1;

        for j in 0..slices {
            let v1 = vertices[k1 + j];
            let v2 = vertices[k2 + j];
            let v3 = vertices[k1 + j + 1];
            let v4 = vertices[k2 + j + 1];

            if i != 0 {
                triangles.push(Triangle::new(v1, v2, v3));
            }
            if i != (stacks - 1) {
                triangles.push(Triangle::new(v3, v2, v4));
            }
        }
    }

    Mesh::new_with_color("Sphere", triangles, (50, 220, 120)) // Emerald Green
}

/// Generates a procedural Torus (Donut)
pub fn create_torus(main_radius: f32, tube_radius: f32, num_main: usize, num_tube: usize) -> Mesh {
    let mut vertices = Vec::new();

    for i in 0..num_main {
        let u = (i as f32) * 2.0 * std::f32::consts::PI / (num_main as f32);
        let cos_u = u.cos();
        let sin_u = u.sin();

        for j in 0..num_tube {
            let v = (j as f32) * 2.0 * std::f32::consts::PI / (num_tube as f32);
            let cos_v = v.cos();
            let sin_v = v.sin();

            let x = (main_radius + tube_radius * cos_v) * cos_u;
            let y = (main_radius + tube_radius * cos_v) * sin_u;
            let z = tube_radius * sin_v;

            vertices.push(Vec3::new(x, y, z));
        }
    }

    let mut triangles = Vec::new();
    for i in 0..num_main {
        let i_next = (i + 1) % num_main;
        for j in 0..num_tube {
            let j_next = (j + 1) % num_tube;

            let idx0 = i * num_tube + j;
            let idx1 = i_next * num_tube + j;
            let idx2 = i_next * num_tube + j_next;
            let idx3 = i * num_tube + j_next;

            triangles.push(Triangle::new(vertices[idx0], vertices[idx1], vertices[idx2]));
            triangles.push(Triangle::new(vertices[idx0], vertices[idx2], vertices[idx3]));
        }
    }

    Mesh::new_with_color("Torus (Donut)", triangles, (255, 90, 150)) // Neon Pink / Coral
}
