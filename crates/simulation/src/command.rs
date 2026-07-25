use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, CommandData, Tick};

use crate::autopilot::{Autopilot, AutopilotConfig};
use crate::flight_control::ArrivalController;
use crate::simulation::{ShipState, Simulation, SimulationEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

pub const FLEET_SIZE: usize = 30;
pub const MAX_PLAYERS: usize = 2;
pub const MAX_UNITS: usize = FLEET_SIZE * MAX_PLAYERS;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum PlayerIdError {
    #[error("player slot must be nonzero and fit in u8")]
    InvalidSlot,
}

impl TryFrom<u32> for PlayerId {
    type Error = PlayerIdError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let slot = u8::try_from(value).map_err(|_| PlayerIdError::InvalidSlot)?;
        Self::new(slot).ok_or(PlayerIdError::InvalidSlot)
    }
}

impl PlayerId {
    /// Valid player slots start at 1; slot 0 is reserved and never accepted.
    pub fn new(slot: u8) -> Option<Self> {
        (slot != 0).then_some(Self(slot))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnitId(pub u32);

impl From<u32> for UnitId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<UnitId> for u32 {
    fn from(value: UnitId) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum UnitIdAllocationError {
    #[error("unit id allocation exhausted")]
    Exhausted,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CommandDataError {
    #[error("destination coordinates must be finite")]
    NonFiniteDestination,
}

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
    connected_players: BTreeSet<PlayerId>,
    unit_id_exhausted: bool,
}

impl World {
    pub fn demo() -> Self {
        let units = crate::fleet::initial_world_positions()
            .into_iter()
            .enumerate()
            .map(|(i, state)| Unit::new(UnitId(i as u32 + 1), None, state))
            .collect();
        Self {
            next_unit_id: MAX_UNITS as u32 + 1,
            units,
            connected_players: BTreeSet::new(),
            unit_id_exhausted: false,
        }
    }

    pub fn connect_player(&mut self, player: PlayerId) -> bool {
        self.connected_players.insert(player)
    }

    /// Assign the player's deterministic fleet slice.
    pub fn assign_player_unit(&mut self, player: PlayerId) -> bool {
        self.assign_player_fleet(player)
    }

    pub fn assign_player_fleet(&mut self, player: PlayerId) -> bool {
        let slot = usize::from(player.0.saturating_sub(1));
        if slot >= MAX_PLAYERS || self.units.len() < MAX_UNITS {
            return false;
        }
        let range = slot * FLEET_SIZE..(slot + 1) * FLEET_SIZE;
        if self.units[range.clone()]
            .iter()
            .any(|unit| unit.owner.is_some_and(|owner| owner != player))
        {
            return false;
        }
        for unit in &mut self.units[range] {
            unit.owner = Some(player);
        }
        true
    }

    /// Assign deterministic owners to every unit in a client mirror.
    pub fn assign_mirror_owners(&mut self) {
        for (index, unit) in self.units.iter_mut().enumerate() {
            let slot = index / FLEET_SIZE;
            unit.owner = (slot < MAX_PLAYERS).then(|| PlayerId((slot + 1) as u8));
        }
    }

    pub fn disconnect_player(&mut self, player: PlayerId) -> bool {
        self.connected_players.remove(&player)
    }

    pub fn is_player_connected(&self, player: PlayerId) -> bool {
        self.connected_players.contains(&player)
    }

    pub fn allocate_unit_id(&mut self) -> Result<UnitId, UnitIdAllocationError> {
        if self.unit_id_exhausted {
            return Err(UnitIdAllocationError::Exhausted);
        }
        let id = UnitId(self.next_unit_id);
        if self.next_unit_id == u32::MAX {
            self.unit_id_exhausted = true;
        } else {
            self.next_unit_id += 1;
        }
        Ok(id)
    }

    fn advance_allocator_past_units(&mut self) {
        if let Some(max_id) = self.units.iter().map(|unit| unit.id.0).max() {
            if max_id == u32::MAX {
                self.unit_id_exhausted = true;
            } else {
                self.next_unit_id = self.next_unit_id.max(max_id + 1);
            }
        }
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }
    /// Validate an authoritative command against the current world state.
    ///
    /// Ownership rules apply to targeted units and connected players.
    pub fn validate_authoritative(
        &self,
        cmd: &AuthoritativeCommand,
    ) -> Result<(), AuthoritativeCommandError> {
        let player = PlayerId::try_from(cmd.player_slot)
            .map_err(|_| AuthoritativeCommandError::InvalidPlayerSlot)?;
        match &cmd.command {
            CommandData::SetDestination { unit_id, .. } => {
                let unit = self
                    .unit(UnitId::from(*unit_id))
                    .ok_or(AuthoritativeCommandError::UnknownUnit)?;
                if unit.owner != Some(player) {
                    return Err(AuthoritativeCommandError::NotOwner);
                }
                Ok(())
            }
            CommandData::ResetSimulation => {
                if self.is_player_connected(player) {
                    Ok(())
                } else {
                    Err(AuthoritativeCommandError::PlayerNotConnected)
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AuthoritativeCommandError {
    #[error("player slot is invalid")]
    InvalidPlayerSlot,
    #[error("unit does not exist")]
    UnknownUnit,
    #[error("player does not own unit")]
    NotOwner,
    #[error("player is not connected")]
    PlayerNotConnected,
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

#[derive(Debug, Error)]
pub enum CommandExecutionError {
    #[error(transparent)]
    UnitIdAllocation(#[from] UnitIdAllocationError),
}

pub trait Command: Send {
    fn execute(&self, world: &mut World) -> Result<(), CommandExecutionError>;
    fn record(&self, execute_tick: Tick) -> RecordedCommand;
}

pub struct SetDestination {
    pub unit_id: UnitId,
    pub destination: Vec2,
}

impl Command for SetDestination {
    fn execute(&self, world: &mut World) -> Result<(), CommandExecutionError> {
        if let Some(u) = world.unit_mut(self.unit_id) {
            u.autopilot.set_destination(self.destination);
        }
        Ok(())
    }

    fn record(&self, execute_tick: Tick) -> RecordedCommand {
        RecordedCommand::SetDestination {
            execute_tick,
            unit_id: self.unit_id,
            destination: [self.destination.x.to_bits(), self.destination.y.to_bits()],
        }
    }
}

/// Reset the world to its deterministic demo state and restore canonical fleet
/// ownership.
///
/// Ownership rule: any connected player (valid slot >= 1) may issue a reset.
/// The command is rejected for the reserved slot 0. The `next_unit_id` counter
/// is not reset, so future spawns receive fresh [`UnitId`]s and previously used
/// ids are never reused within a session.
pub struct ResetSimulation;

impl Command for ResetSimulation {
    fn execute(&self, world: &mut World) -> Result<(), CommandExecutionError> {
        world.advance_allocator_past_units();

        // Rebuild the demo swarm using the deterministic demo layout and fresh
        // IDs from the session allocator.
        let mut units = Vec::new();
        for (i, state) in crate::fleet::initial_world_positions()
            .into_iter()
            .enumerate()
        {
            let id = world.allocate_unit_id()?;
            let owner =
                (i / FLEET_SIZE < MAX_PLAYERS).then(|| PlayerId((i / FLEET_SIZE + 1) as u8));
            units.push(Unit::new(id, owner, state));
        }
        world.units = units;
        Ok(())
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
    pub fn execute_pending(
        &mut self,
        tick: Tick,
        world: &mut World,
    ) -> Result<(), CommandExecutionError> {
        if let Some(commands) = self.pending.remove(&tick) {
            for command in commands {
                command.execute(world)?;
                self.history.push(command.record(tick));
            }
        }
        Ok(())
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
    ) -> Result<Vec<SimulationEvent>, CommandExecutionError> {
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
                    let command: Box<dyn Command> = command.into();
                    simulation.commands.schedule(simulation.tick(), command);
                }
            }
            events.extend(simulation.step()?);
        }

        Ok(events)
    }
}

impl TryFrom<&CommandData> for Box<dyn Command> {
    type Error = CommandDataError;

    fn try_from(data: &CommandData) -> Result<Self, Self::Error> {
        match data {
            CommandData::SetDestination {
                unit_id,
                destination,
            } => {
                let coordinates = [
                    f32::from_bits(destination[0]),
                    f32::from_bits(destination[1]),
                ];
                if !coordinates[0].is_finite() || !coordinates[1].is_finite() {
                    return Err(CommandDataError::NonFiniteDestination);
                }
                Ok(Box::new(SetDestination {
                    unit_id: UnitId(*unit_id),
                    destination: Vec2::from_array(coordinates),
                }))
            }
            CommandData::ResetSimulation => Ok(Box::new(ResetSimulation)),
        }
    }
}

impl From<&RecordedCommand> for Box<dyn Command> {
    fn from(recorded: &RecordedCommand) -> Self {
        match recorded {
            RecordedCommand::SetDestination {
                unit_id,
                destination,
                ..
            } => Box::new(SetDestination {
                unit_id: *unit_id,
                destination: Vec2::from_array([
                    f32::from_bits(destination[0]),
                    f32::from_bits(destination[1]),
                ]),
            }),
            RecordedCommand::ResetSimulation { .. } => Box::new(ResetSimulation),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn destination(slot: u32, unit_id: u32, execute_tick: u64) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: Tick::from(execute_tick),
            player_slot: slot,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id,
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        }
    }

    fn reset(slot: u32, execute_tick: u64) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: Tick::from(execute_tick),
            player_slot: slot,
            sequence: 1,
            command: CommandData::ResetSimulation,
        }
    }

    #[test]
    fn rejects_reserved_slot_and_unowned_units() {
        let world = World::demo();
        assert!(world.validate_authoritative(&destination(0, 1, 0)).is_err());
        assert!(world.validate_authoritative(&destination(1, 1, 0)).is_err());
    }

    #[test]
    fn reset_requires_valid_player_slot() {
        let mut world = World::demo();
        world.connect_player(PlayerId(1));
        assert!(world.validate_authoritative(&reset(0, 0)).is_err());
        assert!(world.validate_authoritative(&reset(1, 0)).is_ok());
        assert!(world.validate_authoritative(&reset(2, 0)).is_err());
    }

    #[test]
    fn rejects_player_slots_that_do_not_fit_player_id() {
        let world = World::demo();
        assert!(
            world
                .validate_authoritative(&reset(u32::from(u8::MAX) + 1, 0))
                .is_err()
        );
    }

    #[test]
    fn accepts_owned_unit_and_records_replayable_command() {
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));
        let cmd = destination(1, 1, 0);
        assert!(world.validate_authoritative(&cmd).is_ok());
        let command: Box<dyn Command> = (&cmd.command).try_into().unwrap();
        let record = command.record(Tick::default());
        command.execute(&mut world).unwrap();
        assert_eq!(
            world.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );

        let mut replay = Simulation::default();
        let events = CommandScheduler::replay(&[record], &mut replay, Tick::default());
        assert_eq!(replay.tick(), Tick::new(1));
        assert_eq!(
            replay.world.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );
        assert!(events.unwrap().is_empty());
    }

    #[test]
    fn reset_preserves_ownership_and_unit_ids() {
        let mut world = World::demo();
        world.assign_player_fleet(PlayerId(1));
        world.assign_player_fleet(PlayerId(2));
        world.units[2].state.position = Vec2::new(100.0, 0.0);
        let next_before = world.next_unit_id;

        ResetSimulation.execute(&mut world).unwrap();

        assert_eq!(world.units.len(), MAX_UNITS);
        assert_eq!(world.units[0].owner, Some(PlayerId(1)));
        assert_eq!(world.units[1].owner, Some(PlayerId(1)));
        assert_eq!(world.units[29].owner, Some(PlayerId(1)));
        assert_eq!(world.units[30].owner, Some(PlayerId(2)));
        assert_eq!(world.units[59].owner, Some(PlayerId(2)));
        assert_ne!(world.units[0].id, UnitId(1));
        assert_ne!(world.units[1].id, UnitId(2));
        assert!(
            world.next_unit_id > next_before,
            "next_unit_id must advance past recreated units"
        );
    }

    #[test]
    fn connected_player_registry_controls_membership() {
        let mut world = World::demo();
        assert!(!world.is_player_connected(PlayerId(1)));
        assert!(world.connect_player(PlayerId(1)));
        assert!(!world.connect_player(PlayerId(1)));
        assert!(world.is_player_connected(PlayerId(1)));
        assert!(world.disconnect_player(PlayerId(1)));
        assert!(!world.disconnect_player(PlayerId(1)));
        assert!(!world.is_player_connected(PlayerId(1)));
    }

    #[test]
    fn mirror_ownership_is_deterministic_for_all_units() {
        let mut world = World::demo();

        world.assign_mirror_owners();

        assert_eq!(world.units[0].owner, Some(PlayerId(1)));
        assert_eq!(world.units[1].owner, Some(PlayerId(1)));
        assert_eq!(world.units[29].owner, Some(PlayerId(1)));
        assert_eq!(world.units[30].owner, Some(PlayerId(2)));
        assert_eq!(world.units[59].owner, Some(PlayerId(2)));
    }

    #[test]
    fn demo_has_two_deterministic_fleets_in_bounds() {
        let first = World::demo();
        let second = World::demo();
        assert_eq!(first.units.len(), MAX_UNITS);
        assert_eq!(
            first
                .units
                .iter()
                .map(|unit| unit.state)
                .collect::<Vec<_>>(),
            second
                .units
                .iter()
                .map(|unit| unit.state)
                .collect::<Vec<_>>()
        );
        let left = first.units[..FLEET_SIZE]
            .iter()
            .map(|unit| unit.state.position)
            .sum::<Vec2>()
            / FLEET_SIZE as f32;
        let right = first.units[FLEET_SIZE..]
            .iter()
            .map(|unit| unit.state.position)
            .sum::<Vec2>()
            / FLEET_SIZE as f32;
        assert!(left.distance(right) > 1.0);
        assert!(
            first
                .units
                .iter()
                .all(|unit| unit.state.position.length() <= crate::simulation::WORLD_RADIUS_M)
        );
    }

    #[test]
    fn trusted_authoritative_commands_apply_on_unowned_mirrors() {
        let mut sim = Simulation::default();
        let command = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id: 1,
                destination: [0.0f32.to_bits(), 10.0f32.to_bits()],
            },
        };

        assert!(sim.schedule_authoritative_trusted(&command));
        sim.step().unwrap();

        assert_eq!(
            sim.world.units[0].autopilot.destination(),
            Some(Vec2::new(0.0, 10.0))
        );
    }

    #[test]
    fn player_slot_assignment_is_deterministic() {
        let mut world = World::demo();

        assert!(world.assign_player_unit(PlayerId(2)));
        assert_eq!(world.units[30].owner, Some(PlayerId(2)));
        assert_eq!(world.units[0].owner, None);
    }

    #[test]
    fn unit_id_allocation_reports_exhaustion() {
        let mut world = World::demo();
        world.next_unit_id = u32::MAX;

        assert_eq!(world.allocate_unit_id().unwrap(), UnitId(u32::MAX));
        assert_eq!(
            world.allocate_unit_id(),
            Err(UnitIdAllocationError::Exhausted)
        );
    }

    #[test]
    fn reset_after_culling_allocates_ids_above_all_previous_ids() {
        let mut world = World::demo();
        let highest_before = world.units.iter().map(|unit| unit.id.0).max().unwrap();
        world.units.truncate(1);
        ResetSimulation.execute(&mut world).unwrap();

        assert!(world.units.iter().all(|unit| unit.id.0 > highest_before));
        let ids: BTreeSet<_> = world.units.iter().map(|unit| unit.id).collect();
        assert_eq!(ids.len(), world.units.len());
        assert!(world.next_unit_id > world.units.iter().map(|unit| unit.id.0).max().unwrap());
    }

    #[test]
    fn reset_after_culling_restores_canonical_fleet_ownership() {
        let mut world = World::demo();
        world.assign_player_fleet(PlayerId(1));
        world.assign_player_fleet(PlayerId(2));
        world.units.drain(0..5);

        ResetSimulation.execute(&mut world).unwrap();

        assert_eq!(world.units.len(), MAX_UNITS);
        assert!(
            world.units[..FLEET_SIZE]
                .iter()
                .all(|unit| unit.owner == Some(PlayerId(1)))
        );
        assert!(
            world.units[FLEET_SIZE..]
                .iter()
                .all(|unit| unit.owner == Some(PlayerId(2)))
        );
    }

    #[test]
    fn command_history_records_only_accepted_commands() {
        let mut scheduler = CommandScheduler::default();
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));

        scheduler.schedule(
            Tick::default(),
            Box::new(SetDestination {
                unit_id: UnitId(1),
                destination: Vec2::new(1.0, 2.0),
            }),
        );
        scheduler.schedule(
            Tick::default(),
            Box::new(SetDestination {
                unit_id: UnitId(2),
                destination: Vec2::new(3.0, 4.0),
            }),
        );
        scheduler
            .execute_pending(Tick::default(), &mut world)
            .unwrap();

        assert_eq!(scheduler.history().len(), 2);
        assert!(scheduler.history().iter().all(|r| matches!(
            r,
            RecordedCommand::SetDestination {
                unit_id: UnitId(1),
                ..
            }
        ) || r.execute_tick() == Tick::new(0)));
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
        .execute(&mut world)
        .unwrap();
        assert_eq!(world.units[0].state.position, pos_before);
        assert_eq!(world.units[0].state.velocity, vel_before);
    }

    #[test]
    fn invalid_command_data_rejects_nan_and_infinity() {
        assert!(
            Box::<dyn Command>::try_from(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NAN.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
        assert!(
            Box::<dyn Command>::try_from(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
        assert!(
            Box::<dyn Command>::try_from(&CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NEG_INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
    }
}
