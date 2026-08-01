use std::io;

use crate::command::{
    AuthoritativeCommand, CommandData, CommandRejected, CommandRejectionReason, CommandRequest,
};
use crate::error::invalid;
use crate::handshake::{
    Capability, ClientHello, HandshakeRejected, HandshakeRejectionReason, ServerHello,
};
use crate::message::Message;
use crate::session::{
    MatchTiming, OpponentPresence, PlayerColor, SessionParticipant, SessionSnapshot,
};
use crate::snapshot::{InitialWorldState, StateChecksum, WorldUnit};
use crate::tick::Tick;
use crate::wire;

fn caps(values: &[i32]) -> Vec<Capability> {
    values
        .iter()
        .filter_map(|value| match *value {
            1 => Some(Capability::StateChecksums),
            2 => Some(Capability::WorldSnapshots),
            _ => None,
        })
        .collect()
}

impl TryFrom<wire::command::Payload> for CommandData {
    type Error = io::Error;

    fn try_from(value: wire::command::Payload) -> Result<Self, Self::Error> {
        match value {
            wire::command::Payload::SetDestination(value) => {
                let destination = value
                    .destination
                    .ok_or_else(|| invalid("missing destination"))?;
                Ok(Self::SetDestination {
                    destination: [destination.x, destination.y],
                    target_unit_ids: value.target_unit_ids,
                })
            }
            wire::command::Payload::ResetSimulation(_) => Ok(Self::ResetSimulation),
        }
    }
}

impl From<wire::WorldUnit> for WorldUnit {
    fn from(value: wire::WorldUnit) -> Self {
        Self {
            id: value.id,
            owner: value.has_owner.then_some(value.owner),
            position_bits: [value.position_x, value.position_y],
            velocity_bits: [value.velocity_x, value.velocity_y],
            heading_bits: value.heading,
            angular_velocity_bits: value.angular_velocity,
            active: value.active,
            destination_bits: value
                .has_destination
                .then_some([value.destination_x, value.destination_y]),
            hull_current: value.hull_current,
            hull_maximum: value.hull_maximum,
            turret_local_heading_bits: value.turret_local_heading,
            turret_target: (value.turret_target_kind != 0)
                .then_some((value.turret_target_kind, value.turret_target_id)),
            turret_cooldown_ticks_remaining: value.turret_cooldown_ticks_remaining,
        }
    }
}

impl TryFrom<wire::Envelope> for Message {
    type Error = io::Error;

