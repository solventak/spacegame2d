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
