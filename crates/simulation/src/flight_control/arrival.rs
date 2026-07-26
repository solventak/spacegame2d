//! Velocity-arrival flight controller.
//!
//! [`ArrivalController`] computes a desired velocity from the position error to
//! the destination, derives a desired acceleration from the velocity error,
//! and turns/thrusts the ship to align its facing with that acceleration. It
//! settles (emits no input) once inside the arrival radius and below the
//! stopped speed.

use crate::flight_control::{
    FlightController, FlightObservation, NeighborRelationship, forward, max_angular_speed,
    thrust_acceleration,
};
use crate::simulation::FlightInput;
use glam::Vec2;

const APPROACH_EPSILON: f32 = 1.0e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClosestApproach {
    pub time_seconds: f32,
    pub predicted_offset: Vec2,
    pub distance_meters: f32,
}

pub(crate) fn closest_approach(
    relative_position: Vec2,
    relative_velocity: Vec2,
    horizon_seconds: f32,
) -> ClosestApproach {
    let horizon_seconds = horizon_seconds.max(0.0);
    let relative_speed_squared = relative_velocity.length_squared();
    let time_seconds = if relative_speed_squared > APPROACH_EPSILON {
        (-relative_position.dot(relative_velocity) / relative_speed_squared)
            .clamp(0.0, horizon_seconds)
    } else {
        0.0
    };
    let predicted_offset = relative_position + relative_velocity * time_seconds;
    ClosestApproach {
        time_seconds,
        predicted_offset,
        distance_meters: predicted_offset.length(),
    }
}

