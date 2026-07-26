use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, CommandData, Tick};

use crate::autopilot::{Autopilot, AutopilotConfig};
use crate::combat::CombatState;
use crate::config::{AvoidanceConfig, MAX_PLAYERS, SimulationConfig};
use crate::flight_control::ArrivalController;
use crate::hitbox::{Hitbox, HitboxShape, PositionedHitbox};
use crate::simulation::{ShipState, Simulation, SimulationEvent};
use crate::structure::{StaticStructure, initial_static_structures};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

pub const FLEET_SIZE: usize = crate::config::DEFAULT_FLEET_SIZE as usize;
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
    #[error("player slot is invalid")]
    InvalidPlayerSlot,
}

pub struct Unit {
    pub id: UnitId,
    pub owner: Option<PlayerId>,
    pub state: ShipState,
    pub autopilot: Autopilot,
    pub combat: CombatState,
    hitbox: Hitbox,
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
            combat: CombatState::new(),
            hitbox: Hitbox::default_ship(),
        }
    }

    pub const fn hitbox(&self) -> Hitbox {
        self.hitbox
    }

    pub fn positioned_hitbox(&self) -> PositionedHitbox {
        self.hitbox.positioned_at(self.state.position)
    }

    pub fn with_avoidance(
        id: UnitId,
        owner: Option<PlayerId>,
        state: ShipState,
        avoidance: AvoidanceConfig,
    ) -> Self {
        let controller = ArrivalController {
            config: crate::flight_control::arrival::ArrivalControllerConfig {
                avoidance,
                ..Default::default()
            },
        };
        Self {
            id,
            owner,
            state,
            autopilot: Autopilot::new(Box::new(controller), AutopilotConfig::default()),
            combat: CombatState::new(),
            hitbox: Hitbox::default_ship(),
        }
    }
}

pub struct World {
    config: SimulationConfig,
    pub next_unit_id: u32,
    pub units: Vec<Unit>,
    static_structures: Vec<StaticStructure>,
    connected_players: BTreeSet<PlayerId>,
    unit_id_exhausted: bool,
}

impl World {
    pub fn demo() -> Self {
        Self::new(SimulationConfig::default())
    }

    pub fn new(config: SimulationConfig) -> Self {
        let units = crate::fleet::initial_world_positions(&config)
            .into_iter()
            .enumerate()
            .map(|(i, state)| {
                Unit::with_avoidance(UnitId(i as u32 + 1), None, state, config.avoidance())
            })
            .collect();
        let next_unit_id = config.total_units() as u32 + 1;
        Self {
            config,
            next_unit_id,
            units,
            static_structures: initial_static_structures(),
            connected_players: BTreeSet::new(),
            unit_id_exhausted: false,
        }
    }

    pub fn config(&self) -> SimulationConfig {
        self.config.clone()
    }

    pub fn structures(&self) -> &[StaticStructure] {
        &self.static_structures
    }

