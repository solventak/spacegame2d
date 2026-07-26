//! Combat mesh construction and the small wgpu renderer that consumes it.

use std::ops::Range;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use spacegame2d_simulation::{MUZZLE_OFFSET_METERS, PlayerId, Unit};
use wgpu::util::DeviceExt;

use crate::combat_presentation::CombatPresentation;

const PLAYER_ONE_COLOR: [f32; 4] = [0.0, 0.9, 1.0, 1.0];
const PLAYER_TWO_COLOR: [f32; 4] = [1.0, 0.35, 0.2, 1.0];
const OUTLINE_COLOR: [f32; 4] = [0.03, 0.05, 0.08, 1.0];
const TRACER_OUTER_COLOR: [f32; 4] = [1.0, 0.55, 0.08, 0.26];
const TRACER_CORE_COLOR: [f32; 4] = [1.0, 0.97, 0.82, 0.45];
const FLASH_COLOR: [f32; 4] = [1.0, 0.9, 0.5, 0.9];
const MOUNT_RADIUS: f32 = 0.16;
const MOUNT_CORE_RADIUS: f32 = 0.11;
const BARREL_OUTER_WIDTH: f32 = 0.10;
const BARREL_CORE_WIDTH: f32 = 0.06;
const TRACER_OUTER_WIDTH: f32 = 0.06;
const TRACER_CORE_WIDTH: f32 = 0.018;
const FLASH_RADIUS: f32 = 0.25;
const MOUNT_SEGMENTS: usize = 12;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, PartialEq)]
pub(crate) struct CombatVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [f32; 4],
}

impl CombatVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CombatMesh {
    pub(crate) vertices: Vec<CombatVertex>,
    pub(crate) turrets: Range<u32>,
    pub(crate) tracers: Range<u32>,
    pub(crate) flashes: Range<u32>,
}

pub(crate) fn build_mesh(
    units: &[Unit],
    presentation: &CombatPresentation,
    now: Instant,
) -> CombatMesh {
    let mut mesh = CombatMesh::default();
    for unit in units {
        append_turret(&mut mesh.vertices, unit);
    }
    mesh.turrets = 0..mesh.vertices.len() as u32;
    for tracer in presentation.tracers() {
        let opacity = tracer.opacity(now);
        append_tracer(&mut mesh.vertices, tracer.start, tracer.end, opacity);
    }
    mesh.tracers = mesh.turrets.end..mesh.vertices.len() as u32;
    for flash in presentation.flashes() {
        append_flash(
            &mut mesh.vertices,
            flash.position,
            flash.scale(now),
            flash.opacity(now),
        );
    }
    mesh.flashes = mesh.tracers.end..mesh.vertices.len() as u32;
    mesh
}

fn team_color(owner: Option<PlayerId>) -> [f32; 4] {
    match owner {
        Some(PlayerId(2)) => PLAYER_TWO_COLOR,
        _ => PLAYER_ONE_COLOR,
    }
}

fn append_turret(vertices: &mut Vec<CombatVertex>, unit: &Unit) {
    let center = unit.state.position;
    append_disc(
        vertices,
        center,
        MOUNT_RADIUS,
        OUTLINE_COLOR,
        MOUNT_SEGMENTS,
    );
    append_disc(
        vertices,
        center,
        MOUNT_CORE_RADIUS,
        team_color(unit.owner),
        MOUNT_SEGMENTS,
    );
    let heading = unit.state.heading_radians + unit.combat.turret.local_heading_radians;
    let forward = Vec2::new(-heading.sin(), heading.cos());
    let start = center + forward * MOUNT_CORE_RADIUS;
    let end = center + forward * MUZZLE_OFFSET_METERS;
    append_quad(vertices, start, end, BARREL_OUTER_WIDTH, OUTLINE_COLOR);
    append_quad(
        vertices,
        start,
        end,
        BARREL_CORE_WIDTH,
        team_color(unit.owner),
    );
}

fn append_tracer(vertices: &mut Vec<CombatVertex>, start: Vec2, end: Vec2, opacity: f32) {
    if !start.is_finite() || !end.is_finite() || start.distance_squared(end) <= f32::EPSILON {
        return;
    }
    append_quad(
        vertices,
        start,
        end,
        TRACER_OUTER_WIDTH,
        with_alpha(TRACER_OUTER_COLOR, opacity),
    );
    append_quad(
        vertices,
        start,
        end,
        TRACER_CORE_WIDTH,
        with_alpha(TRACER_CORE_COLOR, opacity),
    );
}