/// Tuning gains and thresholds for the [`ArrivalController`].
#[derive(Clone, Copy, Debug)]
pub struct ArrivalControllerConfig {
    /// Maximum desired speed in meters per second, clamping the position-gain
    /// term far from the destination.
    pub max_arrival_speed: f32,
    /// Proportional gain on position error (distance → desired speed).
    pub position_gain: f32,
    /// Proportional gain on velocity error (velocity error → desired
    /// acceleration).
    pub velocity_gain: f32,
    /// Proportional gain on heading error (angle → desired angular velocity).
    pub turn_gain: f32,
    /// Gain applied to the current angular velocity when computing the angular
    /// error term.
    pub angular_velocity_gain: f32,
    /// Half-angle (in radians) within which the ship may apply forward thrust.
    pub thrust_angle_radians: f32,
    /// Angular error deadband in radians; smaller magnitudes produce no turn
    /// input.
    pub angular_deadband: f32,
    /// Distance from the destination at which the controller settles.
    pub arrival_radius_meters: f32,
    /// Prediction horizon for neighbor closest-approach guidance.
    pub prediction_horizon_seconds: f32,
    /// Desired hull-to-hull clearance for predicted friendly approaches.
    pub comfort_clearance_meters: f32,
    /// Desired hull-to-hull clearance for predicted opposing approaches.
    pub opposing_comfort_clearance_meters: f32,
    /// Desired hull-to-structure clearance for predicted static approaches.
    pub structure_comfort_clearance_meters: f32,
    /// Strength of each neighbor guidance contribution.
    pub avoidance_strength: f32,
    /// Maximum total neighbor guidance magnitude.
    pub max_avoidance_acceleration: f32,
    /// Maximum static-structure guidance magnitude, applied independently of
    /// movable-neighbor guidance.
    pub max_structure_avoidance_acceleration: f32,
    pub opposing_avoidance_strength: f32,
    pub structure_avoidance_strength: f32,
    pub opposing_speed_squared_scale: f32,
    pub structure_speed_squared_scale: f32,
}
impl Default for ArrivalControllerConfig {
    fn default() -> Self {
        Self {
            max_arrival_speed: 12.0,
            position_gain: 2.0,
            velocity_gain: 1.8,
            turn_gain: 2.0,
            angular_velocity_gain: 1.0,
            thrust_angle_radians: 20.0_f32.to_radians(),
            angular_deadband: 0.08,
            arrival_radius_meters: 0.6,
            prediction_horizon_seconds: 0.75,
            comfort_clearance_meters: 2.,
            opposing_comfort_clearance_meters: 4.0,
            structure_comfort_clearance_meters: 6.0,
            avoidance_strength: 8.0,
            max_avoidance_acceleration: 12.0,
            max_structure_avoidance_acceleration: 24.0,
            opposing_avoidance_strength: 24.0,
            structure_avoidance_strength: 48.0,
            opposing_speed_squared_scale: 1.5,
            structure_speed_squared_scale: 2.0,
        }
    }
}
/// Velocity-arrival controller. Stateless aside from its [`config`](ArrivalControllerConfig) field.
///
/// See the [module docs](crate::flight_control::arrival) for the control law.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArrivalController {
    /// Tuning parameters.
    pub config: ArrivalControllerConfig,
}
impl FlightController for ArrivalController {
    fn name(&self) -> &'static str {
        "velocity-arrival"
    }
    fn desired_input(&self, o: FlightObservation) -> FlightInput {
        let offset = o.destination - o.position;
        let distance = offset.length();
        let avoidance = self.avoidance_acceleration(o);
        if distance <= self.config.arrival_radius_meters
            && o.velocity.length() <= 0.08
            && avoidance.length_squared() <= APPROACH_EPSILON
        {
            return FlightInput::default();
        }
        let target_direction = offset.normalize_or_zero();
        let desired_speed =
            (distance * self.config.position_gain).min(self.config.max_arrival_speed);
        let desired_velocity = target_direction * desired_speed;
        let velocity_error = desired_velocity - o.velocity;
        let desired_acceleration = velocity_error * self.config.velocity_gain + avoidance;
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
        FlightInput {
            thrust,
            turn_left,
            turn_right,
        }
    }
}
impl ArrivalController {
    fn avoidance_acceleration(&self, observation: FlightObservation) -> Vec2 {
        if observation.neighbors.is_empty()
            || (self.config.avoidance_strength <= 0.0
                && self.config.opposing_avoidance_strength <= 0.0
                && self.config.structure_avoidance_strength <= 0.0)
        {
            return Vec2::ZERO;
        }

        let mut friendly_avoidance = Vec2::ZERO;
        let mut opposing_avoidance = Vec2::ZERO;
        let mut structure_avoidance = Vec2::ZERO;
        let speed_fraction = (observation.velocity.length()
            / crate::simulation::MAX_SPEED_METERS_PER_SECOND)
            .clamp(0.0, 1.0);
        let opposing_strength = self.config.opposing_avoidance_strength
            * (1.0 + self.config.opposing_speed_squared_scale * speed_fraction * speed_fraction);
        let structure_strength = self.config.structure_avoidance_strength
            * (1.0 + self.config.structure_speed_squared_scale * speed_fraction * speed_fraction);

        for neighbor in observation.neighbors {
            let relative_position = neighbor.position - observation.position;
            let relative_velocity = neighbor.velocity - observation.velocity;
            let closest = closest_approach(
                relative_position,
                relative_velocity,
                self.config.prediction_horizon_seconds,
            );
            let desired_clearance = match neighbor.relationship {
                NeighborRelationship::Friendly => self.config.comfort_clearance_meters,
                NeighborRelationship::Opposing => self.config.opposing_comfort_clearance_meters,
                NeighborRelationship::StaticStructure => {
                    self.config.structure_comfort_clearance_meters
                }
            };
            let own_hitbox = observation.hitbox.positioned_at(observation.position);
            let neighbor_hitbox = neighbor.hitbox.positioned_at(neighbor.position);
            let contact_distance = own_hitbox.contact_distance_to(neighbor_hitbox);
            let predicted_clearance = closest.distance_meters - contact_distance;
            let activation_distance = contact_distance + desired_clearance;
            if predicted_clearance >= desired_clearance || activation_distance <= 0.0 {
                continue;
            }

            let away = if closest.predicted_offset.length_squared() > APPROACH_EPSILON {
                -closest.predicted_offset.normalize()
            } else if relative_position.length_squared() > APPROACH_EPSILON {
                -relative_position.normalize()
            } else if relative_velocity.length_squared() > APPROACH_EPSILON {
                Vec2::new(-relative_velocity.y, relative_velocity.x).normalize()
            } else {
                Vec2::ZERO
            };
            let penetration =
                ((desired_clearance - predicted_clearance) / activation_distance).clamp(0.0, 1.0);
            let contribution = away * penetration * penetration * 0.5;
            match neighbor.relationship {
                NeighborRelationship::Friendly => {
                    friendly_avoidance += contribution * self.config.avoidance_strength;
                }
                NeighborRelationship::Opposing => {
                    opposing_avoidance += contribution * opposing_strength;
                }
                NeighborRelationship::StaticStructure => {
                    structure_avoidance += contribution * structure_strength;
                }
            }
        }

        let movable_avoidance = (friendly_avoidance + opposing_avoidance)
            .clamp_length_max(self.config.max_avoidance_acceleration.max(0.0));
        movable_avoidance
            + structure_avoidance
                .clamp_length_max(self.config.max_structure_avoidance_acceleration.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_control::{AvoidanceEntityId, NeighborObservation, NeighborRelationship};
    use crate::hitbox::Hitbox;
    use crate::simulation::ShipState;
    use glam::Vec2;
    #[test]
    fn predicts_converging_closest_approach() {
        let result = closest_approach(Vec2::X * 4.0, Vec2::X * -2.0, 5.0);
        assert_eq!(result.time_seconds, 2.0);
        assert_eq!(result.distance_meters, 0.0);
    }

    #[test]
    fn clamps_closest_approach_to_horizon() {
        let result = closest_approach(Vec2::X * 4.0, Vec2::X * -1.0, 0.5);
        assert_eq!(result.time_seconds, 0.5);
        assert_eq!(result.distance_meters, 3.5);
    }

    #[test]
    fn diverging_and_stationary_neighbors_use_current_distance() {
        let diverging = closest_approach(Vec2::X * 2.0, Vec2::X, 1.0);
        assert_eq!(diverging.time_seconds, 0.0);
        assert_eq!(diverging.distance_meters, 2.0);

        let stationary = closest_approach(Vec2::new(3.0, 4.0), Vec2::ZERO, 1.0);
        assert_eq!(stationary.time_seconds, 0.0);
        assert_eq!(stationary.distance_meters, 5.0);
    }

    #[test]
    fn empty_neighbors_produce_no_avoidance() {
        let controller = ArrivalController::default();
        let observation = FlightObservation::from_ship(&ShipState::default(), Vec2::Y, &[]);
        assert_eq!(controller.avoidance_acceleration(observation), Vec2::ZERO);
    }

    #[test]
    fn neighbors_outside_combined_hulls_and_comfort_clearance_produce_no_avoidance() {
        let controller = ArrivalController::default();
        let neighbor = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 3.3,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let neighbors = [neighbor];
        let observation = FlightObservation::from_ship(&ShipState::default(), Vec2::Y, &neighbors);
        assert_eq!(controller.avoidance_acceleration(observation), Vec2::ZERO);
    }

    #[test]
    fn equal_center_distances_produce_more_avoidance_for_larger_hulls() {
        let controller = ArrivalController::default();
        let small_neighbor = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 2.5,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::circle(0.2).unwrap(),
            relationship: NeighborRelationship::Friendly,
        };
        let large_neighbor = NeighborObservation {
            hitbox: Hitbox::circle(1.0).unwrap(),
            ..small_neighbor
        };
        let own_hitbox = Hitbox::circle(0.6).unwrap();
        let small = controller.avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
            &ShipState::default(),
            own_hitbox,
            Vec2::Y,
            &[small_neighbor],
        ));
        let large = controller.avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
            &ShipState::default(),
            own_hitbox,
            Vec2::Y,
            &[large_neighbor],
        ));

        assert!(large.length() > small.length());
    }

    #[test]
    fn zero_clearance_allows_tangency_but_responds_to_overlap() {
        let controller = ArrivalController {
            config: ArrivalControllerConfig {
                comfort_clearance_meters: 0.0,
                opposing_comfort_clearance_meters: 0.0,
                ..ArrivalControllerConfig::default()
            },
        };
        let hitbox = Hitbox::circle(0.6).unwrap();
        let tangent = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 1.2,
            velocity: Vec2::ZERO,
            hitbox,
            relationship: NeighborRelationship::Friendly,
        };
        let overlapping = NeighborObservation {
            position: Vec2::X,
            ..tangent
        };

        assert_eq!(
            controller.avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
                &ShipState::default(),
                hitbox,
                Vec2::Y,
                &[tangent],
            )),
            Vec2::ZERO
        );
        assert!(
            controller
                .avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
                    &ShipState::default(),
                    hitbox,
                    Vec2::Y,
                    &[overlapping],
                ))
                .length()
                > 0.0
        );
    }

    #[test]
    fn opposing_neighbors_use_the_larger_comfort_clearance() {
        let controller = ArrivalController {
            config: ArrivalControllerConfig {
                comfort_clearance_meters: 1.2,
                opposing_comfort_clearance_meters: 2.0,
                ..ArrivalControllerConfig::default()
            },
        };
        let friendly = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 2.8,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let opposing = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 2.8,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Opposing,
        };
        let friendly_result = controller.avoidance_acceleration(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y,
            &[friendly],
        ));
        let opposing_result = controller.avoidance_acceleration(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y,
            &[opposing],
        ));
        assert_eq!(friendly_result, Vec2::ZERO);
        assert!(opposing_result.length() > 0.0);
    }

    #[test]
    fn static_structures_have_a_stronger_independent_avoidance_profile() {
        let controller = ArrivalController::default();
        let opposing = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 3.0,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::circle(3.85).unwrap(),
            relationship: NeighborRelationship::Opposing,
        };
        let structure = NeighborObservation {
            entity_id: AvoidanceEntityId::StaticStructure(crate::StaticStructureId(1)),
            relationship: NeighborRelationship::StaticStructure,
            ..opposing
        };
        let ship = ShipState {
            velocity: Vec2::X * 4.0,
            ..ShipState::default()
        };
        let opposing_result =
            controller.avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
                &ship,
                Hitbox::default_ship(),
                Vec2::Y,
                &[opposing],
            ));
        let structure_result =
            controller.avoidance_acceleration(FlightObservation::from_ship_with_hitbox(
                &ship,
                Hitbox::default_ship(),
                Vec2::Y,
                &[structure],
            ));

        assert!(structure_result.length() > opposing_result.length());
        assert!(
            structure_result.length() <= controller.config.max_structure_avoidance_acceleration
        );
    }

    #[test]
    fn closer_neighbors_produce_stronger_avoidance() {
        let controller = ArrivalController::default();
        let near = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 0.3,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let marginal = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let near_neighbors = [near];
        let marginal_neighbors = [marginal];
        let near_observation =
            FlightObservation::from_ship(&ShipState::default(), Vec2::Y, &near_neighbors);
        let marginal_observation =
            FlightObservation::from_ship(&ShipState::default(), Vec2::Y, &marginal_neighbors);
        assert!(
            controller.avoidance_acceleration(near_observation).length()
                > controller
                    .avoidance_acceleration(marginal_observation)
                    .length()
        );
    }

    #[test]
    fn total_avoidance_is_bounded() {
        let controller = ArrivalController::default();
        let neighbors = vec![
            NeighborObservation {
                entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
                position: Vec2::X * 0.2,
                velocity: Vec2::ZERO,
                hitbox: Hitbox::default_ship(),
                relationship: NeighborRelationship::Friendly,
            };
            64
        ];
        let observation = FlightObservation::from_ship(&ShipState::default(), Vec2::Y, &neighbors);
        assert!(
            controller.avoidance_acceleration(observation).length()
                <= controller.config.max_avoidance_acceleration
        );
    }

    #[test]
    fn opposing_neighbor_has_stronger_base_response() {
        let config = ArrivalControllerConfig {
            max_avoidance_acceleration: 100.0,
            ..ArrivalControllerConfig::default()
        };
        let controller = ArrivalController { config };
        let friendly = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 0.3,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let opposing = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 0.3,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Opposing,
        };
        let friendly_neighbors = [friendly];
        let opposing_neighbors = [opposing];
        let friendly_result = controller.avoidance_acceleration(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y,
            &friendly_neighbors,
        ));
        let opposing_result = controller.avoidance_acceleration(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y,
            &opposing_neighbors,
        ));
        assert!(opposing_result.length() > friendly_result.length());
    }

    #[test]
    fn opposing_speed_boost_is_quadratic_and_normalized() {
        let config = ArrivalControllerConfig {
            max_avoidance_acceleration: 100.0,
            ..ArrivalControllerConfig::default()
        };
        let controller = ArrivalController { config };
        let neighbor = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 0.3,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Opposing,
        };
        let response = |speed: f32| {
            let ship = ShipState {
                velocity: Vec2::Y * speed,
                ..Default::default()
            };
            let neighbors = [neighbor];
            controller
                .avoidance_acceleration(FlightObservation::from_ship(&ship, Vec2::Y, &neighbors))
                .length()
        };
        let base = response(0.0);
        let half = response(crate::simulation::MAX_SPEED_METERS_PER_SECOND * 0.5);
        let max = response(crate::simulation::MAX_SPEED_METERS_PER_SECOND);
        assert!(((half - base) / (max - base) - 0.25).abs() < 1.0e-5);
    }

    #[test]
    fn nearby_neighbor_changes_requested_steering() {
        let controller = ArrivalController::default();
        let neighbor = NeighborObservation {
            entity_id: AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X * 0.5,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let input = controller.desired_input(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::Y * 10.0,
            &[neighbor],
        ));
        assert!(input.turn_left || input.turn_right);
    }

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
            FlightInput {
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
    fn settles_at_destination() {
        let c = ArrivalController::default();
        let i = c.desired_input(FlightObservation::from_ship(
            &ShipState::default(),
            Vec2::ZERO,
            &[],
        ));
        assert_eq!(i, FlightInput::default());
    }
}
