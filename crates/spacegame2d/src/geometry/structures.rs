use glam::Vec2;
use spacegame2d_simulation::{StaticStructure, StaticStructureKind};

use crate::geometry::Vertex;

const CIRCLE_SEGMENTS: usize = 48;

pub fn structure_vertices(structures: &[StaticStructure]) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(structures.len() * (CIRCLE_SEGMENTS * 3 + 12));
    for structure in structures {
        let (fill, indicator) = match structure.kind() {
            StaticStructureKind::CommandCore => ([0.12, 0.28, 0.72, 1.0], [0.82, 0.92, 1.0, 1.0]),
            StaticStructureKind::ShieldRelay => ([0.54, 0.20, 0.76, 1.0], [1.0, 0.78, 0.18, 1.0]),
        };
        push_circle(
            &mut vertices,
            structure.position(),
            structure.visual_radius_meters(),
            fill,
        );
        match structure.kind() {
            StaticStructureKind::CommandCore => push_cross(
                &mut vertices,
                structure.position(),
                structure.visual_radius_meters() * 0.42,
                indicator,
            ),
            StaticStructureKind::ShieldRelay => push_diamond(
                &mut vertices,
                structure.position(),
                structure.visual_radius_meters() * 0.46,
                indicator,
            ),
        }
    }
    vertices
}

fn push_circle(vertices: &mut Vec<Vertex>, center: Vec2, radius: f32, color: [f32; 4]) {
    for index in 0..CIRCLE_SEGMENTS {
        let angle0 = std::f32::consts::TAU * index as f32 / CIRCLE_SEGMENTS as f32;
        let angle1 = std::f32::consts::TAU * (index + 1) as f32 / CIRCLE_SEGMENTS as f32;
        push_triangle(
            vertices,
            center,
            center + Vec2::new(angle0.cos(), angle0.sin()) * radius,
            center + Vec2::new(angle1.cos(), angle1.sin()) * radius,
            color,
        );
    }
}

fn push_cross(vertices: &mut Vec<Vertex>, center: Vec2, radius: f32, color: [f32; 4]) {
    let half_width = radius * 0.28;
    push_quad(
        vertices,
        center + Vec2::new(-half_width, -radius),
        center + Vec2::new(half_width, -radius),
        center + Vec2::new(half_width, radius),
        center + Vec2::new(-half_width, radius),
        color,
    );
    push_quad(
        vertices,
        center + Vec2::new(-radius, -half_width),
        center + Vec2::new(radius, -half_width),
        center + Vec2::new(radius, half_width),
        center + Vec2::new(-radius, half_width),
        color,
    );
}

fn push_diamond(vertices: &mut Vec<Vertex>, center: Vec2, radius: f32, color: [f32; 4]) {
    let top = center + Vec2::Y * radius;
    let right = center + Vec2::X * radius;
    let bottom = center - Vec2::Y * radius;
    let left = center - Vec2::X * radius;
    push_triangle(vertices, center, top, right, color);
    push_triangle(vertices, center, right, bottom, color);
    push_triangle(vertices, center, bottom, left, color);
    push_triangle(vertices, center, left, top, color);
}

fn push_quad(
    vertices: &mut Vec<Vertex>,
    lower_left: Vec2,
    lower_right: Vec2,
    upper_right: Vec2,
    upper_left: Vec2,
    color: [f32; 4],
) {
    push_triangle(vertices, lower_left, lower_right, upper_right, color);
    push_triangle(vertices, lower_left, upper_right, upper_left, color);
}

fn push_triangle(
    vertices: &mut Vec<Vertex>,
    first: Vec2,
    second: Vec2,
    third: Vec2,
    color: [f32; 4],
) {
    for position in [first, second, third] {
        vertices.push(Vertex {
            position: position.to_array(),
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_simulation::World;

    #[test]
    fn structures_generate_finite_world_geometry_with_distinct_indicators() {
        let world = World::demo();
        let vertices = structure_vertices(world.structures());
        assert_eq!(vertices.len(), CIRCLE_SEGMENTS * 3 * 2 + 24);
        assert!(vertices.iter().all(|vertex| {
            vertex
                .position
                .iter()
                .all(|component| component.is_finite())
        }));
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.color == [0.82, 0.92, 1.0, 1.0])
        );
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.color == [1.0, 0.78, 0.18, 1.0])
        );
    }

    #[test]
    fn circles_use_simulation_visual_radii_at_simulation_positions() {
        let world = World::demo();
        let vertices = structure_vertices(world.structures());
        let command_core_boundary = Vec2::from_array(vertices[1].position);
        assert!((command_core_boundary.length() - 3.5).abs() < 0.0001);

        let relay_start = CIRCLE_SEGMENTS * 3 + 12;
        let relay_center = Vec2::from_array(vertices[relay_start].position);
        let relay_boundary = Vec2::from_array(vertices[relay_start + 1].position);
        assert_eq!(relay_center, Vec2::new(0.0, 10.0));
        assert!((relay_center.distance(relay_boundary) - 2.5).abs() < 0.0001);
    }
}