fn append_flash(vertices: &mut Vec<CombatVertex>, center: Vec2, scale: f32, opacity: f32) {
    if !center.is_finite() || opacity <= 0.0 {
        return;
    }
    let color = with_alpha(FLASH_COLOR, opacity);
    let radius = FLASH_RADIUS * scale;
    append_quad(
        vertices,
        center - Vec2::X * radius,
        center + Vec2::X * radius,
        radius * 0.42,
        color,
    );
    append_quad(
        vertices,
        center - Vec2::Y * radius,
        center + Vec2::Y * radius,
        radius * 0.42,
        color,
    );
    append_disc(
        vertices,
        center,
        radius * 0.48,
        with_alpha(TRACER_CORE_COLOR, opacity),
        4,
    );
}

fn append_disc(
    vertices: &mut Vec<CombatVertex>,
    center: Vec2,
    radius: f32,
    color: [f32; 4],
    segments: usize,
) {
    for index in 0..segments {
        let angle_a = index as f32 / segments as f32 * std::f32::consts::TAU;
        let angle_b = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        vertices.extend([
            vertex(center, color),
            vertex(
                center + Vec2::new(angle_a.cos(), angle_a.sin()) * radius,
                color,
            ),
            vertex(
                center + Vec2::new(angle_b.cos(), angle_b.sin()) * radius,
                color,
            ),
        ]);
    }
}

fn append_quad(
    vertices: &mut Vec<CombatVertex>,
    start: Vec2,
    end: Vec2,
    width: f32,
    color: [f32; 4],
) {
    let delta = end - start;
    let Some(direction) = delta.try_normalize() else {
        return;
    };
    let offset = Vec2::new(-direction.y, direction.x) * (width * 0.5);
    let a = start + offset;
    let b = start - offset;
    let c = end - offset;
    let d = end + offset;
    vertices.extend([
        vertex(a, color),
        vertex(b, color),
        vertex(c, color),
        vertex(a, color),
        vertex(c, color),
        vertex(d, color),
    ]);
}

fn vertex(position: Vec2, color: [f32; 4]) -> CombatVertex {
    CombatVertex {
        position: position.to_array(),
        color,
    }
}
fn with_alpha(mut color: [f32; 4], opacity: f32) -> [f32; 4] {
    color[3] *= opacity.clamp(0.0, 1.0);
    color
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CombatUniform {
    viewport: [f32; 4],
}

pub(crate) struct CombatFrame<'a> {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) view_height: f32,
    pub(crate) units: &'a [Unit],
    pub(crate) presentation: &'a CombatPresentation,
    pub(crate) now: Instant,
}

pub(crate) struct CombatRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    capacity: u64,
    mesh: CombatMesh,
}

