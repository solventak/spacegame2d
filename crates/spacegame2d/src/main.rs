//! `spacegame2d` GUI application.
//!
//! Sets up a `wgpu` + `winit` window, runs the fixed-timestep frame loop
//! driving the [`spacegame2d_simulation`] crate, and renders the arena ring,
//! and renders the autopilot drone fleet. Destinations are set by right-click.
//!
//! See the [`spacegame2d_simulation`] crate for the simulation model itself.

mod geometry;
pub mod network;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use spacegame2d_protocol::{CommandRejected, CommandRejectionReason, Tick};
use spacegame2d_simulation::{
    command::PlayerId,
    command::Unit,
    simulation::{SIMULATION_HZ, ShipState, Simulation, SimulationEvent},
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::geometry::{Vertex, overlay::ring_vertices, units::notched_ship_vertices};
use crate::network::ServerEvent;
#[cfg(test)]
use spacegame2d_simulation::simulation::WORLD_RADIUS_M;

const VIEW_HEIGHT_METERS: f32 = 40.0;
const PLAYER_ONE_COLOR: [f32; 4] = [0.0, 0.9, 1.0, 1.0];
const PLAYER_TWO_COLOR: [f32; 4] = [1.0, 0.35, 0.2, 1.0];
const PENDING_MARKER_COLOR: [f32; 4] = [1.0, 0.85, 0.0, 1.0];
const CONFIRMED_MARKER_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / SIMULATION_HZ as u64);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    viewport: [f32; 4],
    ship: [f32; 4],
    marker: [f32; 4],
    ship_color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkerStatus {
    Pending,
    Confirmed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DestinationMarker {
    position: Vec2,
    status: MarkerStatus,
}

#[derive(Debug, Default)]
struct DestinationPresentation {
    pending: Option<(u32, Vec2)>,
    confirmed: Option<Vec2>,
    rejection: Option<(String, Instant)>,
}

impl DestinationPresentation {
    fn begin(&mut self, sequence: u32, destination: Vec2) {
        self.pending = Some((sequence, destination));
        self.rejection = None;
    }

    fn authoritative(
        &mut self,
        local_slot: u32,
        command: &spacegame2d_protocol::AuthoritativeCommand,
    ) {
        if command.command == spacegame2d_protocol::CommandData::ResetSimulation {
            self.clear();
            return;
        }
        if command.player_slot != local_slot {
            return;
        }
        let spacegame2d_protocol::CommandData::SetDestination { destination } = &command.command
        else {
            return;
        };
        let point = Vec2::new(
            f32::from_bits(destination[0]),
            f32::from_bits(destination[1]),
        );
        self.confirmed = Some(point);
        if self
            .pending
            .is_some_and(|(sequence, _)| sequence == command.sequence)
        {
            self.pending = None;
        }
    }

    fn rejected(&mut self, rejection: &CommandRejected, now: Instant) {
        if self
            .pending
            .is_some_and(|(sequence, _)| sequence == rejection.sequence)
        {
            self.pending = None;
        }
        self.rejection = Some((
            rejection_message(rejection.reason).to_owned(),
            now + Duration::from_secs(2),
        ));
    }

    fn clear(&mut self) {
        self.pending = None;
        self.confirmed = None;
        self.rejection = None;
    }

    fn marker(&self) -> Option<DestinationMarker> {
        self.pending
            .map(|(_, position)| DestinationMarker {
                position,
                status: MarkerStatus::Pending,
            })
            .or_else(|| {
                self.confirmed.map(|position| DestinationMarker {
                    position,
                    status: MarkerStatus::Confirmed,
                })
            })
    }

    fn rejection_text(&mut self, now: Instant) -> Option<&str> {
        if self
            .rejection
            .as_ref()
            .is_some_and(|(_, deadline)| *deadline <= now)
        {
            self.rejection = None;
        }
        self.rejection.as_ref().map(|(message, _)| message.as_str())
    }
}

fn rejection_message(reason: CommandRejectionReason) -> &'static str {
    match reason {
        CommandRejectionReason::InvalidPlayer => "Command rejected: invalid player",
        CommandRejectionReason::UnauthorizedFleet => "Command rejected: unauthorized fleet",
        CommandRejectionReason::NonFiniteDestination => "Command rejected: invalid destination",
        CommandRejectionReason::DestinationOutsideArena => "Command rejected: outside arena",
        CommandRejectionReason::InvalidCommand => "Command rejected: invalid command",
    }
}

