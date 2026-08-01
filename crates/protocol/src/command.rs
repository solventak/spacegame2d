use crate::tick::Tick;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandData {
    SetDestination {
        destination: [u32; 2],
        target_unit_ids: Vec<u32>,
    },
    ResetSimulation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRequest {
    pub sequence: u32,
    pub command: CommandData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoritativeCommand {
    pub execute_tick: Tick,
    pub player_slot: u32,
    pub sequence: u32,
    pub command: CommandData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRejectionReason {
    InvalidPlayer,
    UnauthorizedFleet,
    NonFiniteDestination,
    DestinationOutsideArena,
    InvalidCommand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRejected {
    pub sequence: u32,
    pub reason: CommandRejectionReason,
}
