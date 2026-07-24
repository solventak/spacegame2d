use glam::Vec2;

use crate::{geometry::Vertex, simulation::WORLD_RADIUS_M};

pub fn ring_vertices() -> Vec<Vertex> {
    let color = [0.0, 0.0, 0.15, 1.0];
    let segments = 128;
    let inner_radius = WORLD_RADIUS_M - 0.5;
    let outer_radius = WORLD_RADIUS_M + 0.5;
    let mut vertices = Vec::with_capacity(segments * 6);
    for i in 0..segments {
        let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let c0 = Vec2::new(a0.cos(), a0.sin());
        let c1 = Vec2::new(a1.cos(), a1.sin());
        let inner0 = c0 * inner_radius;
        let outer0 = c0 * outer_radius;
        let outer1 = c1 * outer_radius;
        let inner1 = c1 * inner_radius;
        vertices.push(Vertex {
            position: [inner0.x, inner0.y],
            color,
        });
        vertices.push(Vertex {
            position: [outer0.x, outer0.y],
            color,
        });
        vertices.push(Vertex {
            position: [outer1.x, outer1.y],
            color,
        });
        vertices.push(Vertex {
            position: [inner0.x, inner0.y],
            color,
        });
        vertices.push(Vertex {
            position: [outer1.x, outer1.y],
            color,
        });
        vertices.push(Vertex {
            position: [inner1.x, inner1.y],
            color,
        });
    }
    vertices
}
