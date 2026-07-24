mod geometry;
mod input;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use input::{ControlKey, InputController};
use spacegame2d_simulation::{
    autopilot::{Autopilot, AutopilotConfig},
    fleet::Fleet,
    flight_control::ArrivalController,
    simulation::{SIMULATION_HZ, ShipInput, ShipState, Simulation},
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

const VIEW_HEIGHT_METERS: f32 = 40.0;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / SIMULATION_HZ as u64);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    viewport: [f32; 4],
    ship: [f32; 4],
    marker: [f32; 4],
}

fn scene_uniform(width: u32, height: u32, ship: &ShipState, marker: Option<Vec2>) -> SceneUniform {
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
    scene_buffers: Vec<wgpu::Buffer>,
    scene_bind_groups: Vec<wgpu::BindGroup>,
}

impl Renderer {
    async fn new(
        window: Arc<Window>,
        drones: &[spacegame2d_simulation::fleet::Unit],
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
        let scene_uniforms = std::iter::once(ShipState::default())
            .chain(drones.iter().map(|u| u.state))
            .map(|ship| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("scene uniform"),
                    contents: bytemuck::bytes_of(&scene_uniform(
                        config.width,
                        config.height,
                        &ship,
                        None,
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
        drones: &[spacegame2d_simulation::fleet::Unit],
        ship: Option<&ShipState>,
        marker: Option<Vec2>,
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
            for (index, unit) in drones.iter().enumerate() {
                self.queue.write_buffer(
                    &self.scene_buffers[index + 1],
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        &unit.state,
                        None,
                    )),
                );
                pass.set_bind_group(0, &self.scene_bind_groups[index + 1], &[]);
                pass.draw(0..24, 0..1);
            }
            if let Some(ship) = ship {
                self.queue.write_buffer(
                    &self.scene_buffers[0],
                    0,
                    bytemuck::bytes_of(&scene_uniform(
                        self.config.width,
                        self.config.height,
                        ship,
                        marker,
                    )),
                );
                pass.set_bind_group(0, &self.scene_bind_groups[0], &[]);
                pass.draw(0..30, 0..1);
            }
            self.queue.write_buffer(
                &self.ring_scene_buffer,
                0,
                bytemuck::bytes_of(&scene_uniform(
                    self.config.width,
                    self.config.height,
                    &ShipState::default(),
                    None,
                )),
            );
            pass.set_vertex_buffer(0, self.ring_vertex_buffer.slice(..));
            pass.set_bind_group(0, &self.ring_bind_group, &[]);
            pass.draw(0..self.ring_vertex_count, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}

struct App {
    renderer: Option<Renderer>,
    simulation: Simulation,
    drones: Fleet,
    input: InputController,
    autopilot: Autopilot,
    pending_destination: Option<Vec2>,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    next_tick: Instant,
}
impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            simulation: Simulation::default(),
            drones: Fleet::new(),
            input: InputController::default(),
            autopilot: Autopilot::new(
                Box::new(ArrivalController::default()),
                AutopilotConfig::default(),
            ),
            pending_destination: None,
            cursor_position: None,
            next_tick: Instant::now(),
        }
    }
}

fn map_key(key: PhysicalKey) -> Option<ControlKey> {
    match key {
        PhysicalKey::Code(KeyCode::KeyW) => Some(ControlKey::Thrust),
        PhysicalKey::Code(KeyCode::KeyA) => Some(ControlKey::TurnLeft),
        PhysicalKey::Code(KeyCode::KeyD) => Some(ControlKey::TurnRight),
        PhysicalKey::Code(KeyCode::KeyR) => Some(ControlKey::Reset),
        _ => None,
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
        let window = match event_loop
            .create_window(Window::default_attributes().with_title("Spacegame 2D"))
        {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        match pollster::block_on(Renderer::new(window.clone(), self.drones.units())) {
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
        let now = Instant::now();
        while now >= self.next_tick {
            if let Some(command) = self.input.take_command() {
                self.simulation.apply_command(command);
                self.autopilot.cancel_and_clear_destination();
                self.drones.reset();
                self.pending_destination = None;
            } else if let Some(destination) = self.pending_destination.take() {
                self.input.suppress_held_movement_until_release();
                self.autopilot.set_destination(destination);
                self.drones.set_destination(destination);
            }
            let controls = if self.autopilot.is_active() {
                if let Some(ship) = self.simulation.ship() {
                    self.autopilot.controls_for_tick(ship, &[])
                } else {
                    ShipInput::default()
                }
            } else {
                self.input.controls()
            };
            self.simulation.step(controls);
            self.drones.step();
            self.drones
                .cull(spacegame2d_simulation::simulation::WORLD_RADIUS_M);
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
            WindowEvent::Focused(false) => self.input.clear_for_focus_loss(),
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
                    self.pending_destination = Some(screen_to_world(
                        cursor,
                        renderer.config.width,
                        renderer.config.height,
                    ));
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                if let Some(key) = map_key(physical_key) {
                    match state {
                        ElementState::Pressed if key != ControlKey::Reset || !repeat => {
                            self.input.press(key)
                        }
                        ElementState::Released => self.input.release(key),
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render(
                        self.drones.units(),
                        self.simulation.ship(),
                        self.autopilot.destination(),
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_uniform_uses_fixed_world_scale() {
        let u = scene_uniform(1000, 1000, &ShipState::default(), None);
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
}
