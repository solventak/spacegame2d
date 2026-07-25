use prost::Message as ProstMessage;
use std::io::{self, Read, Write};

mod wire {
    include!(concat!(env!("OUT_DIR"), "/spacegame2d.protocol.v1.rs"));
}

pub type Tick = u64;
pub type PlayerSlot = u32;
pub const SIMULATION_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: u32 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    StateChecksums,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandData {
    SetDestination { unit_id: u32, destination: [u32; 2] },
    ResetSimulation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientHello {
    pub simulation_version: u32,
    pub capabilities: Vec<Capability>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerHello {
    pub simulation_version: u32,
    pub simulation_hz: u32,
    pub player_slot: u32,
    pub server_tick: Tick,
    pub capabilities: Vec<Capability>,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    ClientHello(ClientHello),
    ServerHello(ServerHello),
    CommandRequest(CommandRequest),
    AuthoritativeCommand(AuthoritativeCommand),
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn caps(values: &[i32]) -> Vec<Capability> {
    values
        .iter()
        .filter_map(|v| match *v {
            1 => Some(Capability::StateChecksums),
            _ => None,
        })
        .collect()
}
impl ClientHello {
    pub fn is_compatible(&self) -> bool {
        self.simulation_version == SIMULATION_VERSION
    }
}

impl TryFrom<wire::command::Payload> for CommandData {
    type Error = io::Error;

    fn try_from(value: wire::command::Payload) -> Result<Self, Self::Error> {
        match value {
            wire::command::Payload::SetDestination(v) => {
                let d = v
                    .destination
                    .ok_or_else(|| invalid("missing destination"))?;
                Ok(Self::SetDestination {
                    unit_id: v.unit_id,
                    destination: [d.x, d.y],
                })
            }
            wire::command::Payload::ResetSimulation(_) => Ok(Self::ResetSimulation),
        }
    }
}
impl From<&CommandData> for wire::Command {
    fn from(value: &CommandData) -> Self {
        let payload = match value {
            CommandData::SetDestination {
                unit_id,
                destination,
            } => wire::command::Payload::SetDestination(wire::SetDestinationCommand {
                unit_id: *unit_id,
                destination: Some(wire::Vector2Bits {
                    x: destination[0],
                    y: destination[1],
                }),
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
impl From<&Message> for wire::Envelope {
    fn from(message: &Message) -> Self {
        let payload = match message {
            Message::ClientHello(v) => wire::envelope::Payload::ClientHello(wire::ClientHello {
                simulation_version: v.simulation_version,
                supported_capabilities: v
                    .capabilities
                    .iter()
                    .map(|c| match c {
                        Capability::StateChecksums => 1,
                    })
                    .collect(),
            }),
            Message::ServerHello(v) => wire::envelope::Payload::ServerHello(wire::ServerHello {
                simulation_version: v.simulation_version,
                simulation_hz: v.simulation_hz,
                player_slot: v.player_slot,
                server_tick: v.server_tick,
                enabled_capabilities: v
                    .capabilities
                    .iter()
                    .map(|c| match c {
                        Capability::StateChecksums => 1,
                    })
                    .collect(),
            }),
            Message::CommandRequest(v) => {
                wire::envelope::Payload::CommandRequest(wire::CommandRequest {
                    sequence: v.sequence,
                    command: Some((&v.command).into()),
                })
            }
            Message::AuthoritativeCommand(v) => {
                wire::envelope::Payload::AuthoritativeCommand(wire::AuthoritativeCommand {
                    execute_tick: v.execute_tick,
                    player_slot: v.player_slot,
                    sequence: v.sequence,
                    command: Some((&v.command).into()),
                })
            }
        };
        Self {
            payload: Some(payload),
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
            wire::envelope::Payload::ClientHello(v) => Ok(Message::ClientHello(ClientHello {
                simulation_version: v.simulation_version,
                capabilities: caps(&v.supported_capabilities),
            })),
            wire::envelope::Payload::ServerHello(v) => Ok(Message::ServerHello(ServerHello {
                simulation_version: v.simulation_version,
                simulation_hz: v.simulation_hz,
                player_slot: v.player_slot,
                server_tick: v.server_tick,
                capabilities: caps(&v.enabled_capabilities),
            })),
            wire::envelope::Payload::CommandRequest(v) => {
                Ok(Message::CommandRequest(CommandRequest {
                    sequence: v.sequence,
                    command: CommandData::try_from(
                        v.command
                            .and_then(|c| c.payload)
                            .ok_or_else(|| invalid("missing command payload"))?,
                    )?,
                }))
            }
            wire::envelope::Payload::AuthoritativeCommand(v) => {
                Ok(Message::AuthoritativeCommand(AuthoritativeCommand {
                    execute_tick: v.execute_tick,
                    player_slot: v.player_slot,
                    sequence: v.sequence,
                    command: CommandData::try_from(
                        v.command
                            .and_then(|c| c.payload)
                            .ok_or_else(|| invalid("missing command payload"))?,
                    )?,
                }))
            }
        }
    }
}

impl Message {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut body = Vec::new();
        wire::Envelope::from(self)
            .encode(&mut body)
            .map_err(|e| invalid(&e.to_string()))?;
        if body.is_empty() || body.len() > MAX_FRAME_BYTES as usize {
            return Err(invalid("invalid frame size"));
        }
        let mut output = Vec::with_capacity(body.len() + 4);
        output.extend_from_slice(&(body.len() as u32).to_be_bytes());
        output.extend_from_slice(&body);
        Ok(output)
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.encode()?)
    }
}
pub fn read_message<R: Read>(reader: &mut R) -> io::Result<Message> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header);
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(invalid("invalid frame size"));
    }
    let mut body = vec![0; length as usize];
    reader.read_exact(&mut body)?;
    Message::try_from(wire::Envelope::decode(body.as_slice()).map_err(|e| invalid(&e.to_string()))?)
}

#[derive(Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}
impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, bytes: &[u8]) -> io::Result<Vec<Message>> {
        self.buffer.extend_from_slice(bytes);
        let mut messages = Vec::new();
        loop {
            if self.buffer.len() < 4 {
                break;
            }
            let length = u32::from_be_bytes(self.buffer[..4].try_into().unwrap());
            if length == 0 || length > MAX_FRAME_BYTES {
                return Err(invalid("invalid frame size"));
            }
            let total = 4 + length as usize;
            if self.buffer.len() < total {
                break;
            }
            let body = self.buffer[4..total].to_vec();
            self.buffer.drain(..total);
            messages.push(Message::try_from(
                wire::Envelope::decode(body.as_slice()).map_err(|e| invalid(&e.to_string()))?,
            )?);
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn destination() -> CommandData {
        CommandData::SetDestination {
            unit_id: 7,
            destination: [0x8000_0000, 0x0000_0001],
        }
    }
    #[test]
    fn all_messages_round_trip() {
        let messages = [
            Message::ClientHello(ClientHello {
                simulation_version: 1,
                capabilities: vec![Capability::StateChecksums],
            }),
            Message::ServerHello(ServerHello {
                simulation_version: 1,
                simulation_hz: 60,
                player_slot: 2,
                server_tick: 9,
                capabilities: vec![],
            }),
            Message::CommandRequest(CommandRequest {
                sequence: 4,
                command: destination(),
            }),
            Message::AuthoritativeCommand(AuthoritativeCommand {
                execute_tick: 11,
                player_slot: 2,
                sequence: 4,
                command: CommandData::ResetSimulation,
            }),
        ];
        for message in messages {
            let mut bytes = Vec::new();
            message.write(&mut bytes).unwrap();
            assert_eq!(read_message(&mut bytes.as_slice()).unwrap(), message);
        }
    }
    #[test]
    fn fragmented_and_multiple_frames_decode() {
        let message = Message::CommandRequest(CommandRequest {
            sequence: 1,
            command: destination(),
        });
        let bytes = message.encode().unwrap();
        let mut decoder = FrameDecoder::new();
        assert!(decoder.push(&bytes[..2]).unwrap().is_empty());
        assert_eq!(decoder.push(&bytes[2..]).unwrap(), vec![message.clone()]);
        assert_eq!(decoder.push(&bytes).unwrap(), vec![message]);
    }
    #[test]
    fn invalid_lengths_and_missing_payloads_rejected() {
        let mut decoder = FrameDecoder::new();
        assert_eq!(
            decoder.push(&[0, 0, 0, 0]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut oversized = (MAX_FRAME_BYTES + 1).to_be_bytes().to_vec();
        assert_eq!(
            decoder.push(&oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        oversized.clear();
        let empty = [0u8; 0];
        let mut frame = Vec::new();
        wire::Envelope::default().encode(&mut frame).unwrap();
        frame.extend_from_slice(&empty);
        let mut bytes = (frame.len() as u32).to_be_bytes().to_vec();
        bytes.extend(frame);
        assert_eq!(
            read_message(&mut bytes.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
    #[test]
    fn missing_command_payload_reaches_empty_command_branch() {
        // Frame: 4-byte length, Envelope field 3 (CommandRequest), sequence 1,
        // then Command field 2 with an empty oneof. This reaches the missing payload check.
        let bytes = [0, 0, 0, 6, 0x1a, 0x04, 0x08, 0x01, 0x12, 0x00];

        let error = read_message(&mut bytes.as_slice()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "missing command payload");
    }
    #[test]
    fn exact_float_bits_and_unknown_capabilities_survive() {
        let message = Message::CommandRequest(CommandRequest {
            sequence: 2,
            command: CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NAN.to_bits(), f32::INFINITY.to_bits()],
            },
        });
        let decoded = read_message(&mut message.encode().unwrap().as_slice()).unwrap();
        assert_eq!(decoded, message);
        let envelope = wire::Envelope {
            payload: Some(wire::envelope::Payload::ClientHello(wire::ClientHello {
                simulation_version: 1,
                supported_capabilities: vec![1, 999],
            })),
        };
        let mut body = Vec::new();
        envelope.encode(&mut body).unwrap();
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend(body);
        assert_eq!(
            read_message(&mut bytes.as_slice()).unwrap(),
            Message::ClientHello(ClientHello {
                simulation_version: 1,
                capabilities: vec![Capability::StateChecksums]
            })
        );
    }
}
