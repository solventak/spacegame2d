use crate::geometry::Vertex;

pub fn notched_ship_vertices() -> Vec<Vertex> {
    let cyan = [0.0, 0.9, 1.0, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let points = [
        [0.0, 0.60],
        [0.45, -0.40],
        [0.14, -0.40],
        [0.0, -0.15],
        [-0.14, -0.40],
        [-0.45, -0.40],
    ];
    let triangles = [[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 5]];
    let mut vertices = Vec::with_capacity(24);
    for (color, scale) in [(cyan, 1.0), (black, 0.78)] {
        for triangle in triangles {
            for index in triangle {
                vertices.push(Vertex {
                    position: [points[index][0] * scale, points[index][1] * scale],
                    color,
                });
            }
        }
    }
    for triangle in [[0, 1, 2], [0, 2, 1]] {
        for index in triangle {
            let p = [[0.0, 0.18], [0.18, -0.12], [-0.18, -0.12]][index];
            vertices.push(Vertex {
                position: p,
                color: [1.0, 0.0, 0.0, 1.0],
            });
        }
    }
    vertices
}

const OVERLAY_CYAN: [f32; 4] = [0.13, 0.81, 0.91, -1.0];

pub fn selection_bracket_vertices(min: [f32; 2], max: [f32; 2]) -> Vec<Vertex> {
    let size = [max[0] - min[0], max[1] - min[1]];
    let length = (size[0].min(size[1]) * 0.2).clamp(0.3, 1.2);
    let width = 0.055;
    let mut vertices = Vec::with_capacity(48);
    for (x, y, horizontal_direction, vertical_direction) in [
        (min[0], max[1], 1.0, -1.0),
        (max[0], max[1], -1.0, -1.0),
        (min[0], min[1], 1.0, 1.0),
        (max[0], min[1], -1.0, 1.0),
    ] {
        append_quad(
            &mut vertices,
            [x + horizontal_direction * length * 0.5, y],
            [length, width],
        );
        append_quad(
            &mut vertices,
            [x, y + vertical_direction * length * 0.5],
            [width, length],
        );
    }
    vertices
}

pub fn dotted_selection_vertices(min: [f32; 2], max: [f32; 2]) -> Vec<Vertex> {
    let width = 0.045;
    let dash = 0.22;
    let gap = 0.16;
    let mut vertices = Vec::new();
    let style = [dash, gap, width];
    append_dashes(&mut vertices, [min[0], max[0]], min[1], true, style);
    append_dashes(&mut vertices, [min[0], max[0]], max[1], true, style);
    append_dashes(&mut vertices, [min[1], max[1]], min[0], false, style);
    append_dashes(&mut vertices, [min[1], max[1]], max[0], false, style);
    vertices
}

fn append_dashes(
    vertices: &mut Vec<Vertex>,
    range: [f32; 2],
    fixed: f32,
    horizontal: bool,
    style: [f32; 3],
) {
    let [dash, gap, width] = style;
    let mut cursor = range[0];
    while cursor < range[1] {
        let length = (range[1] - cursor).min(dash);
        let center = cursor + length * 0.5;
        append_quad(
            vertices,
            if horizontal {
                [center, fixed]
            } else {
                [fixed, center]
            },
            if horizontal {
                [length, width]
            } else {
                [width, length]
            },
        );
        cursor += dash + gap;
    }
}

fn append_quad(vertices: &mut Vec<Vertex>, center: [f32; 2], size: [f32; 2]) {
    let half = [size[0] * 0.5, size[1] * 0.5];
    for point in [
        [-half[0], -half[1]],
        [half[0], -half[1]],
        [half[0], half[1]],
        [-half[0], -half[1]],
        [half[0], half[1]],
        [-half[0], half[1]],
    ] {
        vertices.push(Vertex {
            position: [center[0] + point[0], center[1] + point[1]],
            color: OVERLAY_CYAN,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_overlays_use_world_space_and_dash_the_drag_rectangle() {
        let brackets = selection_bracket_vertices([-2.0, -1.0], [2.0, 1.0]);
        let dashes = dotted_selection_vertices([-2.0, -1.0], [2.0, 1.0]);
        assert_eq!(brackets.len(), 48);
        assert!(!dashes.is_empty());
        assert!(
            brackets
                .iter()
                .chain(&dashes)
                .all(|vertex| vertex.color[3] < 0.0)
        );
    }
}
