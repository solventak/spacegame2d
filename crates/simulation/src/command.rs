use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, CommandData};

use crate::autopilot::{Autopilot, AutopilotConfig};
use crate::flight_control::ArrivalController;
use crate::simulation::ShipState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);
impl PlayerId {
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

#[derive(Clone, Debug, PartialEq)]
pub enum RecordedCommand {
    SetDestination {
        unit_id: UnitId,
        destination: [u32; 2],
    },
    ResetSimulation,
}
pub trait Command: Send {
    fn execute(&self, world: &mut World);
    fn record(&self) -> RecordedCommand;
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
    fn record(&self) -> RecordedCommand {
        RecordedCommand::SetDestination {
            unit_id: self.unit_id,
            destination: [self.destination.x.to_bits(), self.destination.y.to_bits()],
        }
    }
}
pub struct ResetSimulation;
impl Command for ResetSimulation {
    fn execute(&self, world: &mut World) {
        *world = World::demo();
    }
    fn record(&self) -> RecordedCommand {
        RecordedCommand::ResetSimulation
    }
}
#[derive(Default)]
pub struct CommandScheduler {
    pending: Vec<Box<dyn Command>>,
    history: Vec<RecordedCommand>,
}
impl CommandScheduler {
    pub fn schedule(&mut self, command: Box<dyn Command>) {
        self.pending.push(command);
    }
    pub fn execute_pending(&mut self, world: &mut World) {
        let pending = std::mem::take(&mut self.pending);
        for c in pending {
            self.history.push(c.record());
            c.execute(world);
        }
    }
    pub fn history(&self) -> &[RecordedCommand] {
        &self.history
    }
    pub fn replay(history: &[RecordedCommand], world: &mut World) {
        for c in history {
            match c {
                RecordedCommand::SetDestination {
                    unit_id,
                    destination,
                } => SetDestination {
                    unit_id: *unit_id,
                    destination: Vec2::new(
                        f32::from_bits(destination[0]),
                        f32::from_bits(destination[1]),
                    ),
                }
                .execute(world),
                RecordedCommand::ResetSimulation => ResetSimulation.execute(world),
            }
        }
    }
}
pub fn command_from_data(data: &CommandData) -> Option<Box<dyn Command>> {
    match data {
        CommandData::SetDestination {
            unit_id,
            destination,
        } => {
            let d = [
                f32::from_bits(destination[0]),
                f32::from_bits(destination[1]),
            ];
            (d[0].is_finite() && d[1].is_finite()).then(|| {
                Box::new(SetDestination {
                    unit_id: UnitId(*unit_id),
                    destination: Vec2::from_array(d),
                }) as Box<dyn Command>
            })
        }
        CommandData::ResetSimulation => Some(Box::new(ResetSimulation)),
    }
}
pub fn valid_authoritative(world: &World, cmd: &AuthoritativeCommand) -> bool {
    let Some(player) = PlayerId::new(cmd.player_slot as u8) else {
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

    fn destination(slot: u32, unit_id: u32) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: 0,
            player_slot: slot,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id,
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        }
    }

    #[test]
    fn rejects_reserved_slot_and_unowned_units() {
        let world = World::demo();
        assert!(!valid_authoritative(&world, &destination(0, 1)));
        assert!(!valid_authoritative(&world, &destination(1, 1)));
    }

    #[test]
    fn accepts_owned_unit_and_records_replayable_command() {
        let mut world = World::demo();
        world.units[0].owner = Some(PlayerId(1));
        let cmd = destination(1, 1);
        assert!(valid_authoritative(&world, &cmd));
        let command = command_from_data(&cmd.command).unwrap();
        let record = command.record();
        command.execute(&mut world);
        assert_eq!(
            world.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );
        let mut replay = World::demo();
        CommandScheduler::replay(&[record], &mut replay);
        assert_eq!(
            replay.units[0].autopilot.destination(),
            Some(Vec2::new(1.0, 2.0))
        );
    }
}
