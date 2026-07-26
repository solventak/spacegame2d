use crate::flight_control::{FlightController, FlightObservation, NeighborObservation};
use crate::hitbox::Hitbox;
use crate::simulation::{FlightInput, ShipState};
use glam::Vec2;

/// Default radius around the destination within which the autopilot considers
/// itself "arrived".
pub const DEFAULT_ARRIVAL_RADIUS_METERS: f32 = 0.6;
/// Default speed threshold below which a ship inside the arrival radius is
/// considered stopped.
pub const DEFAULT_STOPPED_SPEED_METERS_PER_SECOND: f32 = 0.08;

/// Tunable parameters for an [`Autopilot`]'s arrival behavior.
#[derive(Clone, Copy, Debug)]
pub struct AutopilotConfig {
    /// Distance from the destination at which arrival is declared.
    pub arrival_radius_meters: f32,
    /// Speed at or below which a ship inside the arrival radius is stopped.
    pub stopped_speed_meters_per_second: f32,
}
impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            arrival_radius_meters: DEFAULT_ARRIVAL_RADIUS_METERS,
            stopped_speed_meters_per_second: DEFAULT_STOPPED_SPEED_METERS_PER_SECOND,
        }
    }
}
/// High-level destination targeting that delegates control output to a
/// swappable [`FlightController`].
///
/// The autopilot owns an optional destination and an active flag. Each tick it
/// asks its controller for the [`FlightInput`] required to reach the destination,
/// and deactivates itself once the ship has arrived (within
/// [`AutopilotConfig::arrival_radius_meters`] and below
/// [`AutopilotConfig::stopped_speed_meters_per_second`]) with no further input
/// demanded by the controller.
pub struct Autopilot {
    controller: Box<dyn FlightController>,
    config: AutopilotConfig,
    destination: Option<Vec2>,
    active: bool,
}
impl Autopilot {
    pub fn new(controller: Box<dyn FlightController>, config: AutopilotConfig) -> Self {
        Self {
            controller,
            config,
            destination: None,
            active: false,
        }
    }
    pub fn set_destination(&mut self, destination: Vec2) {
        self.destination = Some(destination);
        self.active = true;
    }
    pub fn cancel_and_clear_destination(&mut self) {
        self.destination = None;
        self.active = false;
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn destination(&self) -> Option<Vec2> {
        self.destination
    }
    pub(crate) fn config(&self) -> AutopilotConfig {
        self.config
    }
    pub(crate) fn controller_name(&self) -> &'static str {
        self.controller.name()
    }
    pub fn controls_for_tick(
        &mut self,
        ship: &ShipState,
        neighbors: &[NeighborObservation],
    ) -> FlightInput {
        self.controls_for_tick_with_hitbox(ship, Hitbox::default_ship(), neighbors)
    }

    pub fn controls_for_tick_with_hitbox(
        &mut self,
        ship: &ShipState,
        hitbox: Hitbox,
        neighbors: &[NeighborObservation],
    ) -> FlightInput {
        let Some(destination) = self.destination else {
            return FlightInput::default();
        };
        let desired = self
            .controller
            .desired_input(FlightObservation::from_ship_with_hitbox(
                ship,
                hitbox,
                destination,
                neighbors,
            ));
        let settled = ship.position.distance(destination) <= self.config.arrival_radius_meters
            && ship.velocity.length() <= self.config.stopped_speed_meters_per_second
            && desired == FlightInput::default();
        self.active = !settled;
        desired
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_control::{FlightController, FlightObservation, NeighborRelationship};
    #[derive(Debug)]
    struct Script(FlightInput);
    impl FlightController for Script {
        fn name(&self) -> &'static str {
            "script"
        }
        fn desired_input(&self, _: FlightObservation) -> FlightInput {
            self.0
        }
    }
    fn ap(input: FlightInput) -> Autopilot {
        Autopilot::new(Box::new(Script(input)), AutopilotConfig::default())
    }
    #[test]
    fn destination_replacement_keeps_marker() {
        let mut a = ap(FlightInput::default());
        a.set_destination(Vec2::X);
        a.set_destination(Vec2::Y);
        assert_eq!(a.destination(), Some(Vec2::Y));
    }
    #[test]
    fn arrival_requires_position_and_speed() {
        let mut a = ap(FlightInput::default());
        a.set_destination(Vec2::ZERO);
        let s = ShipState {
            velocity: Vec2::X,
            ..Default::default()
        };
        assert!(a.controls_for_tick(&s, &[]) == FlightInput::default() && a.is_active());
        let s = ShipState::default();
        assert_eq!(a.controls_for_tick(&s, &[]), FlightInput::default());
        assert!(!a.is_active());
    }
    #[test]
    fn settled_autopilot_reactivates_for_neighbor_guidance() {
        #[derive(Debug)]
        struct NeighborGuidance;
        impl FlightController for NeighborGuidance {
            fn name(&self) -> &'static str {
                "neighbor-guidance"
            }
            fn desired_input(&self, observation: FlightObservation) -> FlightInput {
                if observation.neighbors.is_empty() {
                    FlightInput::default()
                } else {
                    FlightInput {
                        turn_left: true,
                        ..Default::default()
                    }
                }
            }
        }

        let mut autopilot = Autopilot::new(Box::new(NeighborGuidance), AutopilotConfig::default());
        autopilot.set_destination(Vec2::ZERO);
        assert_eq!(
            autopilot.controls_for_tick(&ShipState::default(), &[]),
            FlightInput::default()
        );
        assert!(!autopilot.is_active());

        let neighbor = NeighborObservation {
            entity_id: crate::flight_control::AvoidanceEntityId::Unit(crate::command::UnitId(1)),
            position: Vec2::X,
            velocity: Vec2::ZERO,
            hitbox: Hitbox::default_ship(),
            relationship: NeighborRelationship::Friendly,
        };
        let input = autopilot.controls_for_tick(&ShipState::default(), &[neighbor]);
        assert!(input.turn_left);
        assert!(autopilot.is_active());
    }

    #[test]
    fn cancel_clears_destination() {
        let mut a = ap(FlightInput::default());
        a.set_destination(Vec2::X);
        a.cancel_and_clear_destination();
        assert!(!a.is_active());
        assert_eq!(a.destination(), None);
    }
}
