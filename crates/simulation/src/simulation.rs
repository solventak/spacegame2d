use crate::combat::CombatTargetId;
use crate::command::{CommandScheduler, World};
use crate::config::SimulationConfig;
use crate::events::simulation_event_sort_key;
use crate::systems::{combat, movement, objective};
use spacegame2d_protocol::{AuthoritativeCommand, Tick};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use crate::objective::ObjectiveState;

pub use crate::events::{MatchResult, SimulationEvent};
pub use crate::physics::{
    ANGULAR_DAMPING_PER_SECOND, ANGULAR_THRUST_NEWTON_METERS, FIXED_DT_SECONDS,
    FORWARD_THRUST_NEWTONS, FlightInput, LINEAR_DAMPING_PER_SECOND,
    MAX_ANGULAR_SPEED_RADIANS_PER_SECOND, MAX_SPEED_METERS_PER_SECOND, MOMENT_OF_INERTIA_KG_M2,
    SHIP_MASS_KG, SIMULATION_HZ, ShipState, WORLD_RADIUS_M, is_out_of_bounds, step_ship,
};

/// Fixed-timestep driver for the autopilot drone world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppliedAuthoritativeCommands {
    pub reset_applied: bool,
}

pub struct Simulation {
    tick: Tick,
    pub world: World,
    pub commands: CommandScheduler,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new(SimulationConfig::default())
    }
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            tick: Tick::default(),
            world: World::new(config),
            commands: CommandScheduler::default(),
        }
    }
}