fn fleet_color(owner: Option<PlayerId>) -> [f32; 4] {
    match owner {
        Some(PlayerId(2)) => PLAYER_TWO_COLOR,
        _ => PLAYER_ONE_COLOR,
    }
}

fn window_title(player_slot: u32) -> String {
    format!("Spacegame 2D - Player {player_slot}")
}

fn render_unit_data(units: &[Unit]) -> Vec<(ShipState, [f32; 4])> {
    units
        .iter()
        .map(|unit| (unit.state, fleet_color(unit.owner)))
        .collect()
}

fn scene_uniform(
    width: u32,
    height: u32,
    ship: &ShipState,
    marker: Option<Vec2>,
    ship_color: [f32; 4],
) -> SceneUniform {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let half_height = VIEW_HEIGHT_METERS * 0.5;
    let half_width = half_height * aspect;
    SceneUniform {
        viewport: [1.0 / half_width, 1.0 / half_height, 0.0, 0.0],
        ship: [
            ship.position.x,
            ship.position.y,
            ship.heading_radians.sin(),
            ship.heading_radians.cos(),
        ],
        marker: marker.map_or([0.0, 0.0, 0.0, 0.0], |p| [p.x, p.y, 1.0, 0.0]),
        ship_color,
    }
}

struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    ring_vertex_buffer: wgpu::Buffer,
    ring_vertex_count: u32,
    ring_scene_buffer: wgpu::Buffer,
    ring_bind_group: wgpu::BindGroup,
    marker_scene_buffer: wgpu::Buffer,
    marker_bind_group: wgpu::BindGroup,
    scene_buffers: Vec<wgpu::Buffer>,
    scene_bind_groups: Vec<wgpu::BindGroup>,
}

impl Renderer {
    async fn new(window: Arc<Window>, units: &[Unit]) -> Result<Self, String> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("failed to create surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("failed to find adapter: {e}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("spacegame2d device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("failed to create device: {e}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let present_mode = if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            capabilities.present_modes[0]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ship shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<SceneUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ship pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ship pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(Vertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("notched ship vertices"),
            contents: bytemuck::cast_slice(&notched_ship_vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ring_vertices = ring_vertices();
        let ring_vertex_count = ring_vertices.len() as u32;
        let ring_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ring vertices"),
            contents: bytemuck::cast_slice(&ring_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ring_scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ring scene uniform"),
            contents: bytemuck::bytes_of(&scene_uniform(
                config.width,
                config.height,
                &ShipState::default(),
                None,
                PLAYER_ONE_COLOR,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ring_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ring bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ring_scene_buffer.as_entire_binding(),
            }],
        });
        let marker_scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("destination marker scene uniform"),
            contents: bytemuck::bytes_of(&scene_uniform(
                config.width,
                config.height,
                &ShipState::default(),
                Some(Vec2::ZERO),
                PENDING_MARKER_COLOR,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let marker_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("destination marker bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: marker_scene_buffer.as_entire_binding(),
            }],
        });
        let scene_uniforms = render_unit_data(units)
            .into_iter()
            .map(|(ship, color)| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("scene uniform"),
                    contents: bytemuck::bytes_of(&scene_uniform(
                        config.width,
                        config.height,
                        &ship,
                        None,
                        color,
                    )),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                })
            })
            .collect::<Vec<_>>();
        let scene_bind_groups = scene_uniforms
            .iter()
            .map(|scene_buffer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("scene bind group"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: scene_buffer.as_entire_binding(),
                    }],
                })
            })
            .collect::<Vec<_>>();
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            vertex_buffer,
            ring_vertex_buffer,
            ring_vertex_count,
            ring_scene_buffer,
            ring_bind_group,
            marker_scene_buffer,
            marker_bind_group,
            scene_buffers: scene_uniforms,
            scene_bind_groups,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
    fn render(
        &mut self,
        units: &[Unit],
        _ship: Option<&ShipState>,
        marker: Option<DestinationMarker>,
    ) -> Result<(), wgpu::CurrentSurfaceTexture> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            error => return Err(error),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ship encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ship pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            for (index, unit) in units.iter().enumerate() {
                self.queue.write_buffer(
                    &self.scene_buffers[index],
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        &unit.state,
                        None,
                        fleet_color(unit.owner),
                    )),
                );
                pass.set_bind_group(0, &self.scene_bind_groups[index], &[]);
                pass.draw(0..24, 0..1);
            }
            self.queue.write_buffer(
                &self.ring_scene_buffer,
                0,
                bytemuck::bytes_of(&scene_uniform(
                    self.config.width,
                    self.config.height,
                    &ShipState::default(),
                    None,
                    PLAYER_ONE_COLOR,
                )),
            );
            pass.set_vertex_buffer(0, self.ring_vertex_buffer.slice(..));
            pass.set_bind_group(0, &self.ring_bind_group, &[]);
            pass.draw(0..self.ring_vertex_count, 0..1);
            if let Some(marker) = marker {
                let color = match marker.status {
                    MarkerStatus::Pending => PENDING_MARKER_COLOR,
                    MarkerStatus::Confirmed => CONFIRMED_MARKER_COLOR,
                };
                self.queue.write_buffer(
                    &self.marker_scene_buffer,
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        &ShipState::default(),
                        Some(marker.position),
                        color,
                    )),
                );
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_bind_group(0, &self.marker_bind_group, &[]);
                pass.draw(24..30, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}

