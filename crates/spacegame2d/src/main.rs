//! `spacegame2d` GUI application.
//!
//! Sets up a `wgpu` + `winit` window, runs the fixed-timestep frame loop
//! driving the [`spacegame2d_simulation`] crate, and renders the arena ring,
//! and renders the autopilot drone fleet. Destinations are set by right-click.
//!
//! See the [`spacegame2d_simulation`] crate for the simulation model itself.

mod camera;
mod combat_presentation;
mod combat_rendering;
mod geometry;
mod hud;
mod match_session;
pub mod network;
mod player_presentation;
mod presentation;
mod session;
mod ui_bridge;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use spacegame2d_protocol::{DisplayName, Tick};
use spacegame2d_simulation::{
    SimulationConfig, StaticStructure,
    command::PlayerId,
    command::Unit,
    simulation::{SIMULATION_HZ, ShipState, Simulation},
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
};

use crate::camera::Camera;
#[cfg(test)]
use crate::camera::VIEW_HEIGHT_METERS;
use crate::combat_presentation::CombatPresentation;
use crate::combat_rendering::{CombatFrame, CombatRenderer};
use crate::geometry::{
    Vertex, overlay::ring_vertices, structures::structure_vertices, units::notched_ship_vertices,
};
use crate::hud::HudWebView;
use crate::match_session::MatchSessionPresenter;
use crate::network::ServerEvent;
#[cfg(test)]
use crate::player_presentation::PLAYER_TWO_COLOR;
use crate::player_presentation::{PLAYER_ONE_COLOR, PlayerColor};
use crate::presentation::{DestinationMarker, DestinationPresentation, MarkerStatus};
use crate::session::{
    ConnectionOutcome, ConnectionProgress, DEFAULT_SERVER_ADDRESS, HandshakeOutcome,
    SessionLifecycle,
};
use crate::ui_bridge::{BridgeHealthConfig, UiBridge};
use spacegame2d_ui_protocol::{
    EngineToUiMessage, MatchSessionResetReason, ProtocolErrorCode, RequestId,
    UI_ENGINE_PROTOCOL_VERSION, UiToEngineMessage,
};

enum AppEvent {
    UiMessage(UiToEngineMessage),
    ProtocolError(ProtocolErrorCode),
    ConnectionProgress {
        attempt_id: RequestId,
        progress: ConnectionProgress,
    },
    ConnectionFinished {
        attempt_id: RequestId,
        result: Box<Result<network::NetworkSession, network::ConnectError>>,
    },
}
#[cfg(test)]
use spacegame2d_simulation::simulation::WORLD_RADIUS_M;

const PENDING_MARKER_COLOR: [f32; 4] = [1.0, 0.85, 0.0, 1.0];
const CONFIRMED_MARKER_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / SIMULATION_HZ as u64);
/// All current pipelines share this value so a future MSAA color target can change it centrally.
const RENDER_SAMPLE_COUNT: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    viewport: [f32; 4],
    ship: [f32; 4],
    marker: [f32; 4],
    ship_color: [f32; 4],
}

