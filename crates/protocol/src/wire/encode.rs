use crate::command::{CommandData, CommandRejectionReason};
use crate::handshake::{Capability, HandshakeRejectionReason};
use crate::message::Message;
use crate::session::{MatchTiming, OpponentPresence, PlayerColor};
use crate::snapshot::WorldUnit;
use crate::wire;

impl From<&CommandData> for wire::Command {
    fn from(value: &CommandData) -> Self {
        let payload = match value {
            CommandData::SetDestination {
                destination,
                target_unit_ids,
            } => wire::command::Payload::SetDestination(wire::SetDestinationCommand {
                destination: Some(wire::Vector2Bits {
                    x: destination[0],
                    y: destination[1],
                }),
                target_unit_ids: target_unit_ids.clone(),
            }),
            CommandData::ResetSimulation => {
                wire::command::Payload::ResetSimulation(wire::ResetSimulationCommand {})
            }
        };
        Self {
            payload: Some(payload),
        }
    }
}

impl From<&WorldUnit> for wire::WorldUnit {
    fn from(value: &WorldUnit) -> Self {
        Self {
            id: value.id,
            owner: value.owner.unwrap_or_default(),
            has_owner: value.owner.is_some(),
            position_x: value.position_bits[0],
            position_y: value.position_bits[1],
            velocity_x: value.velocity_bits[0],
            velocity_y: value.velocity_bits[1],
            heading: value.heading_bits,
            angular_velocity: value.angular_velocity_bits,
            active: value.active,
            has_destination: value.destination_bits.is_some(),
            destination_x: value.destination_bits.map_or(0, |value| value[0]),
            destination_y: value.destination_bits.map_or(0, |value| value[1]),
            hull_current: value.hull_current,
            hull_maximum: value.hull_maximum,
            turret_local_heading: value.turret_local_heading_bits,
            turret_target_kind: value.turret_target.map_or(0, |value| value.0),
            turret_target_id: value.turret_target.map_or(0, |value| value.1),
            turret_cooldown_ticks_remaining: value.turret_cooldown_ticks_remaining,
        }
    }
}

impl From<&Message> for wire::Envelope {
    fn from(message: &Message) -> Self {
        let payload = match message {
            Message::ClientHello(value) => {
                wire::envelope::Payload::ClientHello(wire::ClientHello {
                    simulation_version: value.simulation_version,
                    supported_capabilities: value
                        .capabilities
                        .iter()
                        .map(|capability| match capability {
                            Capability::StateChecksums => 1,
                            Capability::WorldSnapshots => 2,
                        })
                        .collect(),
                    display_name: value.display_name.clone(),
                })
            }
            Message::ServerHello(value) => {
                wire::envelope::Payload::ServerHello(wire::ServerHello {
                    simulation_version: value.simulation_version,
                    simulation_hz: value.simulation_hz,
                    player_slot: value.player_slot,
                    server_tick: value.server_tick.0,
                    fleet_size: value.fleet_size,
                    world_radius_bits: value.world_radius_bits,
                    enabled_capabilities: value
                        .capabilities
                        .iter()
                        .map(|capability| match capability {
                            Capability::StateChecksums => 1,
                            Capability::WorldSnapshots => 2,
                        })
                        .collect(),
                })
            }
            Message::CommandRequest(value) => {
                wire::envelope::Payload::CommandRequest(wire::CommandRequest {
                    sequence: value.sequence,
                    command: Some((&value.command).into()),
                })
            }
            Message::AuthoritativeCommand(value) => {
                wire::envelope::Payload::AuthoritativeCommand(wire::AuthoritativeCommand {
                    execute_tick: value.execute_tick.0,
                    player_slot: value.player_slot,
                    sequence: value.sequence,
                    command: Some((&value.command).into()),
                })
            }
            Message::CommandRejected(value) => {
                wire::envelope::Payload::CommandRejected(wire::CommandRejected {
                    sequence: value.sequence,
                    reason: match value.reason {
                        CommandRejectionReason::InvalidPlayer => 1,
                        CommandRejectionReason::UnauthorizedFleet => 2,
                        CommandRejectionReason::NonFiniteDestination => 3,
                        CommandRejectionReason::DestinationOutsideArena => 4,
                        CommandRejectionReason::InvalidCommand => 5,
                    },
                })
            }
            Message::StateChecksum(value) => {
                wire::envelope::Payload::StateChecksum(wire::StateChecksum {
                    tick: value.tick.0,
                    hash: value.hash.clone(),
                })
            }
            Message::HandshakeRejected(value) => {
                wire::envelope::Payload::HandshakeRejected(wire::HandshakeRejected {
                    reason: match value.reason {
                        HandshakeRejectionReason::ServerFull => 1,
                        HandshakeRejectionReason::IncompatibleVersion => 2,
                        HandshakeRejectionReason::MissingRequiredCapability => 3,
                        HandshakeRejectionReason::InvalidHandshake => 4,
                    },
                })
            }
            Message::InitialWorldState(value) => {
                wire::envelope::Payload::InitialWorldState(wire::InitialWorldState {
                    snapshot_format_version: value.snapshot_format_version,
                    simulation_version: value.simulation_version,
                    tick: value.tick.0,
                    world_radius_bits: value.world_radius_bits,
                    next_unit_id: value.next_unit_id,
                    unit_id_exhausted: value.unit_id_exhausted,
                    units: value.units.iter().map(Into::into).collect(),
                    state_hash: value.state_hash.clone(),
                    pending_commands: value
                        .pending_commands
                        .iter()
                        .map(|command| wire::AuthoritativeCommand {
                            execute_tick: command.execute_tick.0,
                            player_slot: command.player_slot,
                            sequence: command.sequence,
                            command: Some((&command.command).into()),
                        })
                        .collect(),
                })
            }
            Message::SessionSnapshot(value) => {
                let match_timing = match &value.match_timing {
                    MatchTiming::Inactive => {
                        wire::match_timing::State::Inactive(wire::MatchTimingInactive {})
                    }
                    MatchTiming::Active { started_at_tick } => {
                        wire::match_timing::State::Active(wire::ActiveMatchTiming {
                            started_at_tick: started_at_tick.0,
                        })
                    }
                };
                wire::envelope::Payload::SessionSnapshot(wire::SessionSnapshot {
                    local_player_slot: value.local_player_slot,
                    participants: value
                        .participants
                        .iter()
                        .map(|participant| wire::SessionParticipant {
                            player_slot: participant.player_slot,
                            display_name: participant.display_name.clone(),
                            color: match participant.color {
                                PlayerColor::Cyan => 1,
                                PlayerColor::Coral => 2,
                            },
                        })
                        .collect(),
                    opponent_presence: match value.opponent_presence {
                        OpponentPresence::Waiting => 1,
                        OpponentPresence::Present => 2,
                        OpponentPresence::Disconnected => 3,
                    },
                    presence_revision: value.presence_revision,
                    match_timing: Some(wire::MatchTiming {
                        state: Some(match_timing),
                    }),
                })
            }
        };
        Self {
            payload: Some(payload),
        }
    }
}