struct App {
    renderer: Option<Renderer>,
    simulation: Simulation,
    presentation: DestinationPresentation,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    next_tick: Instant,
    network: Option<network::NetworkSession>,
    scheduled: std::collections::BTreeMap<Tick, Vec<spacegame2d_protocol::AuthoritativeCommand>>,
    next_sequence: u32,
    presentation_events: PresentationEventLog,
}

/// Client-only event state used by presentation code.
///
/// Simulation events remain transient consequences of stepping the
/// authoritative world. Keeping this log here means presentation effects can
/// outlive a frame without adding event state to `World`, replay history, or
/// the network protocol.
#[derive(Debug, Default, PartialEq)]
struct PresentationEventLog {
    events: Vec<SimulationEvent>,
}

impl PresentationEventLog {
    pub(crate) fn append(&mut self, events: impl IntoIterator<Item = SimulationEvent>) {
        self.events.extend(events);
    }

    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            simulation: Simulation::default(),
            presentation: DestinationPresentation::default(),
            cursor_position: None,
            next_tick: Instant::now(),
            network: None,
            scheduled: std::collections::BTreeMap::new(),
            next_sequence: 1,
            presentation_events: PresentationEventLog::default(),
        }
    }
}

fn screen_to_world(cursor: winit::dpi::PhysicalPosition<f64>, width: u32, height: u32) -> Vec2 {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let half_height = VIEW_HEIGHT_METERS * 0.5;
    let half_width = half_height * aspect;
    let x = (cursor.x as f32 / width.max(1) as f32 * 2.0 - 1.0) * half_width;
    let y = (1.0 - cursor.y as f32 / height.max(1) as f32 * 2.0) * half_height;
    Vec2::new(x, y)
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let address = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "127.0.0.1:4000".into());
        match network::NetworkSession::connect(&address) {
            Ok(session) => {
                self.simulation = Simulation::default();
                self.simulation.set_tick(session.server_tick);
                if let Err(error) = session.register_player(&mut self.simulation) {
                    eprintln!("failed to register connected player: {error}");
                    event_loop.exit();
                    return;
                }
                self.network = Some(session);
            }
            Err(error) => {
                eprintln!("failed to connect to server {address}: {error}");
                event_loop.exit();
                return;
            }
        }
        let title = self.network.as_ref().map_or_else(
            || "Spacegame 2D".to_owned(),
            |session| window_title(session.player_slot),
        );
        let window = match event_loop.create_window(Window::default_attributes().with_title(title))
        {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        match pollster::block_on(Renderer::new(window.clone(), &self.simulation.world.units)) {
            Ok(renderer) => {
                self.next_tick = Instant::now() + TICK_DURATION;
                window.request_redraw();
                self.renderer = Some(renderer);
            }
            Err(e) => {
                eprintln!("failed to initialize renderer: {e}");
                event_loop.exit();
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(session) = self.network.as_mut() {
            session.set_local_tick(self.simulation.tick());
            match session.poll_events() {
                Ok(events) => {
                    for event in events {
                        match event {
                            ServerEvent::Authoritative(command) => {
                                self.presentation.authoritative(
                                    self.network.as_ref().map_or(0, |s| s.player_slot),
                                    &command,
                                );
                                self.scheduled
                                    .entry(command.execute_tick)
                                    .or_default()
                                    .push(command);
                            }
                            ServerEvent::Rejected(rejection) => {
                                self.presentation.rejected(&rejection, Instant::now());
                            }
                        }
                    }
                }
                Err(error) => {
                    eprintln!("server connection lost: {error}");
                    event_loop.exit();
                    return;
                }
            }
        }
        let now = Instant::now();
        if let Some(renderer) = self.renderer.as_ref() {
            let title = self.presentation.rejection_text(now).map_or_else(
                || {
                    window_title(
                        self.network
                            .as_ref()
                            .map_or(0, |session| session.player_slot),
                    )
                },
                |message| {
                    format!(
                        "{} — {}",
                        window_title(
                            self.network
                                .as_ref()
                                .map_or(0, |session| session.player_slot)
                        ),
                        message
                    )
                },
            );
            renderer.window.set_title(&title);
        }
        while now >= self.next_tick {
            self.simulation.apply_due_commands(&mut self.scheduled);
            let events = self.simulation.step().unwrap_or_default();
            self.presentation_events.append(events);
            self.next_tick += TICK_DURATION;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
        if let Some(renderer) = &self.renderer {
            renderer.window.request_redraw();
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(false) => {}
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if let (Some(cursor), Some(renderer)) =
                    (self.cursor_position, self.renderer.as_ref())
                {
                    let destination =
                        screen_to_world(cursor, renderer.config.width, renderer.config.height);
                    self.presentation.begin(self.next_sequence, destination);
                    if let Some(session) = self.network.as_mut()
                        && let Err(error) = session.send_set_destination(
                            self.next_sequence,
                            [destination.x.to_bits(), destination.y.to_bits()],
                        )
                    {
                        eprintln!("failed to send destination: {error}");
                        event_loop.exit();
                    }
                    self.next_sequence = self.next_sequence.saturating_add(1);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::KeyR),
                        ..
                    },
                ..
            } => {
                if let Some(session) = self.network.as_mut() {
                    if let Err(error) = session.send_reset_simulation(self.next_sequence) {
                        eprintln!("failed to send reset: {error}");
                        event_loop.exit();
                    }
                    self.next_sequence = self.next_sequence.saturating_add(1);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(
                        &self.simulation.world.units,
                        None,
                        self.presentation.marker(),
                    ) {
                        Ok(()) => {}
                        Err(
                            wgpu::CurrentSurfaceTexture::Lost
                            | wgpu::CurrentSurfaceTexture::Outdated,
                        ) => renderer.resize(renderer.config.width, renderer.config.height),
                        Err(
                            wgpu::CurrentSurfaceTexture::Timeout
                            | wgpu::CurrentSurfaceTexture::Occluded,
                        ) => {}
                        Err(e) => {
                            eprintln!("surface error: {e:?}");
                            event_loop.exit();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let _logging =
        spacegame2d_logging::init("spacegame2d", "info").expect("failed to initialize logging");
    tracing::info!(event = "client_starting", "spacegame2d starting");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fleet_color_maps_player_two_to_coral() {
        assert_eq!(fleet_color(Some(PlayerId(1))), PLAYER_ONE_COLOR);
        assert_eq!(fleet_color(Some(PlayerId(2))), PLAYER_TWO_COLOR);
        assert_eq!(fleet_color(None), PLAYER_ONE_COLOR);
    }

    #[test]
    fn window_title_includes_player_slot() {
        assert_eq!(window_title(1), "Spacegame 2D - Player 1");
        assert_eq!(window_title(2), "Spacegame 2D - Player 2");
    }

    #[test]
    fn render_unit_data_covers_every_unit_with_owner_color() {
        let mut world = spacegame2d_simulation::World::demo();
        world.assign_mirror_owners();
        let data = render_unit_data(&world.units);
        assert_eq!(data.len(), 60);
        assert!(
            data[..30]
                .iter()
                .all(|(_, color)| *color == PLAYER_ONE_COLOR)
        );
        assert!(
            data[30..]
                .iter()
                .all(|(_, color)| *color == PLAYER_TWO_COLOR)
        );
    }

    #[test]
    fn destination_presentation_tracks_confirmation_rejection_and_reset() {
        let now = Instant::now();
        let mut presentation = DestinationPresentation::default();
        presentation.begin(4, Vec2::new(2.0, 3.0));
        assert_eq!(presentation.marker().unwrap().status, MarkerStatus::Pending);
        let command = spacegame2d_protocol::AuthoritativeCommand {
            execute_tick: Tick::new(2),
            player_slot: 1,
            sequence: 4,
            command: spacegame2d_protocol::CommandData::SetDestination {
                destination: [2.0f32.to_bits(), 3.0f32.to_bits()],
            },
        };
        presentation.authoritative(1, &command);
        assert_eq!(
            presentation.marker().unwrap().status,
            MarkerStatus::Confirmed
        );
        presentation.begin(5, Vec2::new(40.0, 0.0));
        presentation.rejected(
            &CommandRejected {
                sequence: 5,
                reason: CommandRejectionReason::DestinationOutsideArena,
            },
            now,
        );
        assert_eq!(
            presentation.marker().unwrap().status,
            MarkerStatus::Confirmed
        );
        assert!(presentation.rejection_text(now).is_some());
        presentation.authoritative(
            1,
            &spacegame2d_protocol::AuthoritativeCommand {
                execute_tick: Tick::new(3),
                player_slot: 1,
                sequence: 6,
                command: spacegame2d_protocol::CommandData::ResetSimulation,
            },
        );
        assert!(presentation.marker().is_none());
        assert!(presentation.rejection_text(now).is_none());
    }

    #[test]
    fn screen_to_world_preserves_outside_arena_coordinates() {
        let point = screen_to_world(winit::dpi::PhysicalPosition::new(-1000.0, 5000.0), 800, 600);
        assert!(point.length() > WORLD_RADIUS_M);
        assert!(point.x < 0.0 && point.y < 0.0);
    }

    #[test]
    fn screen_to_world_keeps_inside_click_inside_circular_arena() {
        let point = screen_to_world(winit::dpi::PhysicalPosition::new(400.0, 300.0), 800, 600);
        assert_eq!(point, Vec2::ZERO);

        let point = screen_to_world(winit::dpi::PhysicalPosition::new(600.0, 300.0), 800, 600);
        assert!(point.length() < WORLD_RADIUS_M);
    }

    #[test]
    fn scene_uniform_uses_fixed_world_scale() {
        let u = scene_uniform(1000, 1000, &ShipState::default(), None, PLAYER_ONE_COLOR);
        // Square viewport: half_width == half_height == VIEW_HEIGHT_METERS * 0.5.
        let expected = 1.0 / (VIEW_HEIGHT_METERS * 0.5);
        assert!((u.viewport[0] - expected).abs() < 0.0001);
        assert!((u.viewport[1] - expected).abs() < 0.0001);
    }
    #[test]
    fn ship_mesh_preserves_rear_notch() {
        let vertices = notched_ship_vertices();
        assert_eq!(vertices.len(), 30);
        assert!(vertices[12].position[1].abs() < vertices[0].position[1].abs());
    }
    #[test]
    fn ring_mesh_has_expected_vertex_count() {
        let vertices = ring_vertices();
        assert_eq!(vertices.len(), 128 * 6);
    }

    #[test]
    fn presentation_event_log_aggregates_and_clears_locally() {
        let mut log = PresentationEventLog::default();
        let first = SimulationEvent::BoundaryCrossed {
            tick: Tick::from(4),
            unit_id: spacegame2d_simulation::UnitId(2),
            position: Vec2::new(17.0, 0.0),
        };
        let second = SimulationEvent::BoundaryCrossed {
            tick: Tick::from(5),
            unit_id: spacegame2d_simulation::UnitId(3),
            position: Vec2::new(-17.0, 0.0),
        };

        log.append([first]);
        log.append([second]);

        assert_eq!(log.events, vec![first, second]);
        log.clear();
        assert!(log.events.is_empty());
    }
}