    /// Project a requested destination out of a static structure hitbox.
    ///
    /// The first containing structure in stable identity order wins. Built-in
    /// structures do not overlap; the order keeps future definitions
    /// deterministic if that changes.
    pub fn project_destination(&self, requested: Vec2) -> Vec2 {
        for structure in &self.static_structures {
            let HitboxShape::Circle(circle) = structure.hitbox().shape();
            let offset = requested - structure.position();
            let radius = circle.radius_meters();
            if offset.length_squared() < radius * radius {
                let direction = if offset == Vec2::ZERO {
                    Vec2::X
                } else {
                    offset.normalize()
                };
                return structure.position() + direction * radius;
            }
        }
        requested
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
        let fleet_size = self.config.fleet_size() as usize;
        if slot >= MAX_PLAYERS || self.units.len() < self.config.total_units() {
            return false;
        }
        let range = slot * fleet_size..(slot + 1) * fleet_size;
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
            let slot = index / self.config.fleet_size() as usize;
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
    pub(crate) fn allocator_state(&self) -> (u32, bool) {
        (self.next_unit_id, self.unit_id_exhausted)
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
            CommandData::SetDestination { .. } => {
                if !self.units.iter().any(|unit| unit.owner == Some(player)) {
                    return Err(AuthoritativeCommandError::NoOwnedFleet);
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
    #[error("player owns no fleet units")]
    NoOwnedFleet,
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
        player: PlayerId,
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
    pub player: PlayerId,
    pub destination: Vec2,
}

impl Command for SetDestination {
    fn execute(&self, world: &mut World) -> Result<(), CommandExecutionError> {
        let destination = world.project_destination(self.destination);
        let has_owners = world.units.iter().any(|unit| unit.owner.is_some());
        for unit in &mut world.units {
            if !has_owners || unit.owner == Some(self.player) {
                unit.autopilot.set_destination(destination);
            }
        }
        Ok(())
    }

    fn record(&self, execute_tick: Tick) -> RecordedCommand {
        RecordedCommand::SetDestination {
            execute_tick,
            player: self.player,
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
        let config = world.config.clone();
        for (i, state) in crate::fleet::initial_world_positions(&config)
            .into_iter()
            .enumerate()
        {
            let id = world.allocate_unit_id()?;
            let owner = ((i / config.fleet_size() as usize) < MAX_PLAYERS)
                .then(|| PlayerId((i / config.fleet_size() as usize + 1) as u8));
            units.push(Unit::with_avoidance(id, owner, state, config.avoidance()));
        }
        world.units = units;
        world.static_structures = initial_static_structures();
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

    pub fn clear_pending(&mut self) {
        self.pending.clear();
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
        let command = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 0,
            command: data.clone(),
        };
        Box::<dyn Command>::try_from(&command)
    }
}

impl TryFrom<&AuthoritativeCommand> for Box<dyn Command> {
    type Error = CommandDataError;

    fn try_from(cmd: &AuthoritativeCommand) -> Result<Self, Self::Error> {
        let player =
            PlayerId::try_from(cmd.player_slot).map_err(|_| CommandDataError::InvalidPlayerSlot)?;
        match &cmd.command {
            CommandData::SetDestination { destination } => {
                let coordinates = [
                    f32::from_bits(destination[0]),
                    f32::from_bits(destination[1]),
                ];
                if !coordinates[0].is_finite() || !coordinates[1].is_finite() {
                    return Err(CommandDataError::NonFiniteDestination);
                }
                Ok(Box::new(SetDestination {
                    player,
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
                player,
                destination,
                ..
            } => Box::new(SetDestination {
                player: *player,
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

    #[test]
    fn unit_hitbox_center_follows_ship_position() {
        let mut unit = Unit::new(UnitId(1), None, ShipState::default());
        assert_eq!(unit.positioned_hitbox().center(), Vec2::ZERO);
        unit.state.position = Vec2::new(3.0, -2.0);
        assert_eq!(unit.positioned_hitbox().center(), unit.state.position);
        assert_eq!(unit.hitbox(), Hitbox::default_ship());
    }

    fn destination(slot: u32, _unit_id: u32, execute_tick: u64) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: Tick::from(execute_tick),
            player_slot: slot,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [5.0f32.to_bits(), 6.0f32.to_bits()],
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
            Some(Vec2::new(5.0, 6.0))
        );

        let mut replay = Simulation::default();
        let events = CommandScheduler::replay(&[record], &mut replay, Tick::default());
        assert_eq!(replay.tick(), Tick::new(1));
        assert_eq!(
            replay.world.units[0].autopilot.destination(),
            Some(Vec2::new(5.0, 6.0))
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
    fn reset_restores_default_combat_state() {
        use crate::combat::MAX_HULL;
        let mut world = World::demo();
        let target = world.units[1].id;
        let unit = &mut world.units[0];
        unit.combat.hull.current = 1;
        unit.combat.turret.local_heading_radians = 1.5;
        unit.combat.turret.target = Some(target);
        unit.combat.turret.cooldown_ticks_remaining = 7;
        ResetSimulation.execute(&mut world).unwrap();
        let reset = &world.units[0];
        assert_eq!(reset.combat.hull.current, MAX_HULL);
        assert_eq!(reset.combat.hull.maximum, MAX_HULL);
        assert_eq!(reset.combat.turret.local_heading_radians, 0.0);
        assert_eq!(reset.combat.turret.target, None);
        assert_eq!(reset.combat.turret.cooldown_ticks_remaining, 0);
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
                destination: [0.0f32.to_bits(), 15.0f32.to_bits()],
            },
        };

        assert!(sim.schedule_authoritative_trusted(&command));
        sim.step().unwrap();

        assert_eq!(
            sim.world.units[0].autopilot.destination(),
            Some(Vec2::new(0.0, 15.0))
        );
    }

    #[test]
    fn structures_are_recreated_deterministically_on_reset() {
        let mut world = World::demo();
        let initial = world.structures().to_vec();

        ResetSimulation.execute(&mut world).unwrap();

        assert_eq!(world.structures(), initial);
    }

    #[test]
    fn projection_uses_structure_hitboxes_with_a_positive_x_center_fallback() {
        let world = World::demo();

        assert_eq!(world.project_destination(Vec2::ZERO), Vec2::new(3.85, 0.0));
        assert_eq!(
            world.project_destination(Vec2::new(0.0, 10.0)),
            Vec2::new(2.75, 10.0)
        );
        assert_eq!(
            world.project_destination(Vec2::new(-1.0, 0.0)),
            Vec2::new(-3.85, 0.0)
        );
        assert_eq!(
            world.project_destination(Vec2::new(3.85, 0.0)),
            Vec2::new(3.85, 0.0)
        );
        assert_eq!(
            world.project_destination(Vec2::new(5.0, 6.0)),
            Vec2::new(5.0, 6.0)
        );
    }

    #[test]
    fn destination_commands_share_one_projected_point_but_record_raw_input() {
        let mut world = World::demo();
        world.assign_player_fleet(PlayerId(1));
        let requested = Vec2::new(0.0, 10.0);
        let command = SetDestination {
            player: PlayerId(1),
            destination: requested,
        };

        command.execute(&mut world).unwrap();

        assert!(
            world.units[..FLEET_SIZE]
                .iter()
                .all(|unit| unit.autopilot.destination() == Some(Vec2::new(2.75, 10.0)))
        );
        assert!(
            world.units[FLEET_SIZE..]
                .iter()
                .all(|unit| unit.autopilot.destination().is_none())
        );
        assert_eq!(
            command.record(Tick::new(7)),
            RecordedCommand::SetDestination {
                execute_tick: Tick::new(7),
                player: PlayerId(1),
                destination: [requested.x.to_bits(), requested.y.to_bits()],
            }
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
                player: PlayerId(1),
                destination: Vec2::new(1.0, 2.0),
            }),
        );
        scheduler.schedule(
            Tick::default(),
            Box::new(SetDestination {
                player: PlayerId(1),
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
                player: PlayerId(1),
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
            player: PlayerId(1),
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
                destination: [f32::NAN.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
        assert!(
            Box::<dyn Command>::try_from(&CommandData::SetDestination {
                destination: [f32::INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
        assert!(
            Box::<dyn Command>::try_from(&CommandData::SetDestination {
                destination: [f32::NEG_INFINITY.to_bits(), 0.0f32.to_bits()],
            })
            .is_err()
        );
    }
}
