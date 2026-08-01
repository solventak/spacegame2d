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
const VELOCITY_EPSILON: f32 = 0.0001;
/// Default radius kept as a convenience for callers that render the default
/// prototype configuration. Authoritative simulation code reads the radius
/// from [`crate::config::SimulationConfig`].
pub const WORLD_RADIUS_M: f32 = crate::config::DEFAULT_WORLD_RADIUS_METERS;

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

pub(crate) fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_ship_damps_velocity_without_thrust() {
        let mut ship = ShipState {
            velocity: Vec2::new(4.0, 0.0),
            ..Default::default()
        };
        let speed = ship.velocity.length();
        step_ship(&mut ship, FlightInput::default());
        assert!(ship.velocity.length() < speed);
        assert!(ship.velocity.length() > 0.0);
    }
}
