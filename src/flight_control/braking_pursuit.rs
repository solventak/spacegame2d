use super::{FlightController, FlightObservation, forward, max_angular_speed, thrust_acceleration};
use crate::simulation::ShipInput;

#[derive(Clone, Copy, Debug)]
pub struct BrakingPursuitConfig {
    pub braking_safety_factor: f32,
    pub braking_buffer_meters: f32,
    pub approach_thrust_angle_radians: f32,
    pub braking_thrust_angle_radians: f32,
    pub turn_gain: f32,
    pub angular_velocity_deadband: f32,
}
impl Default for BrakingPursuitConfig {
    fn default() -> Self {
        Self {
            braking_safety_factor: 1.15,
            braking_buffer_meters: 0.20,
            approach_thrust_angle_radians: 65.0_f32.to_radians(),
            braking_thrust_angle_radians: 30.0_f32.to_radians(),
            turn_gain: 2.5,
            angular_velocity_deadband: 0.10,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct BrakingPursuitController {
    pub config: BrakingPursuitConfig,
}
impl FlightController for BrakingPursuitController {
    fn name(&self) -> &'static str {
        "braking-pursuit"
    }
    fn desired_input(&self, o: FlightObservation) -> ShipInput {
        let to_target = o.destination - o.position;
        let distance = to_target.length();
        if distance <= 0.30 && o.velocity.length() <= 0.08 {
            return ShipInput::default();
        }
        let speed = o.velocity.length();
        let target_dir = to_target.normalize_or_zero();
        let velocity_dir = o.velocity.normalize_or_zero();
        let stopping_distance = speed * speed / (2.0 * thrust_acceleration());
        let braking = speed > 0.08
            && stopping_distance * self.config.braking_safety_factor
                + self.config.braking_buffer_meters
                >= distance;
        let desired_dir = if braking { -velocity_dir } else { target_dir };
        let facing = forward(o.heading_radians);
        let angle = facing.perp_dot(desired_dir).atan2(facing.dot(desired_dir));
        let desired_angular =
            (angle * self.config.turn_gain).clamp(-max_angular_speed(), max_angular_speed());
        let angular_error = desired_angular - o.angular_velocity_radians_per_second;
        let (turn_left, turn_right) = if angular_error > self.config.angular_velocity_deadband {
            (true, false)
        } else if angular_error < -self.config.angular_velocity_deadband {
            (false, true)
        } else {
            (false, false)
        };
        let angle_to_direction = facing.dot(desired_dir).clamp(-1.0, 1.0).acos();
        let thrust = if braking {
            angle_to_direction <= self.config.braking_thrust_angle_radians
        } else {
            angle_to_direction <= self.config.approach_thrust_angle_radians
        };
        ShipInput {
            thrust,
            turn_left,
            turn_right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    #[test]
    fn far_target_ahead_requests_thrust() {
        let c = BrakingPursuitController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &crate::simulation::ShipState::default(),
            Vec2::Y * 10.0,
        ));
        assert!(i.thrust);
        assert!(!i.turn_left && !i.turn_right);
    }
    #[test]
    fn side_target_requests_turn() {
        let c = BrakingPursuitController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &crate::simulation::ShipState::default(),
            Vec2::X * 10.0,
        ));
        assert!(i.turn_left || i.turn_right);
    }
    #[test]
    fn fast_ship_near_target_brakes_against_velocity() {
        let c = BrakingPursuitController::default();
        let s = crate::simulation::ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::Y * 4.0,
            ..Default::default()
        };
        let i = c.desired_input(FlightObservation::from_ship(&s, Vec2::Y * 1.0));
        assert!(!i.thrust);
        assert!(i.turn_left || i.turn_right);
    }
    #[test]
    fn zero_distance_is_safe() {
        let c = BrakingPursuitController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &crate::simulation::ShipState::default(),
            Vec2::ZERO,
        ));
        assert_eq!(i, ShipInput::default());
    }
}
