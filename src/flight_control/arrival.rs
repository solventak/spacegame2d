use crate::flight_control::{
    FlightController, FlightObservation, forward, max_angular_speed, thrust_acceleration,
};
use crate::simulation::ShipInput;

#[derive(Clone, Copy, Debug)]
pub struct ArrivalControllerConfig {
    pub max_arrival_speed: f32,
    pub position_gain: f32,
    pub velocity_gain: f32,
    pub turn_gain: f32,
    pub angular_velocity_gain: f32,
    pub thrust_angle_radians: f32,
    pub angular_deadband: f32,
    pub arrival_radius_meters: f32,
    #[allow(dead_code)]
    pub collision_radius_meters: f32,
    #[allow(dead_code)]
    pub collision_strength: f32,
}
impl Default for ArrivalControllerConfig {
    fn default() -> Self {
        Self {
            max_arrival_speed: 6.0,
            position_gain: 2.0,
            velocity_gain: 1.8,
            turn_gain: 2.0,
            angular_velocity_gain: 1.0,
            thrust_angle_radians: 20.0_f32.to_radians(),
            angular_deadband: 0.08,
            arrival_radius_meters: 0.30,
            collision_radius_meters: 0.9,
            collision_strength: 5.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ArrivalController {
    pub config: ArrivalControllerConfig,
}
impl FlightController for ArrivalController {
    fn name(&self) -> &'static str {
        "velocity-arrival"
    }
    fn desired_input(&self, o: FlightObservation) -> ShipInput {
        let offset = o.destination - o.position;
        let distance = offset.length();
        if distance <= self.config.arrival_radius_meters && o.velocity.length() <= 0.08 {
            return ShipInput::default();
        }
        let target_direction = offset.normalize_or_zero();
        let desired_speed =
            (distance * self.config.position_gain).min(self.config.max_arrival_speed);
        let desired_velocity = target_direction * desired_speed;
        let velocity_error = desired_velocity - o.velocity;
        let desired_acceleration = velocity_error * self.config.velocity_gain;
        let desired_direction = if desired_acceleration.length_squared() > 0.0001 {
            desired_acceleration.normalize()
        } else {
            target_direction
        };
        let facing = forward(o.heading_radians);
        let angle = facing
            .perp_dot(desired_direction)
            .atan2(facing.dot(desired_direction));
        let desired_angular_velocity =
            (angle * self.config.turn_gain).clamp(-max_angular_speed(), max_angular_speed());
        let angular_error = desired_angular_velocity
            - o.angular_velocity_radians_per_second * self.config.angular_velocity_gain;
        let (turn_left, turn_right) = if angular_error > self.config.angular_deadband {
            (true, false)
        } else if angular_error < -self.config.angular_deadband {
            (false, true)
        } else {
            (false, false)
        };
        let angle_to_direction = facing.dot(desired_direction).clamp(-1.0, 1.0).acos();
        let thrust = angle_to_direction <= self.config.thrust_angle_radians
            && desired_acceleration.dot(facing) > 0.0
            && desired_acceleration.length() > thrust_acceleration() * 0.08;
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
    use crate::simulation::ShipState;
    use glam::Vec2;
    #[test]
    fn accelerates_toward_a_far_destination() {
        let c = ArrivalController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y * 10.0,
            &[],
        ));
        assert_eq!(
            i,
            ShipInput {
                thrust: true,
                ..Default::default()
            }
        );
    }
    #[test]
    fn brakes_forward_velocity_near_destination() {
        let c = ArrivalController::default();
        let s = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::Y * 4.0,
            ..Default::default()
        };
        let i = c.desired_input(FlightObservation::from_ship(&s, Vec2::Y, &[]));
        assert!(!i.thrust);
        assert!(i.turn_left || i.turn_right);
    }
    #[test]
    fn cancels_lateral_velocity_instead_of_chasing_position_only() {
        let c = ArrivalController::default();
        let s = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::X * 3.0,
            ..Default::default()
        };
        let i = c.desired_input(FlightObservation::from_ship(&s, Vec2::Y * 4.0, &[]));
        assert!(i.turn_left || i.turn_right);
        assert!(!i.thrust);
    }
    #[test]
    fn converges_from_rest_without_orbiting() {
        let controller = ArrivalController::default();
        let destination = Vec2::new(5.0, 4.0);
        let mut simulation = crate::simulation::Simulation::default();
        for _ in 0..3600 {
            let input = controller.desired_input(FlightObservation::from_ship(
                simulation.ship().unwrap(),
                destination,
                &[],
            ));
            simulation.step(input);
        }
        let ship = simulation.ship().unwrap();
        assert!(
            ship.position.distance(destination) <= 0.5,
            "position={:?}",
            ship.position
        );
        assert!(
            ship.velocity.length() <= 0.25,
            "velocity={:?}",
            ship.velocity
        );
    }

    #[test]
    fn settles_at_destination() {
        let c = ArrivalController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::ZERO,
            &[],
        ));
        assert_eq!(i, ShipInput::default());
    }
}