fn fleet_color(owner: Option<PlayerId>) -> [f32; 4] {
    PlayerColor::for_slot(owner.map_or(1, |PlayerId(slot)| slot as u32)).render_rgba()
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
    camera: Camera,
    ship: &ShipState,
    marker: Option<Vec2>,
    ship_color: [f32; 4],
) -> SceneUniform {
    SceneUniform {
        viewport: camera.viewport(width, height),
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
    structure_vertex_buffer: wgpu::Buffer,
    structure_vertex_count: u32,
    structure_scene_buffer: wgpu::Buffer,
    structure_bind_group: wgpu::BindGroup,
    marker_scene_buffer: wgpu::Buffer,
    marker_bind_group: wgpu::BindGroup,
    scene_buffers: Vec<wgpu::Buffer>,
    scene_bind_groups: Vec<wgpu::BindGroup>,
    combat_renderer: CombatRenderer,
}

impl Renderer {
    async fn new(
        window: Arc<Window>,
        units: &[Unit],
        world_radius: f32,
        structures: &[StaticStructure],
    ) -> Result<Self, String> {
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
            multisample: wgpu::MultisampleState {
                count: RENDER_SAMPLE_COUNT,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("notched ship vertices"),
            contents: bytemuck::cast_slice(&notched_ship_vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ring_vertices = ring_vertices(world_radius);
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
                Camera::new(world_radius),
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
        let structure_vertices = structure_vertices(structures);
        let structure_vertex_count = structure_vertices.len() as u32;
        let structure_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("static structure vertices"),
                contents: bytemuck::cast_slice(&structure_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let structure_scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static structure scene uniform"),
            contents: bytemuck::bytes_of(&scene_uniform(
                config.width,
                config.height,
                Camera::new(world_radius),
                &ShipState::default(),
                None,
                PLAYER_ONE_COLOR,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let structure_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("static structure bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: structure_scene_buffer.as_entire_binding(),
            }],
        });
        let marker_scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("destination marker scene uniform"),
            contents: bytemuck::bytes_of(&scene_uniform(
                config.width,
                config.height,
                Camera::new(world_radius),
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
                        Camera::new(world_radius),
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
        let combat_renderer = CombatRenderer::new(&device, format, RENDER_SAMPLE_COUNT);
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
            structure_vertex_buffer,
            structure_vertex_count,
            structure_scene_buffer,
            structure_bind_group,
            marker_scene_buffer,
            marker_bind_group,
            scene_buffers: scene_uniforms,
            scene_bind_groups,
            combat_renderer,
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
        marker: Option<DestinationMarker>,
        combat_presentation: &CombatPresentation,
        camera: Camera,
        now: Instant,
    ) -> Result<(), wgpu::CurrentSurfaceTexture> {
        self.combat_renderer.prepare(
            &self.device,
            &self.queue,
            CombatFrame {
                viewport: camera.viewport(self.config.width, self.config.height),
                units,
                presentation: combat_presentation,
                now,
            },
        );
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
            self.queue.write_buffer(
                &self.structure_scene_buffer,
                0,
                bytemuck::bytes_of(&scene_uniform(
                    self.config.width,
                    self.config.height,
                    camera,
                    &ShipState::default(),
                    None,
                    PLAYER_ONE_COLOR,
                )),
            );
            pass.set_vertex_buffer(0, self.structure_vertex_buffer.slice(..));
            pass.set_bind_group(0, &self.structure_bind_group, &[]);
            pass.draw(0..self.structure_vertex_count, 0..1);

            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            for (index, unit) in units.iter().enumerate() {
                self.queue.write_buffer(
                    &self.scene_buffers[index],
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        camera,
                        &unit.state,
                        None,
                        fleet_color(unit.owner),
                    )),
                );
                pass.set_bind_group(0, &self.scene_bind_groups[index], &[]);
                pass.draw(0..24, 0..1);
            }
            self.combat_renderer.draw_turrets(&mut pass);
            pass.set_pipeline(&self.pipeline);
            self.queue.write_buffer(
                &self.ring_scene_buffer,
                0,
                bytemuck::bytes_of(&scene_uniform(
                    self.config.width,
                    self.config.height,
                    camera,
                    &ShipState::default(),
                    None,
                    PLAYER_ONE_COLOR,
                )),
            );
            pass.set_vertex_buffer(0, self.ring_vertex_buffer.slice(..));
            pass.set_bind_group(0, &self.ring_bind_group, &[]);
            pass.draw(0..self.ring_vertex_count, 0..1);
            self.combat_renderer.draw_effects(&mut pass);
            if let Some(marker) = marker {
                let color = match marker.status {
                    MarkerStatus::Pending => PENDING_MARKER_COLOR,
                    MarkerStatus::Confirmed => CONFIRMED_MARKER_COLOR,
                };
                pass.set_pipeline(&self.pipeline);
                self.queue.write_buffer(
                    &self.marker_scene_buffer,
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        camera,
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
    // Keep the child WebView ahead of its renderer/parent-window owner so it drops first.
    hud: Option<HudWebView>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    renderer_config: SimulationConfig,
    simulation: Simulation,
    presentation: DestinationPresentation,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    next_tick: Instant,
    network: Option<network::NetworkSession>,
    lifecycle: SessionLifecycle<()>,
    bridge: UiBridge,
    match_session: MatchSessionPresenter,
    proxy: EventLoopProxy<AppEvent>,
    scheduled: std::collections::BTreeMap<Tick, Vec<spacegame2d_protocol::AuthoritativeCommand>>,
    next_sequence: u32,
    combat_presentation: CombatPresentation,
    match_result: Option<spacegame2d_simulation::MatchResult>,
    camera: Camera,
    window_title: Option<String>,
}

impl App {
    fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            hud: None,
            window: None,
            renderer: None,
            renderer_config: SimulationConfig::default(),
            simulation: Simulation::default(),
            presentation: DestinationPresentation::default(),
            cursor_position: None,
            next_tick: Instant::now(),
            network: None,
            lifecycle: SessionLifecycle::new(DEFAULT_SERVER_ADDRESS),
            bridge: UiBridge::new(BridgeHealthConfig::default()),
            match_session: MatchSessionPresenter::default(),
            proxy,
            scheduled: std::collections::BTreeMap::new(),
            next_sequence: 1,
            combat_presentation: CombatPresentation::default(),
            match_result: None,
            camera: Camera::new(spacegame2d_simulation::DEFAULT_WORLD_RADIUS_METERS),
            window_title: None,
        }
    }

    fn publish_state(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(hud), Some(window)) = (self.hud.as_mut(), self.window.as_ref()) else {
            return;
        };
        let Some(bridge_id) = self.bridge.bridge_id().cloned() else {
            return;
        };
        let message = EngineToUiMessage::ConnectionStateChanged {
            protocol_version: UI_ENGINE_PROTOCOL_VERSION,
            bridge_id,
            state: self.lifecycle.ui_state(),
        };
        if let Err(error) = hud.publish(window, &message) {
            tracing::error!(event = "hud_state_publish_failed", %error);
            event_loop.exit();
        }
    }

    fn publish_match_state(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(hud), Some(window), Some(bridge_id)) = (
            self.hud.as_mut(),
            self.window.as_ref(),
            self.bridge.bridge_id().cloned(),
        ) else {
            return;
        };
        let message = EngineToUiMessage::MatchSessionStateChanged {
            protocol_version: UI_ENGINE_PROTOCOL_VERSION,
            bridge_id,
            state: self.match_session.state().clone(),
        };
        if let Err(error) = hud.publish(window, &message) {
            tracing::error!(event = "hud_match_session_publish_failed", %error);
            event_loop.exit();
        }
    }

    fn publish_ui_state(&mut self, event_loop: &ActiveEventLoop) {
        self.publish_state(event_loop);
        self.publish_match_state(event_loop);
    }

    fn reset_match_session(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: MatchSessionResetReason,
    ) {
        self.match_session.reset(reason);
        self.publish_match_state(event_loop);
    }

    fn start_attempt(&self, attempt: crate::session::ConnectionAttempt) {
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let progress_proxy = proxy.clone();
            let request_id = attempt.id.clone();
            let result = network::NetworkSession::connect_with_timeout_and_progress(
                &attempt.address,
                &attempt.display_name,
                crate::session::CONNECTION_TIMEOUT,
                move |progress| {
                    let _ = progress_proxy.send_event(AppEvent::ConnectionProgress {
                        attempt_id: request_id.clone(),
                        progress,
                    });
                },
            );
            let _ = proxy.send_event(AppEvent::ConnectionFinished {
                attempt_id: attempt.id,
                result: Box::new(result),
            });
        });
    }

    fn publish_protocol_error(&mut self, event_loop: &ActiveEventLoop, code: ProtocolErrorCode) {
        let (Some(hud), Some(window), Some(bridge_id)) = (
            self.hud.as_mut(),
            self.window.as_ref(),
            self.bridge.bridge_id().cloned(),
        ) else {
            return;
        };
        let message = EngineToUiMessage::ProtocolError {
            protocol_version: UI_ENGINE_PROTOCOL_VERSION,
            bridge_id,
            code,
        };
        if let Err(error) = hud.publish(window, &message) {
            tracing::error!(event = "hud_protocol_error_publish_failed", %error);
            event_loop.exit();
        }
    }

    fn reset_connected_resources(&mut self) {
        self.network = None;
        self.simulation = Simulation::default();
        self.presentation.clear();
        self.scheduled.clear();
        self.combat_presentation.clear();
        self.match_result = None;
        self.next_sequence = 1;
    }

    fn gameplay_available(&self) -> bool {
        self.network
            .as_ref()
            .and_then(network::NetworkSession::match_started_at)
            .is_some()
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let title = "Spacegame 2D".to_owned();
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(&title)
                .with_fullscreen(Some(Fullscreen::Borderless(None))),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window_title = Some(title);
        match pollster::block_on(Renderer::new(
            window.clone(),
            &self.simulation.world.units,
            self.simulation.world_radius(),
            self.simulation.world.structures(),
        )) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => {
                tracing::error!(event = "renderer_initialization_failed", %error);
                event_loop.exit();
                return;
            }
        }
        let proxy = self.proxy.clone();
        match HudWebView::new(&window, move |body| {
            match UiToEngineMessage::decode(&body) {
                Ok(message) => {
                    let _ = proxy.send_event(AppEvent::UiMessage(message));
                }
                Err(error) => {
                    tracing::warn!(event = "hud_command_invalid", %error);
                    let code = match error {
                        spacegame2d_ui_protocol::ProtocolDecodeError::Invalid { code, .. } => code,
                        spacegame2d_ui_protocol::ProtocolDecodeError::Validation(
                            spacegame2d_ui_protocol::ProtocolValidationError::UnsupportedVersion(_),
                        ) => ProtocolErrorCode::UnsupportedVersion,
                        spacegame2d_ui_protocol::ProtocolDecodeError::Validation(_) => {
                            ProtocolErrorCode::InvalidFieldValue
                        }
                    };
                    let _ = proxy.send_event(AppEvent::ProtocolError(code));
                }
            }
        }) {
            Ok(hud) => self.hud = Some(hud),
            Err(error) => {
                tracing::error!(event = "hud_initialization_failed", %error);
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::ProtocolError(code) => self.publish_protocol_error(event_loop, code),
            AppEvent::UiMessage(UiToEngineMessage::UiReady { bridge_id, .. }) => {
                self.bridge.ready(bridge_id, Instant::now());
                self.publish_ui_state(event_loop);
            }
            AppEvent::UiMessage(UiToEngineMessage::ConnectRequested {
                bridge_id,
                request_id,
                address,
                display_name,
                ..
            }) => {
                let Ok(display_name) = DisplayName::try_from(display_name) else {
                    self.publish_protocol_error(event_loop, ProtocolErrorCode::InvalidFieldValue);
                    return;
                };
                if self.bridge.accepts(&bridge_id)
                    && let Some(attempt) = self.lifecycle.connect(
                        request_id,
                        address,
                        display_name.as_str().into(),
                        Instant::now(),
                    )
                {
                    self.reset_match_session(
                        event_loop,
                        MatchSessionResetReason::NewConnectionAttempt,
                    );
                    self.publish_state(event_loop);
                    self.start_attempt(attempt);
                }
            }
            AppEvent::UiMessage(UiToEngineMessage::ConnectionCancelled {
                bridge_id,
                request_id,
                ..
            }) => {
                if self.bridge.accepts(&bridge_id) && self.lifecycle.cancel(&request_id) {
                    self.publish_state(event_loop);
                }
            }
            AppEvent::UiMessage(UiToEngineMessage::DisconnectRequested {
                bridge_id,
                request_id,
                ..
            }) => {
                if self.bridge.accepts(&bridge_id) && self.lifecycle.disconnect(&request_id) {
                    self.reset_connected_resources();
                    self.reset_match_session(event_loop, MatchSessionResetReason::UserDisconnected);
                    self.publish_state(event_loop);
                }
            }
            AppEvent::UiMessage(UiToEngineMessage::HeartbeatAcknowledged {
                bridge_id,
                sequence,
                ..
            }) => {
                let _ = self.bridge.acknowledge(&bridge_id, sequence);
            }
            AppEvent::UiMessage(UiToEngineMessage::BridgeFaultReported {
                bridge_id, code, ..
            }) => {
                if self.bridge.accepts(&bridge_id) {
                    tracing::warn!(event = "hud_bridge_fault_reported", ?code);
                    self.publish_protocol_error(event_loop, code);
                }
            }
            AppEvent::UiMessage(UiToEngineMessage::HudLayoutRequested {
                bridge_id,
                phase,
                transition_duration_ms,
                ..
            }) => {
                if self.bridge.accepts(&bridge_id)
                    && let (Some(hud), Some(window)) = (self.hud.as_mut(), self.window.as_ref())
                    && let Err(error) = hud.set_layout(window, phase, transition_duration_ms)
                {
                    tracing::error!(event = "hud_layout_change_failed", ?phase, %error);
                }
            }
            AppEvent::ConnectionProgress {
                attempt_id,
                progress,
            } => {
                if self.lifecycle.progress(&attempt_id, progress) {
                    self.publish_state(event_loop);
                }
            }
            AppEvent::ConnectionFinished { attempt_id, result } => {
                let mut session = match *result {
                    Ok(session) => session,
                    Err(network::ConnectError::Rejected(reason)) => {
                        let reason = match reason {
                            spacegame2d_protocol::HandshakeRejectionReason::ServerFull => {
                                HandshakeOutcome::ServerFull
                            }
                            spacegame2d_protocol::HandshakeRejectionReason::IncompatibleVersion => {
                                HandshakeOutcome::VersionMismatch
                            }
                            _ => HandshakeOutcome::Rejected,
                        };
                        if self
                            .lifecycle
                            .complete(attempt_id, ConnectionOutcome::Rejected(reason))
                        {
                            self.publish_state(event_loop);
                        }
                        return;
                    }
                    Err(error) => {
                        tracing::warn!(event = "connection_failed", %error);
                        if self
                            .lifecycle
                            .complete(attempt_id, ConnectionOutcome::Failed)
                        {
                            self.publish_state(event_loop);
                        }
                        return;
                    }
                };
                let player_slot = session.player_slot;
                if !self.lifecycle.complete(
                    attempt_id,
                    ConnectionOutcome::Connected {
                        session: (),
                        player_slot,
                    },
                ) {
                    return;
                }
                self.simulation = match session.take_initial_simulation() {
                    Ok(simulation) => simulation,
                    Err(error) => {
                        tracing::error!(event = "snapshot_install_failed", %error);
                        self.lifecycle.session_lost();
                        self.publish_state(event_loop);
                        return;
                    }
                };
                self.camera = Camera::new(self.simulation.world_radius());
                let Some(window) = self.window.clone() else {
                    return;
                };
                if self.renderer_config != self.simulation.config() {
                    self.renderer = None;
                    match pollster::block_on(Renderer::new(
                        window.clone(),
                        &self.simulation.world.units,
                        self.simulation.world_radius(),
                        self.simulation.world.structures(),
                    )) {
                        Ok(renderer) => {
                            self.renderer = Some(renderer);
                            self.renderer_config = self.simulation.config();
                        }
                        Err(error) => {
                            tracing::error!(event = "renderer_initialization_failed", %error);
                            self.lifecycle.session_lost();
                            self.publish_state(event_loop);
                            return;
                        }
                    }
                }
                self.network = Some(session);
                self.next_tick = Instant::now() + TICK_DURATION;
                if let Some(session) = self.network.as_ref()
                    && let Err(error) = self.match_session.update(session)
                {
                    tracing::error!(event = "match_session_presentation_failed", %error);
                    self.reset_connected_resources();
                    self.lifecycle.session_lost();
                    self.reset_match_session(event_loop, MatchSessionResetReason::SessionLost);
                    self.publish_state(event_loop);
                    return;
                }
                self.publish_ui_state(event_loop);
                window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        for _ in 0..8 {
            if !gtk::events_pending() {
                break;
            }
            gtk::main_iteration_do(false);
        }
        let now = Instant::now();
        if let (Some(hud), Some(window)) = (self.hud.as_mut(), self.window.as_ref())
            && let Err(error) = hud.advance(window, now)
        {
            tracing::error!(event = "hud_transition_failed", %error);
            event_loop.exit();
            return;
        }
        if let Some(sequence) = self.bridge.due(now)
            && let (Some(hud), Some(window), Some(bridge_id)) = (
                self.hud.as_mut(),
                self.window.as_ref(),
                self.bridge.bridge_id().cloned(),
            )
        {
            let message = EngineToUiMessage::Heartbeat {
                protocol_version: UI_ENGINE_PROTOCOL_VERSION,
                bridge_id,
                sequence,
            };
            if let Err(error) = hud.publish(window, &message) {
                tracing::error!(event = "hud_heartbeat_publish_failed", %error);
                event_loop.exit();
                return;
            }
        }
        if self.bridge.failed() {
            tracing::error!(event = "hud_bridge_failed");
            event_loop.exit();
            return;
        }
        if self.lifecycle.timeout(now) {
            self.publish_state(event_loop);
        }
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
                            ServerEvent::SessionStateChanged(snapshot) => {
                                tracing::info!(
                                    event = "session_state_changed",
                                    presence = ?snapshot.opponent_presence,
                                    revision = snapshot.presence_revision
                                );
                                if let Some(session) = self.network.as_ref()
                                    && let Err(error) = self.match_session.update(session)
                                {
                                    tracing::error!(event = "match_session_presentation_failed", %error);
                                    self.reset_connected_resources();
                                    self.lifecycle.session_lost();
                                    self.reset_match_session(
                                        event_loop,
                                        MatchSessionResetReason::SessionLost,
                                    );
                                    self.publish_state(event_loop);
                                    return;
                                }
                                self.publish_match_state(event_loop);
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(event = "server_connection_lost", %error);
                    self.reset_connected_resources();
                    self.lifecycle.session_lost();
                    self.reset_match_session(event_loop, MatchSessionResetReason::SessionLost);
                    self.publish_state(event_loop);
                    return;
                }
            }
        }
        if let Some(renderer) = self.renderer.as_ref() {
            let title = self.presentation.rejection_text(now).map_or_else(
                || {
                    self.network.as_ref().map_or_else(
                        || "Spacegame 2D".to_owned(),
                        |session| window_title(session.player_slot),
                    )
                },
                |message| {
                    format!(
                        "{} — {message}",
                        self.network.as_ref().map_or_else(
                            || "Spacegame 2D".to_owned(),
                            |session| window_title(session.player_slot),
                        )
                    )
                },
            );
            if self.window_title.as_ref() != Some(&title) {
                renderer.window.set_title(&title);
                self.window_title = Some(title);
            }
        }
        while self.lifecycle.is_connected() && now >= self.next_tick {
            let applied = self.simulation.apply_due_commands(&mut self.scheduled);
            if applied.reset_applied {
                self.presentation.clear();
                self.combat_presentation.clear();
            }
            let events = self.simulation.step().unwrap_or_default();
            if let Some(spacegame2d_simulation::SimulationEvent::MatchResult { outcome, .. }) =
                events.iter().find(|event| {
                    matches!(
                        event,
                        spacegame2d_simulation::SimulationEvent::MatchResult { .. }
                    )
                })
            {
                self.scheduled.clear();
                self.presentation.clear();
                self.combat_presentation.clear();
                self.match_result = Some(*outcome);
            }
            self.combat_presentation.ingest(now, &events);
            self.combat_presentation.retain_active(now);
            if let Some(session) = self.network.as_mut()
                && self
                    .simulation
                    .tick()
                    .0
                    .is_multiple_of(u64::from(SIMULATION_HZ))
            {
                let _ = session
                    .send_state_checksum(self.simulation.tick(), self.simulation.state_hash());
            }
            self.next_tick += TICK_DURATION;
        }
        let elapsed_update = self.network.as_ref().map(|session| {
            self.match_session
                .update_if_elapsed_changed(session)
                .map(|state| state.is_some())
        });
        match elapsed_update {
            Some(Ok(true)) => self.publish_match_state(event_loop),
            Some(Err(error)) => {
                tracing::error!(event = "match_session_presentation_failed", %error);
                self.reset_connected_resources();
                self.lifecycle.session_lost();
                self.reset_match_session(event_loop, MatchSessionResetReason::SessionLost);
                self.publish_state(event_loop);
                return;
            }
            _ => {}
        }
        let mut deadline = self
            .lifecycle
            .next_deadline()
            .into_iter()
            .chain(self.bridge.next_deadline())
            .min();
        if let Some(hud) = self.hud.as_ref()
            && let Some(hud_deadline) = hud.next_deadline()
        {
            deadline = Some(deadline.map_or(hud_deadline, |value| value.min(hud_deadline)));
        }
        if self.lifecycle.is_connected() {
            deadline = Some(deadline.map_or(self.next_tick, |value| value.min(self.next_tick)));
        }
        match deadline {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
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
            WindowEvent::Focused(false) => self.camera.end_drag(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                    self.camera.clamp(size.width, size.height);
                }
                if let Some(hud) = self.hud.as_mut()
                    && let Some(renderer) = self.renderer.as_ref()
                    && let Err(error) = hud.resize(&renderer.window)
                {
                    tracing::error!(event = "hud_resize_failed", %error);
                    event_loop.exit();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(hud), Some(renderer)) = (self.hud.as_mut(), self.renderer.as_ref())
                    && let Err(error) = hud.resize(&renderer.window)
                {
                    tracing::error!(event = "hud_resize_failed", %error);
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                if let Some(renderer) = self.renderer.as_ref()
                    && self.camera.drag_to(
                        Vec2::new(position.x as f32, position.y as f32),
                        renderer.config.width,
                        renderer.config.height,
                    )
                {
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Middle,
                ..
            } => {
                if state == ElementState::Pressed && self.gameplay_available() {
                    if let Some(cursor) = self.cursor_position {
                        self.camera
                            .begin_drag(Vec2::new(cursor.x as f32, cursor.y as f32));
                    }
                } else {
                    self.camera.end_drag();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if !self.gameplay_available() {
                    return;
                }
                if let (Some(cursor), Some(renderer)) =
                    (self.cursor_position, self.renderer.as_ref())
                {
                    let destination = self.camera.screen_to_world(
                        Vec2::new(cursor.x as f32, cursor.y as f32),
                        renderer.config.width,
                        renderer.config.height,
                    );
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
                if !self.gameplay_available() {
                    return;
                }
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
                        self.presentation.marker(self.simulation.world()),
                        &self.combat_presentation,
                        self.camera,
                        Instant::now(),
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
    #[cfg(target_os = "linux")]
    initialize_linux_webview();
    let _logging =
        spacegame2d_logging::init("spacegame2d", "info").expect("failed to initialize logging");
    #[cfg(target_os = "linux")]
    tracing::info!(
        event = "display_backend_selected",
        display_backend = "x11",
        wayland_session = std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value == "wayland"),
        display_present = std::env::var_os("DISPLAY").is_some(),
    );
    tracing::info!(event = "client_starting", "spacegame2d starting");
    #[cfg(target_os = "linux")]
    use winit::platform::x11::EventLoopBuilderExtX11;
    #[cfg(target_os = "linux")]
    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .with_x11()
        .build()?;
    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(event_loop.create_proxy());
    event_loop.run_app(&mut app)
}

#[cfg(target_os = "linux")]
fn initialize_linux_webview() {
    // SAFETY: this runs before logging, GTK, Wry, Winit, or any worker thread is initialized.
    unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    gtk::init().expect("failed to initialize GTK for the required HUD WebView");
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_protocol::{CommandRejected, CommandRejectionReason};
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
        let fleet_size = world.config().fleet_size() as usize;
        assert_eq!(data.len(), world.units.len());
        assert!(
            data[..fleet_size]
                .iter()
                .all(|(_, color)| *color == PLAYER_ONE_COLOR)
        );
        assert!(
            data[fleet_size..]
                .iter()
                .all(|(_, color)| *color == PLAYER_TWO_COLOR)
        );
    }

    #[test]
    fn destination_presentation_tracks_confirmation_rejection_and_reset() {
        let now = Instant::now();
        let world = spacegame2d_simulation::World::demo();
        let mut presentation = DestinationPresentation::default();
        presentation.begin(4, Vec2::new(2.0, 3.0));
        assert_eq!(
            presentation.marker(&world).unwrap().status,
            MarkerStatus::Pending
        );
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
            presentation.marker(&world).unwrap().status,
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
            presentation.marker(&world).unwrap().status,
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
        assert!(presentation.marker(&world).is_none());
        assert!(presentation.rejection_text(now).is_none());
    }

    #[test]
    fn destination_markers_project_pending_and_confirmed_points() {
        let world = spacegame2d_simulation::World::demo();
        let mut presentation = DestinationPresentation::default();
        let pending = world.structures()[0].position();
        presentation.begin(4, pending);
        assert_eq!(
            presentation.marker(&world),
            Some(DestinationMarker {
                position: world.project_destination(pending),
                status: MarkerStatus::Pending,
            })
        );

        presentation.authoritative(
            1,
            &spacegame2d_protocol::AuthoritativeCommand {
                execute_tick: Tick::new(2),
                player_slot: 1,
                sequence: 4,
                command: spacegame2d_protocol::CommandData::SetDestination {
                    destination: [
                        world.structures()[1].position().x.to_bits(),
                        0.0f32.to_bits(),
                    ],
                },
            },
        );
        assert_eq!(
            presentation.marker(&world),
            Some(DestinationMarker {
                position: world.project_destination(world.structures()[1].position()),
                status: MarkerStatus::Confirmed,
            })
        );
    }

    #[test]
    fn screen_to_world_preserves_outside_arena_coordinates() {
        let point =
            Camera::new(WORLD_RADIUS_M).screen_to_world(Vec2::new(-1000.0, 5000.0), 800, 600);
        assert!(point.length() > WORLD_RADIUS_M);
        assert!(point.x < 0.0 && point.y < 0.0);
    }

    #[test]
    fn screen_to_world_keeps_inside_click_inside_circular_arena() {
        let camera = Camera::new(WORLD_RADIUS_M);
        let point = camera.screen_to_world(Vec2::new(400.0, 300.0), 800, 600);
        assert_eq!(point, Vec2::ZERO);

        let point = camera.screen_to_world(Vec2::new(600.0, 300.0), 800, 600);
        assert!(point.length() < WORLD_RADIUS_M);
    }

    #[test]
    fn scene_uniform_uses_fixed_world_scale() {
        let u = scene_uniform(
            1000,
            1000,
            Camera::new(WORLD_RADIUS_M),
            &ShipState::default(),
            None,
            PLAYER_ONE_COLOR,
        );
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
        let vertices = ring_vertices(WORLD_RADIUS_M);
        assert_eq!(vertices.len(), 128 * 6);
    }
}
