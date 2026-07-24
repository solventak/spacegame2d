use glam::Vec2;

/// Simulation tick rate in hertz. The integrator advances in fixed
/// [`FIXED_DT_SECONDS`] steps regardless of wall-clock frame timing.
pub const SIMULATION_HZ: u32 = 60;
/// Fixed timestep derived from [`SIMULATION_HZ`], in seconds.
pub const FIXED_DT_SECONDS: f32 = 1.0 / SIMULATION_HZ as f32;
/// Ship mass in kilograms, used to convert thrust into linear acceleration.
pub const SHIP_MASS_KG: f32 = 1.0;
/// Forward thrust force in newtons applied while `thrust` is held.
pub const FORWARD_THRUST_NEWTONS: f32 = 8.0;
/// Hard cap on ship speed in meters per second.
pub const MAX_SPEED_METERS_PER_SECOND: f32 = 8.0;
/// Linear velocity damping rate per second; the ship coasts to a stop without
/// active thrust.
pub const LINEAR_DAMPING_PER_SECOND: f32 = 0.8;
/// Ship moment of inertia in kg·m², used to convert torque into angular
/// acceleration.
pub const MOMENT_OF_INERTIA_KG_M2: f32 = 0.25;
/// Angular thrust torque in newton-meters applied while turning.
pub const ANGULAR_THRUST_NEWTON_METERS: f32 = 2.0;
/// Hard cap on angular speed in radians per second.
pub const MAX_ANGULAR_SPEED_RADIANS_PER_SECOND: f32 = 3.0;
/// Angular velocity damping rate per second; rotation decays without active
/// turn input.
pub const ANGULAR_DAMPING_PER_SECOND: f32 = 2.5;
/// Radius of the circular arena in meters. A ship whose position exceeds this
/// radius is destroyed on the next tick.
pub const WORLD_RADIUS_M: f32 = 16.0;
const VELOCITY_EPSILON: f32 = 0.0001;

/// Per-tick discrete input applied to a ship by the player or an autopilot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipInput {
    /// Apply forward thrust along the current heading.
    pub thrust: bool,
    /// Apply counterclockwise (left) angular thrust.
    pub turn_left: bool,
    /// Apply clockwise (right) angular thrust.
    pub turn_right: bool,
}

/// Command issued to a [`Simulation`], distinct from per-tick [`ShipInput`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationCommand {
    #[allow(dead_code)]
    /// Respawn the ship at the origin without rewinding the tick counter.
    ResetShip,
    /// Respawn the ship and rewind the tick counter to zero.
    ResetSimulation,
}

/// Integrated kinematic state of a single ship.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipState {
    /// Position in arena-space meters, origin at the arena center.
    pub position: Vec2,
    /// Velocity in meters per second.
    pub velocity: Vec2,
    /// Heading angle in radians. `0.0` points along +Y; positive is
    /// counterclockwise.
    pub heading_radians: f32,
    /// Angular velocity in radians per second.
    pub angular_velocity_radians_per_second: f32,
}

impl Default for ShipState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading_radians: 0.0,
            angular_velocity_radians_per_second: 0.0,
        }
    }
}

/// Fixed-timestep driver for a single player-controlled ship inside a circular
/// arena.
///
/// Each call to [`Simulation::step`] advances the ship by one
/// [`FIXED_DT_SECONDS`] tick and destroys it if it leaves the arena. Use
/// [`Simulation::apply_command`] to reset.
#[derive(Clone, Debug, PartialEq)]
pub struct Simulation {
    tick: u64,
    ship: Option<ShipState>,
    world_radius: f32,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            tick: 0,
            ship: Some(ShipState::default()),
            world_radius: WORLD_RADIUS_M,
        }
    }
}

impl Simulation {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Construct a simulation with a custom world boundary radius.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_world_radius(world_radius: f32) -> Self {
        Self {
            world_radius,
            ..Self::default()
        }
    }
    /// World boundary radius for this simulation's ship.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn world_radius(&self) -> f32 {
        self.world_radius
    }
    pub fn ship(&self) -> Option<&ShipState> {
        self.ship.as_ref()
    }
    pub fn apply_command(&mut self, command: SimulationCommand) {
        match command {
            SimulationCommand::ResetShip => {
                self.ship = Some(ShipState::default());
            }
            SimulationCommand::ResetSimulation => {
                self.ship = Some(ShipState::default());
                self.tick = 0;
            }
        }
    }

    pub fn step(&mut self, input: ShipInput) {
        if let Some(ref mut ship) = self.ship {
            step_ship(ship, input);
            if is_out_of_bounds(ship.position, self.world_radius) {
                let pos = ship.position;
                log::info!(
                    "ship destroyed: out of bounds at ({:.1}, {:.1})",
                    pos.x,
                    pos.y
                );
                self.ship = None;
            }
        }
        self.tick = self.tick.saturating_add(1);
    }
}

/// Returns `true` when `position` lies strictly outside a circle of
/// `world_radius` centered at the origin.
pub fn is_out_of_bounds(position: Vec2, world_radius: f32) -> bool {
    position.length() > world_radius
}

