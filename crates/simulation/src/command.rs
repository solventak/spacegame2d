use std::collections::{BTreeMap, HashMap};

use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, CommandData, Tick};

use crate::autopilot::{Autopilot, AutopilotConfig};
use crate::flight_control::ArrivalController;
use crate::simulation::{ShipState, Simulation, SimulationEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

impl PlayerId {
    /// Valid player slots start at 1; slot 0 is reserved and never accepted.
    pub fn new(slot: u8) -> Option<Self> {
        (slot != 0).then_some(Self(slot))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnitId(pub u32);

pub struct Unit {
    pub id: UnitId,
    pub owner: Option<PlayerId>,
    pub state: ShipState,
    pub autopilot: Autopilot,
}

impl Unit {
    pub fn new(id: UnitId, owner: Option<PlayerId>, state: ShipState) -> Self {
        Self {
            id,
            owner,
            state,
            autopilot: Autopilot::new(
                Box::new(ArrivalController::default()),
                AutopilotConfig::default(),
            ),
        }
    }
}

pub struct World {
    pub next_unit_id: u32,
    pub units: Vec<Unit>,
}

impl World {
    pub fn demo() -> Self {
        let units = crate::fleet::initial_drone_positions()
            .into_iter()
            .enumerate()
            .map(|(i, state)| Unit::new(UnitId(i as u32 + 1), None, state))
            .collect();
        Self {
            next_unit_id: crate::fleet::DRONE_COUNT as u32 + 1,
            units,
        }
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }
}

/// A command that has been accepted and recorded in the scheduler's history.
///
/// Each recorded variant carries the [`Tick`] at which it was executed so that
/// replay can apply commands at the correct ticks and reproduce multi-tick
/// command sequences.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordedCommand {
    SetDestination {
        execute_tick: Tick,
        unit_id: UnitId,
        destination: [u32; 2],
    },
    ResetSimulation {
        execute_tick: Tick,
    },
}

impl RecordedCommand {
    /// The tick at which the recorded command was originally executed.
    pub fn execute_tick(&self) -> Tick {
        match self {
            RecordedCommand::SetDestination { execute_tick, .. } => *execute_tick,
            RecordedCommand::ResetSimulation { execute_tick } => *execute_tick,
        }
    }
}

pub trait Command: Send {
    fn execute(&self, world: &mut World);
    fn record(&self, execute_tick: Tick) -> RecordedCommand;
}

pub struct SetDestination {
    pub unit_id: UnitId,
    pub destination: Vec2,
}

impl Command for SetDestination {
    fn execute(&self, world: &mut World) {
        if let Some(u) = world.unit_mut(self.unit_id) {
            u.autopilot.set_destination(self.destination);
        }
    }

    fn record(&self, execute_tick: Tick) -> RecordedCommand {
        RecordedCommand::SetDestination {
            execute_tick,
            unit_id: self.unit_id,
            destination: [self.destination.x.to_bits(), self.destination.y.to_bits()],
        }
    }
}

/// Reset the world to its deterministic demo state while preserving player
/// ownership of surviving units.
///
/// Ownership rule: any connected player (valid slot >= 1) may issue a reset.
/// The command is rejected for the reserved slot 0. After a reset, each
/// surviving unit keeps its [`UnitId`] and owner. The `next_unit_id` counter
/// is not reset, so future spawns receive fresh [`UnitId`]s and previously
/// used ids are never reused within a session.
pub struct ResetSimulation;

impl Command for ResetSimulation {
    fn execute(&self, world: &mut World) {
        // Capture ownership by current UnitId so surviving units keep their
        // owners after the reset.
        let owners: HashMap<UnitId, PlayerId> = world
            .units
            .iter()
            .filter_map(|u| u.owner.map(|owner| (u.id, owner)))
            .collect();

        // Rebuild the demo swarm using the deterministic demo layout and the
        // original UnitId range. Ownership is preserved for units that still
        // exist; the next_unit_id counter is intentionally left unchanged.
        world.units = crate::fleet::initial_drone_positions()
            .into_iter()
            .enumerate()
            .map(|(i, state)| {
                let id = UnitId(i as u32 + 1);
                Unit::new(id, owners.get(&id).copied(), state)
            })
            .collect();
    }

    fn record(&self, execute_tick: Tick) -> RecordedCommand {
        RecordedCommand::ResetSimulation { execute_tick }
    }
}

#[derive(Default)]
pub struct CommandScheduler {
    pending: BTreeMap<Tick, Vec<Box<dyn Command>>>,
    history: Vec<RecordedCommand>,
}

impl CommandScheduler {
    /// Schedule a command to execute at the given tick.
    pub fn schedule(&mut self, tick: Tick, command: Box<dyn Command>) {
        self.pending.entry(tick).or_default().push(command);
    }

    /// Execute all commands pending for `tick`, recording each one in
    /// `history` before executing.
    pub fn execute_pending(&mut self, tick: Tick, world: &mut World) {
        if let Some(commands) = self.pending.remove(&tick) {
            for command in commands {
                self.history.push(command.record(tick));
                command.execute(world);
            }
        }
    }

    pub fn history(&self) -> &[RecordedCommand] {
        &self.history
    }

    /// Replay a recorded history on `simulation`, advancing from the
    /// simulation's current tick through `end_tick` and applying commands at
    /// the tick recorded in each [`RecordedCommand`].
    ///
    /// Returns the deterministic event vector produced across the replay.
    pub fn replay(
        history: &[RecordedCommand],
        simulation: &mut Simulation,
        end_tick: Tick,
    ) -> Vec<SimulationEvent> {
        // Drop any stale scheduler state so replay starts from a clean slate.
        simulation.commands = CommandScheduler::default();

        let mut by_tick: BTreeMap<Tick, Vec<RecordedCommand>> = BTreeMap::new();
        for command in history {
            by_tick
                .entry(command.execute_tick())
                .or_default()
                .push(command.clone());
        }

        let mut events = Vec::new();
        while simulation.tick() <= end_tick {
            if let Some(commands) = by_tick.get(&simulation.tick()) {
                for command in commands {
                    if let Some(command) = command_from_record(command) {
                        simulation.commands.schedule(simulation.tick(), command);
                    }
                }
            }
            events.extend(simulation.step());
        }

        events
    }
}

fn set_destination_command(unit_id: UnitId, destination: [u32; 2]) -> Option<Box<dyn Command>> {
    let d = [
        f32::from_bits(destination[0]),
        f32::from_bits(destination[1]),
    ];
    (d[0].is_finite() && d[1].is_finite()).then(|| {
        Box::new(SetDestination {
            unit_id,
            destination: Vec2::from_array(d),
        }) as Box<dyn Command>
    })
}

pub fn command_from_data(data: &CommandData) -> Option<Box<dyn Command>> {
    match data {
        CommandData::SetDestination {
            unit_id,
            destination,
        } => set_destination_command(UnitId(*unit_id), *destination),
        CommandData::ResetSimulation => Some(Box::new(ResetSimulation)),
    }
}

fn command_from_record(recorded: &RecordedCommand) -> Option<Box<dyn Command>> {
    match recorded {
        RecordedCommand::SetDestination {
            execute_tick: _,
            unit_id,
            destination,
        } => set_destination_command(*unit_id, *destination),
        RecordedCommand::ResetSimulation { execute_tick: _ } => Some(Box::new(ResetSimulation)),
    }
}

/// Validate an authoritative command against the current world state.
///
/// Ownership rules:
/// - `SetDestination` is accepted only when the targeted unit is owned by the
///   requesting player.
/// - `ResetSimulation` is accepted from any connected player (slot >= 1) and
///   rejected for the reserved slot 0.
pub fn valid_authoritative(world: &World, cmd: &AuthoritativeCommand) -> bool {
    let Ok(slot) = u8::try_from(cmd.player_slot) else {
        return false;
    };
    let Some(player) = PlayerId::new(slot) else {
        return false;
    };
    match &cmd.command {
        CommandData::SetDestination { unit_id, .. } => world
            .unit(UnitId(*unit_id))
            .is_some_and(|u| u.owner == Some(player)),
        CommandData::ResetSimulation => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destination(slot: u32, unit_id: u32, execute_tick: Tick) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick,
            player_slot: slot,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id,
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        }
    }

    fn reset(slot: u32, execute_tick: Tick) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick,
            player_slot: slot,
            sequence: 1,
            command: CommandData::ResetSimulation,
        }
    }

    #[test]
    fn rejects_reserved_slot_and_unowned_units() {
        let world = World::demo();
        assert!(!valid_authoritative(&world, &destination(0, 1, 0)));
        assert!(!valid_authoritative(&world, &destination(1, 1, 0)));
    }

    #[test]
    fn reset_requires_valid_player_slot() {
        let world = World::demo();
        assert!(!valid_authoritative(&world, &reset(0, 0)));
        assert!(valid_authoritative(&world, &reset(1, 0)));
    }

    #[test]
    fn rejects_player_slots_that_do_not_fit_player_id() {
        let world = World::demo();
        assert!(!valid_authoritative(
            &world,
            &reset(u32::from(u8::MAX) + 1, 0)
        ));
    }

    #[test]
    fn accepts_owned_unit_and_records_replayable_command() {
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));
        let cmd = destination(1, 1, 0);
        assert!(valid_authoritative(&world, &cmd));
        let command = command_from_data(&cmd.command).unwrap();
        let record = command.record(0);
        command.execute(&mut world);
        assert_eq!(
            world.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );

        let mut replay = Simulation::default();
        let events = CommandScheduler::replay(&[record], &mut replay, 0);
        assert_eq!(replay.tick(), 1);
        assert_eq!(
            replay.world.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );
        assert!(events.is_empty());
    }

    #[test]
    fn reset_preserves_ownership_and_unit_ids() {
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));
        world.units[1].owner = Some(PlayerId(2));
        world.units[2].state.position = Vec2::new(100.0, 0.0);
        let next_before = world.next_unit_id;

        ResetSimulation.execute(&mut world);

        assert_eq!(world.units.len(), crate::fleet::DRONE_COUNT);
        assert_eq!(world.units[0].owner, Some(PlayerId(1)));
        assert_eq!(world.units[1].owner, Some(PlayerId(2)));
        assert_eq!(world.units[2].owner, None);
        assert_eq!(world.units[0].id, UnitId(1));
        assert_eq!(world.units[1].id, UnitId(2));
        assert_eq!(
            world.next_unit_id, next_before,
            "next_unit_id must not be reset"
        );
    }

    #[test]
    fn command_history_records_only_accepted_commands() {
        let mut scheduler = CommandScheduler::default();
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));

        scheduler.schedule(
            0,
            Box::new(SetDestination {
                unit_id: UnitId(1),
                destination: Vec2::new(1.0, 2.0),
            }),
        );
        scheduler.schedule(
            0,
            Box::new(SetDestination {
                unit_id: UnitId(2),
                destination: Vec2::new(3.0, 4.0),
            }),
        );
        scheduler.execute_pending(0, &mut world);

        assert_eq!(scheduler.history().len(), 2);
        assert!(scheduler.history().iter().all(|r| matches!(
            r,
            RecordedCommand::SetDestination {
                unit_id: UnitId(1),
                ..
            }
        ) || r.execute_tick() == 0));
    }

    #[test]
    fn set_destination_does_not_teleport() {
        let mut world = World::demo();
        let pos_before = world.units[0].state.position;
        let vel_before = world.units[0].state.velocity;
        SetDestination {
            unit_id: UnitId(1),
            destination: Vec2::new(100.0, 100.0),
        }
        .execute(&mut world);
        assert_eq!(world.units[0].state.position, pos_before);
        assert_eq!(world.units[0].state.velocity, vel_before);
    }

    #[test]
    fn invalid_command_data_rejects_nan_and_infinity() {
        assert!(
            command_from_data(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NAN.to_bits(), 0.0f32.to_bits()],
            })
            .is_none()
        );
        assert!(
            command_from_data(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_none()
        );
        assert!(
            command_from_data(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NEG_INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_none()
        );
    }
}