impl CombatRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("combat shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("combat_shader.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("combat layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<CombatUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("combat pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("combat pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(CombatVertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("combat viewport uniform"),
            contents: bytemuck::bytes_of(&CombatUniform {
                viewport: [1.0, 1.0, 0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("combat bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let capacity = std::mem::size_of::<CombatVertex>() as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("combat vertices"),
            size: capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            capacity,
            mesh: CombatMesh::default(),
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: CombatFrame<'_>,
    ) {
        self.mesh = build_mesh(frame.units, frame.presentation, frame.now);
        let aspect = frame.width.max(1) as f32 / frame.height.max(1) as f32;
        let half_height = frame.view_height * 0.5;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&CombatUniform {
                viewport: [1.0 / (half_height * aspect), 1.0 / half_height, 0.0, 0.0],
            }),
        );
        let bytes = bytemuck::cast_slice(&self.mesh.vertices);
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("combat vertices"),
                size: self.capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytes);
        }
    }

    pub(crate) fn draw_turrets(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.draw_range(pass, self.mesh.turrets.clone());
    }
    pub(crate) fn draw_effects(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.draw_range(pass, self.mesh.tracers.clone());
        self.draw_range(pass, self.mesh.flashes.clone());
    }
    fn draw_range(&self, pass: &mut wgpu::RenderPass<'_>, range: Range<u32>) {
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(range, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShipState;
    use crate::combat_presentation::CombatPresentation;
    use spacegame2d_protocol::Tick;
    use spacegame2d_simulation::{SimulationEvent, UnitId};
    use std::time::Instant;

    fn unit(owner: Option<PlayerId>) -> Unit {
        Unit::new(UnitId(1), owner, ShipState::default())
    }

    #[test]
    fn tracer_uses_authoritative_impact_range_and_has_two_nonzero_quads() {
        let now = Instant::now();
        let mut presentation = CombatPresentation::default();
        presentation.ingest(
            now,
            &[SimulationEvent::ShotFired {
                tick: Tick::new(0),
                shooter_id: UnitId(1),
                muzzle_origin: Vec2::ZERO,
                ray_endpoint: Vec2::new(0.0, 12.0),
                impact_position: Vec2::new(0.0, 12.0),
                hit_unit_id: None,
            }],
            &[],
        );
        let mesh = build_mesh(&[], &presentation, now);
        assert_eq!(mesh.tracers.len(), 12);
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| Vec2::from_array(vertex.position).is_finite())
        );
    }

    #[test]
    fn empty_mesh_has_contiguous_empty_ranges() {
        let mesh = build_mesh(&[], &CombatPresentation::default(), Instant::now());
        assert!(mesh.turrets.is_empty() && mesh.tracers.is_empty() && mesh.flashes.is_empty());
    }

    #[test]
    fn turret_mesh_contains_mount_barrel_and_team_color() {
        let mesh = build_mesh(
            &[unit(Some(PlayerId(2)))],
            &CombatPresentation::default(),
            Instant::now(),
        );
        assert_eq!(mesh.turrets, 0..84);
        assert!(mesh.tracers.is_empty());
        assert!(mesh.flashes.is_empty());
        assert!(
            mesh.vertices
                .iter()
                .any(|vertex| vertex.color == PLAYER_TWO_COLOR)
        );
        assert!(mesh.vertices.iter().all(|vertex| {
            Vec2::from_array(vertex.position).is_finite()
                && vertex.color.iter().all(|component| component.is_finite())
        }));
    }

    #[test]
    fn tracer_rejects_degenerate_and_non_finite_segments() {
        let mut vertices = Vec::new();
        append_tracer(&mut vertices, Vec2::ZERO, Vec2::ZERO, 1.0);
        append_tracer(&mut vertices, Vec2::NAN, Vec2::X, 1.0);
        assert!(vertices.is_empty());
        append_tracer(&mut vertices, Vec2::ZERO, Vec2::Y, 2.0);
        assert_eq!(vertices.len(), 12);
        assert!(vertices.iter().all(|vertex| vertex.color[3] <= 1.0));
    }

    #[test]
    fn flash_rejects_invalid_input_and_builds_cross_and_disc() {
        let mut vertices = Vec::new();
        append_flash(&mut vertices, Vec2::NAN, 1.0, 1.0);
        append_flash(&mut vertices, Vec2::ZERO, 1.0, 0.0);
        assert!(vertices.is_empty());
        append_flash(&mut vertices, Vec2::ZERO, 2.0, 0.5);
        assert_eq!(vertices.len(), 24);
        assert!(vertices.iter().any(|vertex| vertex.color[3] == 0.45));
        assert!(vertices.iter().any(|vertex| vertex.color[3] == 0.225));
    }

    #[test]
    fn primitive_builders_handle_degenerate_quads_and_disc_segments() {
        let mut vertices = Vec::new();
        append_quad(&mut vertices, Vec2::ZERO, Vec2::ZERO, 1.0, OUTLINE_COLOR);
        assert!(vertices.is_empty());
        append_disc(&mut vertices, Vec2::ZERO, 1.0, OUTLINE_COLOR, 5);
        assert_eq!(vertices.len(), 15);
        assert!(vertices.iter().all(|vertex| vertex.color == OUTLINE_COLOR));
    }

    #[test]
    fn alpha_is_clamped_and_vertex_layout_matches_vertex_type() {
        assert_eq!(with_alpha([1.0, 0.5, 0.25, 0.5], -1.0)[3], 0.0);
        assert_eq!(with_alpha([1.0, 0.5, 0.25, 0.5], 2.0)[3], 0.5);
        let layout = CombatVertex::layout();
        assert_eq!(
            layout.array_stride,
            std::mem::size_of::<CombatVertex>() as u64
        );
        assert_eq!(layout.attributes.len(), 2);
    }

    #[test]
    fn mesh_ranges_are_contiguous_when_tracer_and_flash_are_present() {
        let now = Instant::now();
        let mut presentation = CombatPresentation::default();
        let target = unit(Some(PlayerId(1)));
        presentation.ingest(
            now,
            &[SimulationEvent::ShotFired {
                tick: Tick::new(0),
                shooter_id: UnitId(1),
                muzzle_origin: Vec2::ZERO,
                ray_endpoint: Vec2::Y * 12.0,
                impact_position: Vec2::Y * 4.0,
                hit_unit_id: Some(target.id),
            }],
            &[target],
        );
        let mesh = build_mesh(&[unit(None)], &presentation, now);
        assert_eq!(mesh.turrets.end, mesh.tracers.start);
        assert_eq!(mesh.tracers.end, mesh.flashes.start);
        assert_eq!(mesh.flashes.end, mesh.vertices.len() as u32);
        assert!(!mesh.tracers.is_empty());
        assert!(!mesh.flashes.is_empty());
    }
}
