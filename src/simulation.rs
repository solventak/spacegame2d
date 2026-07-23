use glam::Vec2;

pub const SIMULATION_HZ: u32 = 60;
pub const FIXED_DT_SECONDS: f32 = 1.0 / SIMULATION_HZ as f32;
pub const SHIP_MASS_KG: f32 = 1.0;
pub const FORWARD_THRUST_NEWTONS: f32 = 8.0;
pub const MAX_SPEED_METERS_PER_SECOND: f32 = 8.0;
pub const LINEAR_DAMPING_PER_SECOND: f32 = 0.8;
pub const MOMENT_OF_INERTIA_KG_M2: f32 = 0.25;
pub const ANGULAR_THRUST_NEWTON_METERS: f32 = 2.0;
pub const MAX_ANGULAR_SPEED_RADIANS_PER_SECOND: f32 = 3.0;
pub const ANGULAR_DAMPING_PER_SECOND: f32 = 2.5;
const VELOCITY_EPSILON: f32 = 0.0001;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipInput {
    pub thrust: bool,
    pub turn_left: bool,
    pub turn_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationCommand {
    ResetShip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading_radians: f32,
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Simulation {
    tick: u64,
    ship: ShipState,
}

impl Simulation {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn ship(&self) -> &ShipState {
        &self.ship
    }
    pub fn apply_command(&mut self, command: SimulationCommand) {
        if matches!(command, SimulationCommand::ResetShip) {
            self.ship = ShipState::default();
        }
    }

    pub fn step(&mut self, input: ShipInput) {
        step_ship(&mut self.ship, input);
        self.tick = self.tick.saturating_add(1);
    }
}

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
        assert_eq!(*sim.ship(), ShipState::default());
    }
    #[test]
    fn empty_step_advances_tick_without_motion() {
        let mut sim = Simulation::default();
        sim.step(ShipInput::default());
        assert_eq!(sim.tick(), 1);
        assert_eq!(*sim.ship(), ShipState::default());
    }
    #[test]
    fn reset_restores_ship_without_rewinding_tick() {
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
        assert_eq!(*sim.ship(), ShipState::default());
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
        assert!(sim.ship().velocity.y > 0.0);
        assert!(sim.ship().position.y > 0.0);
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
        let speed = sim.ship().velocity.length();
        let sim = run(sim, ShipInput::default(), 1);
        assert!(sim.ship().velocity.length() < speed);
        assert!(sim.ship().velocity.length() > 0.0);
    }
    #[test]
    fn total_velocity_is_capped() {
        let sim = run(
            Simulation::default(),
            ShipInput {
                thrust: true,
                ..Default::default()
            },
            500,
        );
        assert!(sim.ship().velocity.length() <= MAX_SPEED_METERS_PER_SECOND + 0.0001);
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
        assert!(sim.ship().angular_velocity_radians_per_second > 0.0);
        assert!(sim.ship().heading_radians > 0.0);
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
        assert!(sim.ship().heading_radians < 0.0);
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
        assert_eq!(sim.ship().angular_velocity_radians_per_second, 0.0);
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
            sim.ship().angular_velocity_radians_per_second <= MAX_ANGULAR_SPEED_RADIANS_PER_SECOND
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
        assert!(sim.ship().position.x.abs() > 0.01);
        assert!(sim.ship().position.y > 0.0);
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
}