/// Integrate one tick of ship physics from `input` into `ship`.
///
/// Applies angular thrust and damping, integrates heading, applies linear
/// thrust and damping, clamps to the speed caps, and finally advances the
/// position. This is the shared integrator used by both the player ship and
/// each drone in the [`Fleet`](crate::fleet::Fleet).
pub fn step_ship(ship: &mut ShipState, input: ShipInput) {
    let turn_axis = input.turn_left as i32 - input.turn_right as i32;
    if turn_axis != 0 {
        ship.angular_velocity_radians_per_second += turn_axis as f32
            * (ANGULAR_THRUST_NEWTON_METERS / MOMENT_OF_INERTIA_KG_M2)
            * FIXED_DT_SECONDS;
    } else {
        ship.angular_velocity_radians_per_second *=
            (1.0 - ANGULAR_DAMPING_PER_SECOND * FIXED_DT_SECONDS).max(0.0);
    }
    ship.angular_velocity_radians_per_second = ship.angular_velocity_radians_per_second.clamp(
        -MAX_ANGULAR_SPEED_RADIANS_PER_SECOND,
        MAX_ANGULAR_SPEED_RADIANS_PER_SECOND,
    );
    ship.heading_radians = wrap_angle(
        ship.heading_radians + ship.angular_velocity_radians_per_second * FIXED_DT_SECONDS,
    );

    if input.thrust {
        let forward = Vec2::new(-ship.heading_radians.sin(), ship.heading_radians.cos());
        ship.velocity += forward * (FORWARD_THRUST_NEWTONS / SHIP_MASS_KG) * FIXED_DT_SECONDS;
    } else {
        ship.velocity *= (1.0 - LINEAR_DAMPING_PER_SECOND * FIXED_DT_SECONDS).max(0.0);
    }
    if ship.velocity.length() > MAX_SPEED_METERS_PER_SECOND {
        ship.velocity = ship.velocity.normalize() * MAX_SPEED_METERS_PER_SECOND;
    }
    if ship.velocity.length_squared() < VELOCITY_EPSILON * VELOCITY_EPSILON {
        ship.velocity = Vec2::ZERO;
    }
    if ship.angular_velocity_radians_per_second.abs() < VELOCITY_EPSILON {
        ship.angular_velocity_radians_per_second = 0.0;
    }
    ship.position += ship.velocity * FIXED_DT_SECONDS;
}
fn wrap_angle(angle: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(two_pi) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(mut sim: Simulation, input: ShipInput, ticks: usize) -> Simulation {
        for _ in 0..ticks {
            sim.step(input);
        }
        sim
    }
    #[test]
    fn starts_stationary() {
        let sim = Simulation::default();
        assert_eq!(sim.tick(), 0);
        assert_eq!(*sim.ship().unwrap(), ShipState::default());
    }
    #[test]
    fn empty_step_advances_tick_without_motion() {
        let mut sim = Simulation::default();
        sim.step(ShipInput::default());
        assert_eq!(sim.tick(), 1);
        assert_eq!(*sim.ship().unwrap(), ShipState::default());
    }
    #[test]
    fn reset_ship_restores_ship_without_rewinding_tick() {
        let mut sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            20,
        );
        let tick = sim.tick();
        sim.apply_command(SimulationCommand::ResetShip);
        assert_eq!(sim.tick(), tick);
        assert_eq!(*sim.ship().unwrap(), ShipState::default());
    }
    #[test]
    fn reset_simulation_rewinds_tick_and_respawns_ship() {
        let mut sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            20,
        );
        sim.apply_command(SimulationCommand::ResetSimulation);
        assert_eq!(sim.tick(), 0);
        assert_eq!(*sim.ship().unwrap(), ShipState::default());
    }
    #[test]
    fn reset_after_removal_respawns_ship_and_rewinds_tick() {
        let mut sim = Simulation::default();
        sim.ship = Some(ShipState {
            position: Vec2::new(WORLD_RADIUS_M + 0.01, 0.0),
            ..Default::default()
        });
        sim.step(ShipInput::default());
        assert!(sim.ship().is_none());
        assert_eq!(sim.tick(), 1);
        sim.apply_command(SimulationCommand::ResetSimulation);
        assert_eq!(sim.tick(), 0);
        assert_eq!(*sim.ship().unwrap(), ShipState::default());
    }
    #[test]
    fn forward_thrust_accelerates_along_heading() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            1,
        );
        assert!(sim.ship().unwrap().velocity.y > 0.0);
        assert!(sim.ship().unwrap().position.y > 0.0);
    }
    #[test]
    fn release_thrust_damps_drift() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            30,
        );
        let speed = sim.ship().unwrap().velocity.length();
        let sim = run(sim, ShipInput::default(), 1);
        assert!(sim.ship().unwrap().velocity.length() < speed);
        assert!(sim.ship().unwrap().velocity.length() > 0.0);
    }
    #[test]
    fn total_velocity_is_capped() {
        // Use a very large world boundary so the ship never escapes while ramping
        // up to terminal velocity across the full 500-tick run.
        let sim = run(
            Simulation::with_world_radius(10_000.0),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            500,
        );
        assert!(sim.ship().unwrap().velocity.length() <= MAX_SPEED_METERS_PER_SECOND + 0.0001);
    }
    #[test]
    fn left_applies_counterclockwise_angular_thrust() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                turn_left: true,
                ..Default::default()
            },
            1,
        );
        assert!(sim.ship().unwrap().angular_velocity_radians_per_second > 0.0);
        assert!(sim.ship().unwrap().heading_radians > 0.0);
    }
    #[test]
    fn right_applies_clockwise_angular_thrust() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                turn_right: true,
                ..Default::default()
            },
            1,
        );
        assert!(sim.ship().unwrap().heading_radians < 0.0);
    }
    #[test]
    fn opposed_turn_inputs_cancel_torque() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                turn_left: true,
                turn_right: true,
                ..Default::default()
            },
            10,
        );
        assert_eq!(sim.ship().unwrap().angular_velocity_radians_per_second, 0.0);
    }
    #[test]
    fn angular_velocity_is_capped() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                turn_left: true,
                ..Default::default()
            },
            500,
        );
        assert!(
            sim.ship().unwrap().angular_velocity_radians_per_second
                <= MAX_ANGULAR_SPEED_RADIANS_PER_SECOND
        );
    }
    #[test]
    fn combined_input_curves_trajectory() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                turn_left: true,
                ..Default::default()
            },
            60,
        );
        assert!(sim.ship().unwrap().position.x.abs() > 0.01);
        assert!(sim.ship().unwrap().position.y > 0.0);
    }
    #[test]
    fn identical_tick_inputs_are_deterministic() {
        let inputs = [ShipInput {
            thrust: true,
            ..Default::default()
        }; 30];
        let mut a = Simulation::default();
        let mut b = Simulation::default();
        for input in inputs {
            a.step(input);
            b.step(input);
        }
        assert_eq!(a, b);
    }
    #[test]
    fn exact_boundary_is_in_bounds() {
        assert!(!is_out_of_bounds(
            Vec2::new(WORLD_RADIUS_M, 0.0),
            WORLD_RADIUS_M
        ));
        assert!(!is_out_of_bounds(
            Vec2::new(0.0, WORLD_RADIUS_M),
            WORLD_RADIUS_M
        ));
    }
    #[test]
    fn epsilon_beyond_boundary_is_out_of_bounds() {
        assert!(is_out_of_bounds(
            Vec2::new(WORLD_RADIUS_M + 0.01, 0.0),
            WORLD_RADIUS_M
        ));
    }
    #[test]
    fn ship_at_exact_boundary_survives_tick() {
        let mut sim = Simulation::default();
        sim.ship = Some(ShipState {
            position: Vec2::new(WORLD_RADIUS_M, 0.0),
            ..Default::default()
        });
        sim.step(ShipInput::default());
        assert!(sim.ship().is_some());
    }
    #[test]
    fn ship_epsilon_beyond_boundary_is_removed() {
        let mut sim = Simulation::default();
        sim.ship = Some(ShipState {
            position: Vec2::new(WORLD_RADIUS_M + 0.01, 0.0),
            ..Default::default()
        });
        sim.step(ShipInput::default());
        assert!(sim.ship().is_none());
    }

    // --- logging emission ---------------------------------------------------

    use std::io::Write;
    use std::sync::{Arc, Mutex, Once, OnceLock};

    struct CaptureSink(Arc<Mutex<Vec<u8>>>);
    impl Write for CaptureSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn capture_buffer() -> &'static Arc<Mutex<Vec<u8>>> {
        static CAPTURE: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
        CAPTURE.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
    }

    fn ensure_env_logger() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let sink = Box::new(CaptureSink(capture_buffer().clone()));
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Info)
                .target(env_logger::Target::Pipe(sink))
                .try_init()
                .ok();
        });
    }

    #[test]
    fn ship_destroyed_info_log_actually_emits() {
        ensure_env_logger();
        // Use distinctive coordinates so concurrent out-of-bounds tests logging
        // into the shared capture buffer cannot satisfy this assertion for us.
        let marker_pos = Vec2::new(WORLD_RADIUS_M + 5.0, 3.0);
        let expected = format!(
            "out of bounds at ({:.1}, {:.1})",
            marker_pos.x, marker_pos.y
        );

        {
            let mut buf = capture_buffer().lock().unwrap();
            buf.clear();
        }
        let mut sim = Simulation::default();
        sim.ship = Some(ShipState {
            position: marker_pos,
            ..Default::default()
        });
        sim.step(ShipInput::default());
        log::logger().flush();

        let recorded = String::from_utf8(capture_buffer().lock().unwrap().clone()).unwrap();
        assert!(
            recorded.contains(&expected),
            "expected info log to emit {expected:?}, got: {recorded}"
        );
    }
}
