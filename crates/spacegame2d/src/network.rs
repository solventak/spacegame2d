use std::{
    collections::BTreeMap,
    io::{self, Read},
    net::TcpStream,
};

use spacegame2d_protocol::{
    AuthoritativeCommand, ClientHello, CommandData, CommandRequest, Message, SIMULATION_VERSION,
    ServerHello,
};
use spacegame2d_simulation::simulation::SIMULATION_HZ;

pub struct NetworkSession {
    stream: TcpStream,
    pub player_slot: u32,
    pub server_tick: u64,
    decoder: spacegame2d_protocol::FrameDecoder,
}

impl NetworkSession {
    pub fn connect(address: &str) -> io::Result<Self> {
        let mut stream = TcpStream::connect(address)?;
        stream.set_nodelay(true)?;
        spacegame2d_protocol::write_message(
            &mut stream,
            &Message::ClientHello(ClientHello {
                simulation_version: SIMULATION_VERSION,
                capabilities: Vec::new(),
            }),
        )?;
        let hello = match spacegame2d_protocol::read_message(&mut stream)? {
            Message::ServerHello(value) => value,
            _ => return Err(invalid("expected ServerHello")),
        };
        validate_server_hello(&hello)?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            player_slot: hello.player_slot,
            server_tick: hello.server_tick,
            decoder: spacegame2d_protocol::FrameDecoder::new(),
        })
    }

    pub fn is_connected(&self) -> bool {
        true
    }

    pub fn send_set_destination(
        &mut self,
        sequence: u32,
        unit_id: u32,
        destination: [u32; 2],
    ) -> io::Result<()> {
        self.send(
            sequence,
            CommandData::SetDestination {
                unit_id,
                destination,
            },
        )
    }

    pub fn send_reset_simulation(&mut self, sequence: u32) -> io::Result<()> {
        self.send(sequence, CommandData::ResetSimulation)
    }

    fn send(&mut self, sequence: u32, command: CommandData) -> io::Result<()> {
        spacegame2d_protocol::write_message(
            &mut self.stream,
            &Message::CommandRequest(CommandRequest {
                sequence,
                command: command.clone(),
            }),
        )?;
        tracing::info!(
            event = "command_sent",
            cmd = %format!("{}:{}", self.player_slot, sequence),
            local_tick = self.server_tick,
            kind = command_kind(&command),
        );
        Ok(())
    }

    pub fn poll_commands(&mut self) -> io::Result<Vec<AuthoritativeCommand>> {
        let mut bytes = [0u8; 4096];
        let mut result = Vec::new();
        loop {
            match self.stream.read(&mut bytes) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "server disconnected",
                    ));
                }
                Ok(size) => {
                    for message in self.decoder.push(&bytes[..size])? {
                        if let Message::AuthoritativeCommand(command) = message {
                            tracing::info!(
                                event = "authoritative_received",
                                execute_tick = command.execute_tick,
                                server_tick = self.server_tick,
                                local_tick = self.server_tick,
                                kind = command_kind(&command.command),
                            );
                            result.push(command);
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(result),
                Err(error) => return Err(error),
            }
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn validate_server_hello(hello: &ServerHello) -> io::Result<()> {
    if hello.simulation_version != SIMULATION_VERSION {
        return Err(invalid("simulation version mismatch"));
    }
    if hello.simulation_hz != SIMULATION_HZ {
        return Err(invalid("simulation frequency mismatch"));
    }
    if hello.player_slot == 0 {
        return Err(invalid("server assigned reserved player slot"));
    }
    Ok(())
}

fn command_kind(command: &CommandData) -> &'static str {
    match command {
        CommandData::SetDestination { .. } => "set_destination",
        CommandData::ResetSimulation => "reset_simulation",
    }
}

pub fn apply_due_commands(
    simulation: &mut spacegame2d_simulation::simulation::Simulation,
    scheduled: &mut BTreeMap<u64, Vec<AuthoritativeCommand>>,
) {
    if let Some(commands) = scheduled.remove(&simulation.tick()) {
        for command in commands {
            simulation.schedule_authoritative(&command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_server_hello_rejects_version_and_hz() {
        let base = ServerHello {
            simulation_version: SIMULATION_VERSION,
            simulation_hz: SIMULATION_HZ,
            player_slot: 1,
            server_tick: 42,
            capabilities: vec![],
        };
        let mut wrong = base.clone();
        wrong.simulation_version += 1;
        assert_eq!(
            validate_server_hello(&wrong).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        wrong = base.clone();
        wrong.simulation_hz += 1;
        assert_eq!(
            validate_server_hello(&wrong).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn apply_due_commands_only_at_matching_tick() {
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        simulation.world.units[0].owner = Some(spacegame2d_simulation::command::PlayerId(1));
        let command = AuthoritativeCommand {
            execute_tick: 2,
            player_slot: 1,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id: 1,
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        };
        let mut scheduled = BTreeMap::from([(2, vec![command])]);
        apply_due_commands(&mut simulation, &mut scheduled);
        assert!(simulation.commands.history().is_empty());
        simulation.step();
        simulation.step();
        apply_due_commands(&mut simulation, &mut scheduled);
        simulation.step();
        assert_eq!(simulation.commands.history().len(), 1);
    }
}
