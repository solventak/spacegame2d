//! Flight control abstraction: turn a flight observation into per-tick ship
//! input.
//!
//! The [`FlightController`] trait decouples the steering policy (how to reach a
//! destination) from the integrator in [`crate::simulation`]. The concrete
//! [`ArrivalController`] implements a velocity-arrival controller with
//! optional neighbor avoidance.

use glam::Vec2;

use crate::simulation::{
    FORWARD_THRUST_NEWTONS, MAX_ANGULAR_SPEED_RADIANS_PER_SECOND, SHIP_MASS_KG, ShipInput,
    ShipState,
};

pub mod arrival;
pub use arrival::ArrivalController;

/// Snapshot of one neighbor's kinematic state, observed at the start of a tick
/// so every drone sees a consistent world.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct NeighborObservation {
    /// Neighbor position in arena-space meters.
    pub position: Vec2,
    /// Neighbor velocity in meters per second.
    pub velocity: Vec2,
}

/// Read-only view of the ship and world state a [`FlightController`] uses to
/// compute control output for a single tick.
#[derive(Clone, Copy, Debug)]
pub struct FlightObservation<'a> {
    /// Ship position in arena-space meters.
    pub position: Vec2,
    /// Ship velocity in meters per second.
    pub velocity: Vec2,
    /// Ship heading in radians.
    pub heading_radians: f32,
    /// Ship angular velocity in radians per second.
    pub angular_velocity_radians_per_second: f32,
    /// Target destination in arena-space meters.
    pub destination: Vec2,
    /// Neighbors visible to this ship this tick.
    #[allow(dead_code)]
    pub neighbors: &'a [NeighborObservation],
}

impl<'a> FlightObservation<'a> {
    pub fn from_ship(
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

/// Steering policy: map a flight observation to the per-tick [`ShipInput`]
/// that best drives the ship toward its destination.
///
/// Implementations are held behind a `Box<dyn FlightController>` by the
/// [`Autopilot`](crate::autopilot::Autopilot) so the controller can be swapped
/// without changing the autopilot's surface.
pub trait FlightController: std::fmt::Debug {
    /// Human-readable identifier for this controller, mainly for diagnostics.
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    /// Compute the ship input for this tick given `observation`.
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
                .desired_input(FlightObservation::from_ship(
                    &ShipState::default(),
                    Vec2::X,
                    &[]
                ))
                .thrust
        );
    }
}
