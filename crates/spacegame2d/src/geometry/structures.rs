use glam::Vec2;
use spacegame2d_simulation::{PlayerId, StaticStructure, StaticStructureKind};

use crate::geometry::Vertex;

const CIRCLE_SEGMENTS: usize = 48;
const INSET_SCALE: f32 = 0.78;
const PLAYER_ONE_COLOR: [f32; 4] = [0.0, 0.9, 1.0, 1.0];
const PLAYER_TWO_COLOR: [f32; 4] = [1.0, 0.35, 0.2, 1.0];
const INSET_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub fn structure_vertices(structures: &[StaticStructure]) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(structures.len() * (CIRCLE_SEGMENTS * 6 + 12));
    for structure in structures {
        let owner_color = match structure.owner() {
            PlayerId(1) => PLAYER_ONE_COLOR,
            PlayerId(2) => PLAYER_TWO_COLOR,
            _ => PLAYER_ONE_COLOR,
        };
        push_circle(
            &mut vertices,
            structure.position(),
            structure.visual_radius_meters(),
            owner_color,
        );
        push_circle(
            &mut vertices,
            structure.position(),
            structure.visual_radius_meters() * INSET_SCALE,
            INSET_COLOR,
        );
        match structure.kind() {
            StaticStructureKind::CommandCore => push_cross(
                &mut vertices,
                structure.position(),
                structure.visual_radius_meters() * 0.42,
                owner_color,
            ),
            StaticStructureKind::ShieldRelay => push_diamond(
                &mut vertices,
                structure.position(),
                structure.visual_radius_meters() * 0.46,
                owner_color,
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
        assert_eq!(vertices.len(), (CIRCLE_SEGMENTS * 6 + 12) * 4);
        assert!(vertices.iter().all(|vertex| {
            vertex
                .position
                .iter()
                .all(|component| component.is_finite())
        }));
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.color == PLAYER_ONE_COLOR)
        );
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.color == PLAYER_TWO_COLOR)
        );
        assert!(vertices.iter().any(|vertex| vertex.color == INSET_COLOR));
    }

    #[test]
    fn circles_use_simulation_visual_radii_at_simulation_positions() {
        let world = World::demo();
        let vertices = structure_vertices(world.structures());
        for (index, structure) in world.structures().iter().enumerate() {
            let start = index * (CIRCLE_SEGMENTS * 6 + 12);
            let center = Vec2::from_array(vertices[start].position);
            let boundary = Vec2::from_array(vertices[start + 1].position);
            assert_eq!(center, structure.position());
            assert!((center.distance(boundary) - structure.visual_radius_meters()).abs() < 0.0001);
        }
    }
}
