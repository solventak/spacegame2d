use glam::Vec2;

use crate::autopilot::{Autopilot, AutopilotConfig};
use crate::flight_control::{ArrivalController, NeighborObservation, NeighborRelationship};
use crate::simulation::{ShipState, is_out_of_bounds, step_ship};

/// Number of drones spawned by [`Fleet::new`] and [`Fleet::reset`].
pub const DRONE_COUNT: usize = 30;

/// One drone: its physical state paired with the autopilot driving it.
///
/// Pairing them in a single struct removes the parallel-array invariant that
/// used to exist between `Vec<ShipState>` and `Vec<Autopilot>` -- there is now
/// exactly one collection to add to or remove from, so state and controller can
/// never diverge.
pub struct Unit {
    pub state: ShipState,
    pub autopilot: Autopilot,
}

impl Unit {
    fn new_drone(state: ShipState) -> Self {
        Self {
            state,
            autopilot: Autopilot::new(
                Box::new(ArrivalController::default()),
                AutopilotConfig::default(),
            ),
        }
    }
}

/// Owned collection of drones. Centralizes spawn, per-tick stepping, neighbor
/// observation, arena culling, and reset so callers never hand-roll index
/// coupling between state and controller arrays.
///
/// Today units are identified by their position in the internal `Vec`. When
/// persistent unit identity is needed (per-unit orders, targeting, netcode),
/// the stable-ID mapping can live entirely behind this type -- swap the inner
/// storage for a slotmap/arena and expose a `UnitId` without churning the
/// `step` / `cull` / `reset` / `set_destination` surface.
pub struct Fleet {
    units: Vec<Unit>,
}

impl Fleet {
    pub fn new() -> Self {
        Self {
            units: initial_units(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn len(&self) -> usize {
        self.units.len()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Borrow the live units, in render order.
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Respawn the full starting swarm, discarding survivors.
    pub fn reset(&mut self) {
        self.units = initial_units();
    }

    /// Set a shared destination on every drone's autopilot.
    pub fn set_destination(&mut self, destination: Vec2) {
        for unit in &mut self.units {
            unit.autopilot.set_destination(destination);
        }
    }

    /// Advance every drone by one tick. Each drone computes its controls from a
    /// consistent snapshot of neighbor positions/velocities taken at the start
    /// of the tick, then integrates.
    pub fn step(&mut self) {
        // Snapshot neighbor state once so every drone sees the same world within
        // this tick (no read-after-write from already-stepped peers).
        let observations: Vec<NeighborObservation> = self
            .units
            .iter()
            .map(|u| NeighborObservation {
                position: u.state.position,
                velocity: u.state.velocity,
                relationship: NeighborRelationship::Friendly,
            })
            .collect();
        for (index, unit) in self.units.iter_mut().enumerate() {
            let neighbors: Vec<NeighborObservation> = observations
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, n)| *n)
                .collect();
            let controls = unit.autopilot.controls_for_tick(&unit.state, &neighbors);
            step_ship(&mut unit.state, controls);
        }
    }

    /// Remove drones that have left the arena, logging each destruction. Uses
    /// `swap_remove` so a single pass mutates only this collection with no
    /// per-tick allocation; drone order is not semantically meaningful.
    pub fn cull(&mut self, world_radius: f32) {
        let mut i = 0;
        while i < self.units.len() {
            let pos = self.units[i].state.position;
            if is_out_of_bounds(pos, world_radius) {
                let removed = self.units.swap_remove(i);
                log::info!(
                    "drone destroyed: out of bounds at ({:.1}, {:.1})",
                    removed.state.position.x,
                    removed.state.position.y
                );
            } else {
                i += 1;
            }
        }
    }
}

impl Default for Fleet {
    fn default() -> Self {
        Self::new()
    }
}

fn initial_units() -> Vec<Unit> {
    initial_drone_positions()
        .into_iter()
        .map(Unit::new_drone)
        .collect()
}

/// Deterministic starting positions for the drone swarm, derived from a fixed
/// PRNG seed so resets reproduce the same layout.
pub fn initial_drone_positions() -> Vec<ShipState> {
    initial_fleet_positions(0x5EED_1234, -4.0)
}

pub fn initial_world_positions() -> Vec<ShipState> {
    let mut positions = initial_fleet_positions(0x5EED_1234, -4.0);
    positions.extend(initial_fleet_positions(0xC0FF_EE42, 4.0));
    positions
}

fn initial_fleet_positions(mut seed: u32, x_offset: f32) -> Vec<ShipState> {
    (0..DRONE_COUNT)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = (seed as f32 / u32::MAX as f32) * 3.0 - 1.5 + x_offset;
            seed = seed.rotate_left(13);
            let y = (seed as f32 / u32::MAX as f32) * 6.0 - 3.0;
            ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::WORLD_RADIUS_M;

    #[test]
    fn initial_drones_are_deterministic_and_on_screen() {
        let first = initial_drone_positions();
        assert_eq!(first, initial_drone_positions());
        assert_eq!(first.len(), DRONE_COUNT);
        assert!(
            first
                .iter()
                .all(|drone| { drone.position.x.abs() <= 8.0 && drone.position.y.abs() <= 5.0 })
        );
    }

    #[test]
    fn new_fleet_has_drone_count_units() {
        let fleet = Fleet::new();
        assert_eq!(fleet.len(), DRONE_COUNT);
        assert_eq!(fleet.units().len(), DRONE_COUNT);
    }

    #[test]
    fn reset_restores_full_count_after_cull() {
        let mut fleet = Fleet::new();
        // Push every drone out of bounds, cull, then reset.
        for unit in fleet.units.iter_mut() {
            unit.state.position = Vec2::new(WORLD_RADIUS_M + 5.0, 0.0);
        }
        fleet.cull(WORLD_RADIUS_M);
        assert!(fleet.is_empty());
        fleet.reset();
        assert_eq!(fleet.len(), DRONE_COUNT);
    }

    #[test]
    fn cull_removes_out_of_bounds_and_keeps_in_bounds() {
        let mut fleet = Fleet::new();
        // Half the drones escape, half stay at the origin.
        for (i, unit) in fleet.units.iter_mut().enumerate() {
            if i % 2 == 0 {
                unit.state.position = Vec2::new(WORLD_RADIUS_M + 1.0, 0.0);
            } else {
                unit.state.position = Vec2::ZERO;
            }
        }
        fleet.cull(WORLD_RADIUS_M);
        let survivors = fleet.units();
        assert!(!survivors.is_empty());
        assert!(
            survivors
                .iter()
                .all(|u| !is_out_of_bounds(u.state.position, WORLD_RADIUS_M)),
            "no out-of-bounds drone should survive culling"
        );
    }

    #[test]
    fn step_with_no_destination_keeps_drones_stationary() {
        let mut fleet = Fleet::new();
        let before: Vec<ShipState> = fleet.units().iter().map(|u| u.state).collect();
        fleet.step();
        let after: Vec<ShipState> = fleet.units().iter().map(|u| u.state).collect();
        assert_eq!(before, after, "idle drones should not move");
    }

    #[test]
    fn set_destination_marks_all_autopilots_active() {
        let mut fleet = Fleet::new();
        fleet.set_destination(Vec2::new(10.0, 10.0));
        assert!(fleet.units().iter().all(|u| u.autopilot.is_active()));
        assert!(
            fleet
                .units()
                .iter()
                .all(|u| u.autopilot.destination() == Some(Vec2::new(10.0, 10.0)))
        );
    }
}
