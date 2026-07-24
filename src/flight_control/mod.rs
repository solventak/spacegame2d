use glam::Vec2;

use crate::simulation::{
    FORWARD_THRUST_NEWTONS, MAX_ANGULAR_SPEED_RADIANS_PER_SECOND, SHIP_MASS_KG, ShipInput,
    ShipState,
};

pub mod arrival;
pub use arrival::ArrivalController;

#[derive(Clone, Copy, Debug)]
pub struct NeighborObservation {
    pub position: Vec2,
    pub velocity: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct FlightObservation<'a> {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading_radians: f32,
    pub angular_velocity_radians_per_second: f32,
    pub destination: Vec2,
    pub neighbors: &'static [NeighborObservation],
}

impl FlightObservation<'_> {
    pub fn from_ship<'a>(
        ship: &ShipState,
        destination: Vec2,
        neighbors: &'a [NeighborObservation],
    ) -> FlightObservation<'a> {
        Self {
            position: ship.position,
            velocity: ship.velocity,
            heading_radians: ship.heading_radians,
            angular_velocity_radians_per_second: ship.angular_velocity_radians_per_second,
            destination,
            neighbors,
        }
    }
}

pub trait FlightController: std::fmt::Debug {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn desired_input(&self, observation: FlightObservation<'_>) -> ShipInput;
}

pub(crate) fn forward(heading: f32) -> Vec2 {
    Vec2::new(-heading.sin(), heading.cos())
}
pub(crate) fn thrust_acceleration() -> f32 {
    FORWARD_THRUST_NEWTONS / SHIP_MASS_KG
}
pub(crate) fn max_angular_speed() -> f32 {
    MAX_ANGULAR_SPEED_RADIANS_PER_SECOND
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    struct AlwaysThrust;
    impl FlightController for AlwaysThrust {
        fn name(&self) -> &'static str {
            "always-thrust"
        }
        fn desired_input(&self, _: FlightObservation) -> ShipInput {
            ShipInput {
                thrust: true,
                ..Default::default()
            }
        }
    }
    #[test]
    fn controller_is_swappable_behind_trait_object() {
        let controller: Box<dyn FlightController> = Box::new(AlwaysThrust);
        assert_eq!(controller.name(), "always-thrust");
        assert!(
            controller
                .desired_input(FlightObservation::from_ship(&ShipState::default(), Vec2::X))
                .thrust
        );
    }
}
