use crate::command::{CommandScheduler, UnitId, World, command_from_data, valid_authoritative};
use crate::flight_control::NeighborObservation;
use glam::Vec2;
use spacegame2d_protocol::AuthoritativeCommand;

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
pub struct FlightInput {
    /// Apply forward thrust along the current heading.
    pub thrust: bool,
    /// Apply counterclockwise (left) angular thrust.
    pub turn_left: bool,
    /// Apply clockwise (right) angular thrust.
    pub turn_right: bool,
}

/// Command issued to a [`Simulation`], distinct from per-tick [`FlightInput`].
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

/// Fixed-timestep driver for the autopilot drone world.
pub struct Simulation {
    tick: u64,
    world_radius: f32,
    pub world: World,
    pub commands: CommandScheduler,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            tick: 0,
            world_radius: WORLD_RADIUS_M,
            world: World::demo(),
            commands: CommandScheduler::default(),
        }
    }
}

impl Simulation {
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Align a mirror simulation with the authoritative server clock.
    pub fn set_tick(&mut self, tick: u64) {
        self.tick = tick;
    }

    pub fn with_world_radius(world_radius: f32) -> Self {
        Self {
            world_radius,
            ..Self::default()
        }
    }

    pub fn world_radius(&self) -> f32 {
        self.world_radius
    }
    pub fn world(&self) -> &World {
        &self.world
    }
    pub fn schedule_authoritative(&mut self, cmd: &AuthoritativeCommand) -> bool {
        if !valid_authoritative(&self.world, cmd) {
            return false;
        }
        let Some(command) = command_from_data(&cmd.command) else {
            return false;
        };
        self.commands.schedule(command);
        true
    }

    /// Advance one deterministic tick. The client demo's drones are stepped by
    /// [`crate::fleet::Fleet`]; this simulation contains no player entity.
    pub fn step(&mut self) -> Vec<SimulationEvent> {
        self.commands.execute_pending(&mut self.world);
        let observations: Vec<NeighborObservation> = self
            .world
            .units
            .iter()
            .map(|u| NeighborObservation {
                position: u.state.position,
                velocity: u.state.velocity,
            })
            .collect();
        for (i, unit) in self.world.units.iter_mut().enumerate() {
            let neighbors = observations
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, n)| *n)
                .collect::<Vec<_>>();
            let input = unit.autopilot.controls_for_tick(&unit.state, &neighbors);
            step_ship(&mut unit.state, input);
        }
        let mut events = Vec::new();
        self.world.units.retain(|u| {
            if is_out_of_bounds(u.state.position, self.world_radius) {
                events.push(SimulationEvent::BoundaryCrossed {
                    tick: self.tick,
                    unit_id: u.id,
                    position: u.state.position,
                });
                false
            } else {
                true
            }
        });
        events.sort_by_key(|event| match event {
            SimulationEvent::BoundaryCrossed { unit_id, .. } => *unit_id,
        });
        self.tick = self.tick.saturating_add(1);
        events
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationEvent {
    BoundaryCrossed {
        tick: u64,
        unit_id: UnitId,
        position: Vec2,
    },
}

/// Returns `true` when `position` lies strictly outside a circle of
/// `world_radius` centered at the origin.
pub fn is_out_of_bounds(position: Vec2, world_radius: f32) -> bool {
    position.length() > world_radius
}

/// Integrate one tick of ship physics from autopilot flight input into `ship`.
pub fn step_ship(ship: &mut ShipState, input: FlightInput) {
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
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starts_without_player_ship() {
        let sim = Simulation::default();
        assert_eq!(sim.tick(), 0);
    }
    #[test]
    fn step_advances_tick_without_player_input() {
        let mut sim = Simulation::default();
        assert!(sim.step().is_empty());
        assert_eq!(sim.tick(), 1);
    }

    #[test]
    fn step_emits_boundary_crossed_and_removes_unit() {
        let mut sim = Simulation::with_world_radius(1.0);
        sim.world.units.truncate(1);
        sim.world.units[0].state.position = Vec2::new(1.1, 0.0);
        let unit_id = sim.world.units[0].id;

        let events = sim.step();

        assert_eq!(
            events,
            vec![SimulationEvent::BoundaryCrossed {
                tick: 0,
                unit_id,
                position: Vec2::new(1.1, 0.0),
            }]
        );
        assert!(sim.world.units.is_empty());
        assert_eq!(sim.tick(), 1);
    }

    #[test]
    fn boundary_events_are_sorted_by_unit_id() {
        let mut sim = Simulation::with_world_radius(1.0);
        sim.world.units.truncate(2);
        sim.world.units.swap(0, 1);
        for unit in &mut sim.world.units {
            unit.state.position = Vec2::new(1.1, 0.0);
        }
        let mut expected = sim
            .world
            .units
            .iter()
            .map(|unit| unit.id)
            .collect::<Vec<_>>();
        expected.sort_unstable();

        let events = sim.step();
        let actual = events
            .iter()
            .map(|event| match event {
                SimulationEvent::BoundaryCrossed { unit_id, .. } => *unit_id,
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }
    #[test]
    fn physics_input_remains_deterministic_for_autopilot() {
        let mut a = ShipState::default();
        let mut b = ShipState::default();
        let input = FlightInput {
            thrust: true,
            turn_left: true,
            turn_right: false,
        };
        step_ship(&mut a, input);
        step_ship(&mut b, input);
        assert_eq!(a, b);
    }
}