    fn try_from(value: wire::Envelope) -> Result<Self, Self::Error> {
        match value
            .payload
            .ok_or_else(|| invalid("missing envelope payload"))?
        {
            wire::envelope::Payload::ClientHello(value) => Ok(Message::ClientHello(ClientHello {
                simulation_version: value.simulation_version,
                capabilities: caps(&value.supported_capabilities),
                display_name: value.display_name,
            })),
            wire::envelope::Payload::ServerHello(value) => Ok(Message::ServerHello(ServerHello {
                simulation_version: value.simulation_version,
                simulation_hz: value.simulation_hz,
                player_slot: value.player_slot,
                server_tick: Tick::from(value.server_tick),
                fleet_size: value.fleet_size,
                world_radius_bits: value.world_radius_bits,
                capabilities: caps(&value.enabled_capabilities),
            })),
            wire::envelope::Payload::CommandRequest(value) => {
                Ok(Message::CommandRequest(CommandRequest {
                    sequence: value.sequence,
                    command: CommandData::try_from(
                        value
                            .command
                            .and_then(|command| command.payload)
                            .ok_or_else(|| invalid("missing command payload"))?,
                    )?,
                }))
            }
            wire::envelope::Payload::AuthoritativeCommand(value) => {
                Ok(Message::AuthoritativeCommand(AuthoritativeCommand {
                    execute_tick: Tick::from(value.execute_tick),
                    player_slot: value.player_slot,
                    sequence: value.sequence,
                    command: CommandData::try_from(
                        value
                            .command
                            .and_then(|command| command.payload)
                            .ok_or_else(|| invalid("missing command payload"))?,
                    )?,
                }))
            }
            wire::envelope::Payload::CommandRejected(value) => {
                let reason = match value.reason {
                    1 => CommandRejectionReason::InvalidPlayer,
                    2 => CommandRejectionReason::UnauthorizedFleet,
                    3 => CommandRejectionReason::NonFiniteDestination,
                    4 => CommandRejectionReason::DestinationOutsideArena,
                    5 => CommandRejectionReason::InvalidCommand,
                    _ => return Err(invalid("invalid command rejection reason")),
                };
                Ok(Message::CommandRejected(CommandRejected {
                    sequence: value.sequence,
                    reason,
                }))
            }
            wire::envelope::Payload::StateChecksum(value) => {
                Ok(Message::StateChecksum(StateChecksum {
                    tick: Tick::from(value.tick),
                    hash: value.hash,
                }))
            }
            wire::envelope::Payload::HandshakeRejected(value) => {
                let reason = match value.reason {
                    1 => HandshakeRejectionReason::ServerFull,
                    2 => HandshakeRejectionReason::IncompatibleVersion,
                    3 => HandshakeRejectionReason::MissingRequiredCapability,
                    4 => HandshakeRejectionReason::InvalidHandshake,
                    _ => return Err(invalid("invalid handshake rejection reason")),
                };
                Ok(Message::HandshakeRejected(HandshakeRejected { reason }))
            }
            wire::envelope::Payload::InitialWorldState(value) => {
                Ok(Message::InitialWorldState(InitialWorldState {
                    snapshot_format_version: value.snapshot_format_version,
                    simulation_version: value.simulation_version,
                    tick: Tick::from(value.tick),
                    world_radius_bits: value.world_radius_bits,
                    next_unit_id: value.next_unit_id,
                    unit_id_exhausted: value.unit_id_exhausted,
                    units: value.units.into_iter().map(Into::into).collect(),
                    state_hash: value.state_hash,
                    pending_commands: value
                        .pending_commands
                        .into_iter()
                        .map(|command| {
                            Ok(AuthoritativeCommand {
                                execute_tick: Tick::from(command.execute_tick),
                                player_slot: command.player_slot,
                                sequence: command.sequence,
                                command: CommandData::try_from(
                                    command
                                        .command
                                        .and_then(|command| command.payload)
                                        .ok_or_else(|| invalid("missing command payload"))?,
                                )?,
                            })
                        })
                        .collect::<Result<Vec<_>, io::Error>>()?,
                }))
            }
            wire::envelope::Payload::SessionSnapshot(value) => {
                let opponent_presence = match value.opponent_presence {
                    1 => OpponentPresence::Waiting,
                    2 => OpponentPresence::Present,
                    3 => OpponentPresence::Disconnected,
                    _ => return Err(invalid("invalid opponent presence")),
                };
                let match_timing = match value
                    .match_timing
                    .and_then(|timing| timing.state)
                    .ok_or_else(|| invalid("missing match timing"))?
                {
                    wire::match_timing::State::Inactive(_) => MatchTiming::Inactive,
                    wire::match_timing::State::Active(active) => MatchTiming::Active {
                        started_at_tick: Tick::from(active.started_at_tick),
                    },
                };
                let participants = value
                    .participants
                    .into_iter()
                    .map(|participant| {
                        let color = match participant.color {
                            1 => PlayerColor::Cyan,
                            2 => PlayerColor::Coral,
                            _ => return Err(invalid("invalid player color")),
                        };
                        Ok(SessionParticipant {
                            player_slot: participant.player_slot,
                            display_name: participant.display_name,
                            color,
                        })
                    })
                    .collect::<Result<Vec<_>, io::Error>>()?;
                let snapshot = SessionSnapshot {
                    local_player_slot: value.local_player_slot,
                    participants,
                    opponent_presence,
                    presence_revision: value.presence_revision,
                    match_timing,
                };
                snapshot.validate()?;
                Ok(Message::SessionSnapshot(snapshot))
            }
        }
    }
}