impl Simulation {
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// Align a mirror simulation with the authoritative server clock.
    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
    }

    pub fn with_world_radius(world_radius: f32) -> Self {
        let config = SimulationConfig::default()
            .with_world_radius_meters(world_radius)
            .expect("test world radius must be valid");
        Self::new(config)
    }

    pub fn world_radius(&self) -> f32 {
        self.config().world_radius_meters()
    }

    pub fn config(&self) -> SimulationConfig {
        self.world.config()
    }

    /// Restore the canonical match world while preserving the monotonic simulation tick.
    pub fn reset_match(&mut self) -> Result<(), crate::command::CommandExecutionError> {
        self.commands.clear_pending();
        self.world.reset_match()
    }

    pub fn world(&self) -> &World {
        &self.world
    }
    pub fn schedule_authoritative(&mut self, cmd: &AuthoritativeCommand) -> bool {
        if self.world.validate_authoritative(cmd).is_err() {
            return false;
        }
        self.schedule_authoritative_trusted(cmd)
    }

    pub fn apply_due_commands(
        &mut self,
        scheduled: &mut BTreeMap<Tick, Vec<AuthoritativeCommand>>,
    ) -> AppliedAuthoritativeCommands {
        let current_tick = self.tick;
        let due_ticks: Vec<Tick> = scheduled
            .range(..=current_tick)
            .map(|(tick, _)| *tick)
            .collect();
        let mut result = AppliedAuthoritativeCommands::default();
        for tick in due_ticks {
            let Some(commands) = scheduled.remove(&tick) else {
                continue;
            };
            for mut command in commands {
                if result.reset_applied {
                    break;
                }
                if command.execute_tick < current_tick {
                    command.execute_tick = current_tick;
                }
                if matches!(
                    command.command,
                    spacegame2d_protocol::CommandData::ResetSimulation
                ) {
                    scheduled.clear();
                    self.commands.clear_pending();
                    result.reset_applied = true;
                }
                self.schedule_authoritative_trusted(&command);
            }
        }
        result
    }

    /// Schedule a command already validated by the authoritative server.
    /// Client mirrors use this path because they do not maintain the server's
    /// complete ownership registry.
    pub fn schedule_authoritative_trusted(&mut self, cmd: &AuthoritativeCommand) -> bool {
        let Ok(command) = Box::<dyn crate::command::Command>::try_from(cmd) else {
            return false;
        };
        self.commands.schedule(cmd.execute_tick, command);
        true
    }

    /// Advance one deterministic tick by applying queued commands, stepping the
    /// authoritative World units, and deriving transient boundary events.
    pub fn step(&mut self) -> Result<Vec<SimulationEvent>, crate::command::CommandExecutionError> {
        self.commands.execute_pending(self.tick, &mut self.world)?;
        movement::run(&mut self.world);
        let mut events = Vec::new();
        combat::run(&mut self.world, self.tick, &mut events);
        let world_radius = self.world_radius();
        self.world.units.retain(|unit| {
            if unit.combat.hull.current == 0 {
                events.push(SimulationEvent::HullDepleted {
                    tick: self.tick,
                    unit_id: unit.id,
                    position: unit.state.position,
                });
                false
            } else if is_out_of_bounds(unit.state.position, world_radius) {
                events.push(SimulationEvent::BoundaryCrossed {
                    tick: self.tick,
                    unit_id: unit.id,
                    position: unit.state.position,
                });
                false
            } else {
                true
            }
        });
        if let Some(outcome) = objective::run(&mut self.world, self.tick, false, &mut events) {
            events.push(SimulationEvent::MatchResult {
                tick: self.tick,
                outcome,
            });
            self.commands.clear_pending();
            self.world.reset_match()?;
        }
        let live_ids = self
            .world
            .units
            .iter()
            .map(|unit| unit.id)
            .collect::<BTreeSet<_>>();
        for unit in &mut self.world.units {
            if unit.combat.turret.target.is_some_and(
                |target| matches!(target, CombatTargetId::Unit(id) if !live_ids.contains(&id)),
            ) {
                unit.combat.turret.target = None;
            }
        }
        events.sort_by_key(simulation_event_sort_key);
        self.tick = self.tick.increment(Tick::new(1));
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use spacegame2d_protocol::CommandData;
    use std::collections::BTreeMap;

    use crate::autopilot::{Autopilot, AutopilotConfig};
    use crate::combat::{ImpactEntityId, WEAPON_DAMAGE};
    use crate::command::{PlayerId, UnitId};
    use crate::flight_control::{
        ArrivalController, AvoidanceProfile, AvoidanceProfiles, NeighborRelationship,
    };
    use crate::objective::{BREACH_DURATION_TICKS, EXPOSURE_DURATION_TICKS};
    use glam::Vec2;

    #[test]
    fn starts_without_player_ship() {
        let sim = Simulation::default();
        assert_eq!(sim.tick(), Tick::new(0));
    }
    #[test]
    fn step_advances_tick_without_player_input() {
        let mut sim = Simulation::default();
        assert!(sim.step().unwrap().is_empty());
        assert_eq!(sim.tick(), Tick::new(1));
    }

    fn attacker_at_first_relay() -> Simulation {
        let mut simulation = Simulation::default();
        simulation.world.units.truncate(1);
        simulation.world.units[0].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = simulation.world.structures()[1].position();
        simulation
    }

    #[test]
    fn relay_breach_uses_one_attacker_token_and_exact_duration() {
        let mut simulation = attacker_at_first_relay();
        for _ in 0..BREACH_DURATION_TICKS - 1 {
            simulation.step().unwrap();
        }
        let pair = simulation.world.home_objective_pairs()[0];
        assert_eq!(pair.state(), ObjectiveState::Breaching);
        assert_eq!(pair.breach_progress_ticks(), BREACH_DURATION_TICKS - 1);

        let events = simulation.step().unwrap();
        let pair = simulation.world.home_objective_pairs()[0];
        assert_eq!(pair.state(), ObjectiveState::Exposed);
        assert_eq!(pair.exposure_ticks_remaining(), EXPOSURE_DURATION_TICKS);
        assert!(matches!(
            events.as_slice(),
            [SimulationEvent::ObjectiveTransition {
                previous_state: ObjectiveState::Breaching,
                next_state: ObjectiveState::Exposed,
                ..
            }]
        ));
    }

    #[test]
    fn defender_contests_would_be_completion_tick_and_unowned_units_do_not() {
        let mut simulation = attacker_at_first_relay();
        simulation.world.units.push(crate::command::Unit::new(
            UnitId(99),
            None,
            ShipState {
                position: simulation.world.structures()[1].position(),
                ..ShipState::default()
            },
        ));
        for _ in 0..BREACH_DURATION_TICKS - 1 {
            simulation.step().unwrap();
        }
        simulation.world.units[1].owner = Some(PlayerId(1));
        let events = simulation.step().unwrap();
        let pair = simulation.world.home_objective_pairs()[0];
        assert_eq!(pair.state(), ObjectiveState::Contested);
        assert_eq!(pair.breach_progress_ticks(), BREACH_DURATION_TICKS - 1);
        assert!(events.iter().any(|event| matches!(
            event,
            SimulationEvent::ObjectiveTransition {
                previous_state: ObjectiveState::Breaching,
                next_state: ObjectiveState::Contested,
                ..
            }
        )));
    }

    #[test]
    fn breach_decays_recovers_and_then_starts_again_on_following_tick() {
        let mut simulation = attacker_at_first_relay();
        for _ in 0..BREACH_DURATION_TICKS {
            simulation.step().unwrap();
        }
        for _ in 0..EXPOSURE_DURATION_TICKS - 1 {
            simulation.step().unwrap();
        }
        assert_eq!(
            simulation.world.home_objective_pairs()[0].state(),
            ObjectiveState::Exposed
        );
        let recovery = simulation.step().unwrap();
        let pair = simulation.world.home_objective_pairs()[0];
        assert_eq!(pair.state(), ObjectiveState::Protected);
        assert_eq!(pair.breach_progress_ticks(), 0);
        assert!(matches!(
            recovery.as_slice(),
            [SimulationEvent::ObjectiveTransition {
                previous_state: ObjectiveState::Exposed,
                next_state: ObjectiveState::Protected,
                ..
            }]
        ));
        simulation.step().unwrap();
        let pair = simulation.world.home_objective_pairs()[0];
        assert_eq!(pair.state(), ObjectiveState::Breaching);
        assert_eq!(pair.breach_progress_ticks(), 1);
    }

    #[test]
    fn frozen_objectives_do_not_change_or_emit_events() {
        let mut simulation = attacker_at_first_relay();
        let mut events = Vec::new();
        let tick = simulation.tick();
        objective::run(&mut simulation.world, tick, true, &mut events);
        assert!(events.is_empty());
        assert_eq!(
            simulation.world.home_objective_pairs()[0].state(),
            ObjectiveState::Protected
        );
    }

    #[test]
    fn step_emits_boundary_crossed_and_removes_unit() {
        let mut sim = Simulation::with_world_radius(1.0);
        sim.world.units.truncate(1);
        sim.world.units[0].state.position = Vec2::new(1.1, 0.0);
        let unit_id = sim.world.units[0].id;

        let events = sim.step().unwrap();

        assert_eq!(
            events,
            vec![SimulationEvent::BoundaryCrossed {
                tick: Tick::default(),
                unit_id,
                position: Vec2::new(1.1, 0.0),
            }]
        );
        assert!(sim.world.units.is_empty());
        assert_eq!(sim.tick(), Tick::new(1));
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

        let events = sim.step().unwrap();
        let actual = events
            .iter()
            .map(|event| match event {
                SimulationEvent::ShotFired { shooter_id, .. } => *shooter_id,
                SimulationEvent::HullDepleted { unit_id, .. }
                | SimulationEvent::BoundaryCrossed { unit_id, .. } => *unit_id,
                SimulationEvent::ObjectiveTransition { .. } => {
                    unreachable!("this test only produces boundary events")
                }
                SimulationEvent::CoreHitProtected { .. } | SimulationEvent::MatchResult { .. } => {
                    unreachable!("this test only produces boundary events")
                }
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

    type UnitSnapshot = (
        UnitId,
        Option<PlayerId>,
        ShipState,
        crate::combat::CombatState,
        Option<Vec2>,
        bool,
    );

    fn unit_snapshot(world: &World) -> Vec<UnitSnapshot> {
        world
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    u.owner,
                    u.state,
                    u.combat,
                    u.autopilot.destination(),
                    u.autopilot.is_active(),
                )
            })
            .collect()
    }

    #[derive(Debug, PartialEq)]
    struct AuthoritativeSnapshot {
        tick: Tick,
        units: Vec<UnitSnapshot>,
        events: Vec<SimulationEvent>,
    }

    fn authoritative_set_destination(
        execute_tick: u64,
        player_slot: u32,
        sequence: u32,
        unit_id: u32,
        destination: [u32; 2],
    ) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: Tick::from(execute_tick),
            player_slot,
            sequence,
            command: CommandData::SetDestination {
                destination,
                target_unit_ids: vec![unit_id],
            },
        }
    }

    #[test]
    fn unit_id_and_owner_are_stable_across_commands() {
        // CMD-002: stable identity and ownership.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let id_before = sim.world.units[0].id;
        let owner_before = sim.world.units[0].owner;
        let cmd = authoritative_set_destination(0, 1, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        sim.step().unwrap();
        assert_eq!(sim.world.units[0].id, id_before);
        assert_eq!(sim.world.units[0].owner, owner_before);
    }

    #[test]
    fn cross_player_command_is_rejected() {
        // CMD-004: ownership validation.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let pos_before = sim.world.units[0].state.position;
        let cmd = authoritative_set_destination(0, 2, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(!sim.schedule_authoritative(&cmd));
        let events = sim.step().unwrap();
        assert_eq!(sim.world.units[0].state.position, pos_before);
        assert!(events.is_empty());
        assert!(sim.commands.history().is_empty());
    }

    #[test]
    fn invalid_commands_leave_world_unchanged() {
        // CMD-006: invalid commands cause no state mutation or panic.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let snapshot_before = unit_snapshot(&sim.world);

        let bad_slot = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 0,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
                target_unit_ids: vec![1],
            },
        };
        assert!(!sim.schedule_authoritative(&bad_slot));

        let unknown_unit =
            authoritative_set_destination(0, 2, 2, 999, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&unknown_unit));

        let nan_destination = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 3,
            command: CommandData::SetDestination {
                destination: [f32::NAN.to_bits(), 0.0f32.to_bits()],
                target_unit_ids: vec![1],
            },
        };
        assert!(!sim.schedule_authoritative(&nan_destination));

        let events = sim.step().unwrap();
        assert_eq!(unit_snapshot(&sim.world), snapshot_before);
        assert!(events.is_empty());
        assert_eq!(sim.commands.history().len(), 1);
    }

    #[test]
    fn commands_execute_before_physics_and_tick_advances_by_one() {
        // CMD-007: fixed tick order. The command must be applied before physics
        // so the same tick produces a steering result based on the new destination.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        // Position the unit far from the requested destination and face it so
        // the autopilot produces thrust on the very first tick.
        sim.world.units[0].state.position = Vec2::new(10.0, 0.0);
        // Forward vector is (-sin(h), cos(h)); heading = PI/2 points along -X.
        sim.world.units[0].state.heading_radians = std::f32::consts::FRAC_PI_2;
        let cmd = authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        let tick_before = sim.tick();
        sim.step().unwrap();
        assert_eq!(sim.tick(), tick_before + Tick::new(1));
        // The command applied before physics: the destination is set and the
        // unit has moved toward it in the same tick.
        assert_eq!(sim.world.units[0].autopilot.destination(), Some(Vec2::ZERO));
        assert_ne!(sim.world.units[0].state.position, Vec2::new(10.0, 0.0));
    }

    #[test]
    fn set_destination_does_not_teleport_units() {
        // CMD-010: no teleport.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let pos_before = sim.world.units[0].state.position;
        let cmd =
            authoritative_set_destination(0, 1, 1, 1, [100.0f32.to_bits(), 100.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        sim.step().unwrap();
        assert!(
            sim.world.units[0].state.position.distance(pos_before)
                <= MAX_SPEED_METERS_PER_SECOND * FIXED_DT_SECONDS + 1.0e-6
        );
    }

    #[test]
    fn history_records_only_accepted_commands() {
        // CMD-014: accepted-only history.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let accepted =
            authoritative_set_destination(0, 1, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        let rejected =
            authoritative_set_destination(0, 2, 2, 1, [3.0f32.to_bits(), 4.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&accepted));
        assert!(!sim.schedule_authoritative(&rejected));
        sim.step().unwrap();
        assert_eq!(sim.commands.history().len(), 1);
        assert_eq!(sim.commands.history()[0].execute_tick(), Tick::new(0));
    }

    #[test]
    fn cross_peer_replay_is_deterministic_tick_by_tick() {
        // DEP-005: compare the authoritative per-tick snapshot, rather than
        // rather than only comparing a legacy aggregate at the end.
        let commands = vec![
            authoritative_set_destination(1, 1, 1, 1, [6.0f32.to_bits(), 5.0f32.to_bits()]),
            authoritative_set_destination(3, 1, 2, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]),
        ];
        let ticks = 10;

        let mut a = Simulation::default();
        a.world.units[0].owner = Some(PlayerId(1));
        let mut a_snapshots = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == a.tick() {
                    a.schedule_authoritative(cmd);
                }
            }
            let events = a.step().unwrap();
            a_snapshots.push(AuthoritativeSnapshot {
                tick: a.tick(),
                units: unit_snapshot(&a.world),
                events,
            });
        }

        let mut b = Simulation::default();
        b.world.units[0].owner = Some(PlayerId(1));
        let mut b_snapshots = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == b.tick() {
                    b.schedule_authoritative(cmd);
                }
            }
            let events = b.step().unwrap();
            b_snapshots.push(AuthoritativeSnapshot {
                tick: b.tick(),
                units: unit_snapshot(&b.world),
                events,
            });
        }

        assert_eq!(a_snapshots, b_snapshots);
    }

    #[test]
    fn cross_peer_boundary_event_vectors_are_non_empty_at_same_ticks() {
        let command =
            authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 100.0f32.to_bits()]);
        let mut peers = [
            Simulation::with_world_radius(1.0),
            Simulation::with_world_radius(1.0),
        ];
        for peer in &mut peers {
            peer.world.units.truncate(1);
            peer.world.units[0].owner = Some(PlayerId(1));
            peer.world.units[0].state.position = Vec2::new(0.0, 0.9);
            peer.world.units[0].state.heading_radians = 0.0;
            assert!(peer.schedule_authoritative(&command));
        }

        let mut event_vectors: [Vec<Vec<SimulationEvent>>; 2] = [Vec::new(), Vec::new()];
        for _ in 0..20 {
            for (events, peer) in event_vectors.iter_mut().zip(&mut peers) {
                events.push(peer.step().unwrap());
            }
        }

        assert_eq!(event_vectors[0], event_vectors[1]);
        let non_empty: Vec<_> = event_vectors[0]
            .iter()
            .enumerate()
            .filter(|(_, events)| !events.is_empty())
            .collect();
        assert_eq!(non_empty.len(), 1);
        assert_eq!(non_empty[0].0, 9);
        assert!(matches!(
            non_empty[0].1.as_slice(),
            [SimulationEvent::BoundaryCrossed { tick, .. }] if *tick == Tick::from(9)
        ));
    }

    #[test]
    fn authoritative_command_drives_unit_across_boundary_on_two_peers() {
        let command =
            authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 100.0f32.to_bits()]);
        let mut peers = [
            Simulation::with_world_radius(1.0),
            Simulation::with_world_radius(1.0),
        ];
        for peer in &mut peers {
            peer.world.units.truncate(1);
            peer.world.units[0].owner = Some(PlayerId(1));
            peer.world.units[0].state.position = Vec2::new(0.0, 0.9);
            peer.world.units[0].state.heading_radians = 0.0;
            assert!(peer.schedule_authoritative(&command));
        }

        let mut control = Simulation::with_world_radius(1.0);
        control.world.units.truncate(1);
        control.world.units[0].owner = Some(PlayerId(1));
        control.world.units[0].state.position = Vec2::new(0.0, 0.9);
        control.world.units[0].state.heading_radians = 0.0;

        let mut peer_events: [Vec<Vec<SimulationEvent>>; 2] = [Vec::new(), Vec::new()];
        for _ in 0..30 {
            for (events, peer) in peer_events.iter_mut().zip(&mut peers) {
                events.push(peer.step().unwrap());
            }
            assert!(control.step().unwrap().is_empty());
        }

        assert!(peer_events[0].iter().any(|events| !events.is_empty()));
        assert_eq!(peer_events[0], peer_events[1]);
        assert!(peers.iter().all(|peer| peer.world.units.is_empty()));
    }

    #[test]
    fn recorded_history_replays_to_identical_state_and_events() {
        // CMD-008: replay determinism.
        let commands = vec![
            authoritative_set_destination(1, 1, 1, 1, [6.0f32.to_bits(), 5.0f32.to_bits()]),
            authoritative_set_destination(3, 1, 2, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]),
        ];
        let ticks = 10;

        let mut original = Simulation::default();
        original.world.units[0].owner = Some(PlayerId(1));
        let mut original_events = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == original.tick() {
                    original.schedule_authoritative(cmd);
                }
            }
            original_events.extend(original.step().unwrap());
        }
        let history = original.commands.history().to_vec();

        let mut replay = Simulation::default();
        replay.world.units[0].owner = Some(PlayerId(1));
        let replay_events = CommandScheduler::replay(&history, &mut replay, Tick::from(ticks - 1));

        assert_eq!(replay.tick(), original.tick());
        assert_eq!(unit_snapshot(&replay.world), unit_snapshot(&original.world));
        assert_eq!(replay_events.unwrap(), original_events);
    }

    #[test]
    fn avoidance_improves_pairwise_separation_without_bypassing_physics() {
        fn configured_simulation(avoidance_strength: f32) -> Simulation {
            let mut simulation = Simulation::with_world_radius(100.0);
            simulation.world.units.truncate(3);
            let states = [
                ShipState {
                    position: Vec2::new(-1.5, 0.0),
                    ..Default::default()
                },
                ShipState {
                    position: Vec2::new(1.5, 0.0),
                    ..Default::default()
                },
                ShipState {
                    position: Vec2::new(0.0, -1.5),
                    ..Default::default()
                },
            ];
            let defaults = AvoidanceProfiles::default();
            let profiles = defaults
                .profiles()
                .iter()
                .map(|profile| {
                    let strength = match profile.relationship() {
                        NeighborRelationship::Friendly | NeighborRelationship::Opposing => {
                            avoidance_strength
                        }
                        NeighborRelationship::StaticStructure => profile.strength(),
                    };
                    AvoidanceProfile::new(
                        profile.relationship(),
                        profile.group(),
                        profile.comfort_clearance_meters(),
                        strength,
                        profile.speed_squared_scale(),
                    )
                    .unwrap()
                })
                .collect();
            let avoidance = AvoidanceProfiles::new(
                defaults.prediction_horizon_seconds(),
                defaults.groups().to_vec(),
                profiles,
            )
            .unwrap();
            for (unit, state) in simulation.world.units.iter_mut().zip(states) {
                let mut controller_config = ArrivalController::default().config;
                controller_config.avoidance = avoidance.clone();
                unit.state = state;
                unit.autopilot = Autopilot::new(
                    Box::new(ArrivalController {
                        config: controller_config,
                    }),
                    AutopilotConfig::default(),
                );
                unit.autopilot.set_destination(Vec2::new(0.0, 6.0));
            }
            simulation
        }

        fn minimum_separation(simulation: &Simulation) -> f32 {
            simulation
                .world
                .units
                .iter()
                .enumerate()
                .flat_map(|(index, unit)| {
                    simulation.world.units[index + 1..]
                        .iter()
                        .map(move |other| unit.state.position.distance(other.state.position))
                })
                .fold(f32::INFINITY, f32::min)
        }

        fn run(mut simulation: Simulation) -> (f32, Vec<ShipState>) {
            let mut minimum = f32::INFINITY;
            for _ in 0..360 {
                simulation.step().unwrap();
                minimum = minimum.min(minimum_separation(&simulation));
                for unit in &simulation.world.units {
                    assert!(unit.state.position.is_finite());
                    assert!(unit.state.velocity.is_finite());
                    assert!(unit.state.velocity.length() <= MAX_SPEED_METERS_PER_SECOND);
                    assert!(unit.state.angular_velocity_radians_per_second.is_finite());
                    assert!(
                        unit.state.angular_velocity_radians_per_second.abs()
                            <= MAX_ANGULAR_SPEED_RADIANS_PER_SECOND
                    );
                }
            }
            (
                minimum,
                simulation
                    .world
                    .units
                    .iter()
                    .map(|unit| unit.state)
                    .collect(),
            )
        }

        let (baseline_separation, baseline_states) = run(configured_simulation(0.0));
        let (avoidance_separation, avoidance_states) = run(configured_simulation(8.0));
        assert!(
            avoidance_separation > baseline_separation,
            "avoidance separation {avoidance_separation} should exceed baseline {baseline_separation}"
        );
        assert!(
            avoidance_states
                .iter()
                .all(|state| state.position.distance(Vec2::new(0.0, 6.0)) < 6.0),
            "avoidance should preserve destination progress"
        );

        let (_, repeated_states) = run(configured_simulation(8.0));
        assert_eq!(avoidance_states, repeated_states);
        assert_ne!(baseline_states, avoidance_states);
    }

    #[test]
    fn set_tick_advances_clock() {
        let mut sim = Simulation::default();
        sim.set_tick(Tick::from(42));
        assert_eq!(sim.tick(), Tick::new(42));
    }

    #[test]
    fn world_accessors_and_custom_radius() {
        let sim = Simulation::with_world_radius(2.0);
        assert_eq!(sim.world_radius(), 2.0);
        assert_eq!(sim.world().units.len(), crate::command::MAX_UNITS);
        assert_eq!(sim.config().fleet_size(), crate::DEFAULT_FLEET_SIZE);
        let custom = Simulation::new(crate::SimulationConfig::new(3).unwrap());
        assert_eq!(custom.world().units.len(), 6);
    }

    #[test]
    fn reset_cutover_discards_future_commands_and_preserves_tick() {
        let mut sim = Simulation::new(crate::SimulationConfig::new(3).unwrap());
        sim.world.connect_player(PlayerId(1));
        sim.set_tick(Tick::new(4));
        let reset = AuthoritativeCommand {
            execute_tick: Tick::new(4),
            player_slot: 1,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        let destination = AuthoritativeCommand {
            execute_tick: Tick::new(5),
            player_slot: 1,
            sequence: 2,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
                target_unit_ids: vec![1],
            },
        };
        let mut scheduled = BTreeMap::from([
            (Tick::new(4), vec![reset]),
            (Tick::new(5), vec![destination]),
        ]);
        let applied = sim.apply_due_commands(&mut scheduled);
        assert!(applied.reset_applied);
        assert!(scheduled.is_empty());
        assert_eq!(sim.tick(), Tick::new(4));
        sim.step().unwrap();
        assert_eq!(sim.tick(), Tick::new(5));
        assert!(
            sim.world
                .units
                .iter()
                .all(|unit| unit.autopilot.destination().is_none())
        );
    }

    #[test]
    fn reset_simulation_records_in_history_and_replays() {
        // Covers ResetSimulation::record and command_from_record for reset.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        sim.world.connect_player(PlayerId(1));
        let reset = AuthoritativeCommand {
            execute_tick: Tick::from(2),
            player_slot: 1,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        assert!(sim.schedule_authoritative(&reset));
        sim.step().unwrap();
        sim.step().unwrap();
        sim.step().unwrap();
        assert_eq!(sim.commands.history().len(), 1);
        assert_eq!(sim.commands.history()[0].execute_tick(), Tick::new(2));
        assert!(matches!(
            sim.commands.history()[0],
            crate::command::RecordedCommand::ResetSimulation { .. }
        ));

        let history = sim.commands.history().to_vec();
        let mut replay = Simulation::default();
        replay.world.units[0].owner = Some(PlayerId(1));
        CommandScheduler::replay(&history, &mut replay, sim.tick() - Tick::new(1)).unwrap();
        assert_eq!(replay.tick(), sim.tick());
        assert_eq!(replay.world.units[0].owner, Some(PlayerId(1)));
    }
    fn combat_simulation(unit_count: u32) -> Simulation {
        let mut simulation = Simulation::new(crate::SimulationConfig::new(unit_count).unwrap());
        for unit in &mut simulation.world.units {
            unit.state.position = Vec2::new(100.0, 100.0);
        }
        simulation
    }

    #[test]
    fn aligned_target_is_acquired_and_hit_immediately() {
        let mut simulation = combat_simulation(1);
        let shooter_id = simulation.world.units[0].id;
        let target_id = simulation.world.units[1].id;
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::ZERO;
        simulation.world.units[1].state.position = Vec2::new(0.0, 5.0);

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(CombatTargetId::Unit(target_id))
        );
        assert_eq!(
            simulation.world.units[1].combat.hull.current,
            simulation.world.units[1].combat.hull.maximum - WEAPON_DAMAGE
        );
        assert!(
            matches!(events.as_slice(), [SimulationEvent::ShotFired { shooter_id: id, impact_entity: Some(ImpactEntityId::Unit(hit)), .. }] if *id == shooter_id && *hit == target_id)
        );
    }

    #[test]
    fn intervening_hostile_is_hit_without_retargeting() {
        let mut simulation = combat_simulation(2);
        simulation.world.units.truncate(3);
        let shooter_id = simulation.world.units[0].id;
        let target_id = simulation.world.units[1].id;
        let interceptor_id = simulation.world.units[2].id;
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[2].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::ZERO;
        simulation.world.units[1].state.position = Vec2::new(0.0, 8.0);
        simulation.world.units[2].state.position = Vec2::new(0.0, 4.0);
        simulation.world.units[0].combat.turret.target = Some(CombatTargetId::Unit(target_id));
        simulation.world.units[2].combat.hull.current = WEAPON_DAMAGE;

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(CombatTargetId::Unit(target_id))
        );
        assert!(simulation.world.unit(interceptor_id).is_none());
        assert!(
            matches!(events.as_slice(), [SimulationEvent::ShotFired { shooter_id: id, impact_entity: Some(ImpactEntityId::Unit(hit)), .. }, SimulationEvent::HullDepleted { unit_id, .. }] if *id == shooter_id && *hit == interceptor_id && *unit_id == interceptor_id)
        );
    }

    fn shot_past_first_core(target_y: f32) -> Simulation {
        let mut simulation = combat_simulation(1);
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::new(-40.0, -8.0);
        simulation.world.units[1].state.position = Vec2::new(-40.0, target_y);
        simulation
    }

    #[test]
    fn structure_between_shooter_and_target_intercepts_without_damage() {
        let mut simulation = shot_past_first_core(4.0);
        let target_id = simulation.world.units[1].id;
        let core = simulation.world.structures()[0];
        let structures_before = simulation.world.structures().to_vec();

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(CombatTargetId::Unit(target_id)),
            "physical interception must not retarget the turret"
        );
        assert_eq!(
            simulation.world.units[1].combat.hull.current,
            simulation.world.units[1].combat.hull.maximum
        );
        assert_eq!(simulation.world.structures(), structures_before.as_slice());
        assert!(matches!(
            events.as_slice(),
            [SimulationEvent::ShotFired {
                impact_entity: Some(ImpactEntityId::StaticStructure(id)),
                impact_position,
                ..
            }, SimulationEvent::CoreHitProtected { core_id, .. }]
                if *id == core.id() && *core_id == core.id() && *impact_position == Vec2::new(-40.0, -3.85)
        ));
    }

    #[test]
    fn target_before_structure_receives_normal_damage() {
        let mut simulation = shot_past_first_core(-4.5);
        let target_id = simulation.world.units[1].id;

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[1].combat.hull.current,
            simulation.world.units[1].combat.hull.maximum - WEAPON_DAMAGE
        );
        assert!(matches!(
            events.as_slice(),
            [SimulationEvent::ShotFired {
                impact_entity: Some(ImpactEntityId::Unit(id)),
                ..
            }] if *id == target_id
        ));
    }

    fn exposed_core_target_simulation(core_health: u32) -> Simulation {
        let mut simulation = combat_simulation(1);
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::new(40.0, -8.0);
        simulation.world.units[1].state.position = Vec2::new(100.0, 100.0);
        let pair = &mut simulation.world.home_objective_pairs_mut()[1];
        pair.set_objective_state(ObjectiveState::Exposed, BREACH_DURATION_TICKS, 1);
        pair.set_core_health_current(core_health);
        simulation
    }

    #[test]
    fn exposed_enemy_core_is_targeted_and_damaged() {
        let mut simulation = exposed_core_target_simulation(crate::MAX_CORE_HEALTH);
        let core_id = simulation.world.home_objective_pairs()[1].core_id();

        simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(CombatTargetId::CommandCore(core_id))
        );
        assert_eq!(
            simulation.world.home_objective_pairs()[1].core_health_current(),
            crate::MAX_CORE_HEALTH - WEAPON_DAMAGE
        );
    }

    #[test]
    fn core_destruction_emits_result_then_restores_reset_world() {
        let mut simulation = exposed_core_target_simulation(WEAPON_DAMAGE);
        let core_id = simulation.world.home_objective_pairs()[1].core_id();

        let events = simulation.step().unwrap();

        assert!(events.iter().any(|event| matches!(
            event,
            SimulationEvent::MatchResult {
                outcome: MatchResult::Victory { winner: PlayerId(1), loser: PlayerId(2), destroyed_core }, ..
            } if *destroyed_core == core_id
        )));
        assert!(
            simulation
                .world
                .home_objective_pairs()
                .iter()
                .all(|pair| pair.state() == ObjectiveState::Protected
                    && pair.core_health_current() == crate::MAX_CORE_HEALTH)
        );
    }

    #[test]
    fn friendly_units_do_not_intercept_hostile_targets() {
        let mut simulation = combat_simulation(2);
        simulation.world.units.truncate(3);
        let target_id = simulation.world.units[1].id;
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[2].owner = Some(PlayerId(1));
        simulation.world.units[0].state.position = Vec2::ZERO;
        simulation.world.units[1].state.position = Vec2::new(0.0, 8.0);
        simulation.world.units[2].state.position = Vec2::new(0.0, 4.0);

        let events = simulation.step().unwrap();

        assert!(events.iter().any(|event| matches!(
            event,
            SimulationEvent::ShotFired {
                shooter_id: UnitId(1),
                impact_entity: Some(ImpactEntityId::Unit(id)),
                ..
            } if *id == target_id
        )));
    }

    #[test]
    fn cross_entity_impacts_repeat_with_identical_events_and_hashes() {
        let mut left = shot_past_first_core(4.0);
        let mut right = shot_past_first_core(4.0);

        assert_eq!(left.step().unwrap(), right.step().unwrap());
        assert_eq!(left.state_hash(), right.state_hash());
    }
}
