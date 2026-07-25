use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, Read, Write},
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
    local_tick: u64,
    decoder: spacegame2d_protocol::FrameDecoder,
    outgoing: VecDeque<Vec<u8>>,
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
            local_tick: hello.server_tick,
            decoder: spacegame2d_protocol::FrameDecoder::new(),
            outgoing: VecDeque::new(),
        })
    }

    pub fn is_connected(&self) -> bool {
        true
    }

    pub fn register_player(
        &self,
        simulation: &mut spacegame2d_simulation::simulation::Simulation,
    ) -> io::Result<()> {
        let player = u8::try_from(self.player_slot)
            .ok()
            .and_then(spacegame2d_simulation::command::PlayerId::new)
            .ok_or_else(|| invalid("server assigned invalid player slot"))?;
        simulation.world.assign_mirror_owners();
        simulation.world.connect_player(player);
        Ok(())
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

    pub fn set_local_tick(&mut self, tick: u64) {
        self.local_tick = tick;
    }

    fn send(&mut self, sequence: u32, command: CommandData) -> io::Result<()> {
        self.outgoing
            .push_back(spacegame2d_protocol::encode_message(
                &Message::CommandRequest(CommandRequest {
                    sequence,
                    command: command.clone(),
                }),
            )?);
        self.flush_outgoing()?;
        tracing::info!(
            event = "command_sent",
            cmd = %format!("{}:{}", self.player_slot, sequence),
            local_tick = self.local_tick,
            kind = command_kind(&command),
        );
        Ok(())
    }

    fn flush_outgoing(&mut self) -> io::Result<()> {
        while let Some(frame) = self.outgoing.front_mut() {
            match self.stream.write(frame) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "server write closed",
                    ));
                }
                Ok(size) if size == frame.len() => {
                    self.outgoing.pop_front();
                }
                Ok(size) => {
                    frame.drain(..size);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn poll_commands(&mut self) -> io::Result<Vec<AuthoritativeCommand>> {
        self.flush_outgoing()?;
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
                                local_tick = self.local_tick,
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
    u8::try_from(hello.player_slot)
        .ok()
        .and_then(spacegame2d_simulation::command::PlayerId::new)
        .ok_or_else(|| invalid("server assigned invalid player slot"))?;
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
    let current_tick = simulation.tick();
    let due_ticks = scheduled
        .range(..=current_tick)
        .map(|(tick, _)| *tick)
        .collect::<Vec<_>>();
    for tick in due_ticks {
        if let Some(commands) = scheduled.remove(&tick) {
            for mut command in commands {
                if command.execute_tick < current_tick {
                    command.execute_tick = current_tick;
                }
                simulation.schedule_authoritative_trusted(&command);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    fn synthetic_server(response: Message, disconnect: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = spacegame2d_protocol::read_message(&mut stream).unwrap();
            spacegame2d_protocol::write_message(&mut stream, &response).unwrap();
            if disconnect {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            } else {
                thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        address
    }

    fn synthetic_server_with_authoritative(command: AuthoritativeCommand) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = spacegame2d_protocol::read_message(&mut stream).unwrap();
            spacegame2d_protocol::write_message(&mut stream, &Message::ServerHello(server_hello()))
                .unwrap();
            spacegame2d_protocol::write_message(
                &mut stream,
                &Message::AuthoritativeCommand(command),
            )
            .unwrap();
            thread::sleep(std::time::Duration::from_millis(100));
        });
        address
    }

    fn server_hello() -> ServerHello {
        ServerHello {
            simulation_version: SIMULATION_VERSION,
            simulation_hz: SIMULATION_HZ,
            player_slot: 7,
            server_tick: 123,
            capabilities: vec![],
        }
    }

    #[test]
    fn connect_returns_slot_and_tick() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let session = NetworkSession::connect(&address).unwrap();
        assert_eq!(session.player_slot, 7);
        assert_eq!(session.server_tick, 123);
        assert!(session.is_connected());
    }

    #[test]
    fn connect_rejects_wrong_version() {
        let mut hello = server_hello();
        hello.simulation_version += 1;
        let address = synthetic_server(Message::ServerHello(hello), false);
        let error = match NetworkSession::connect(&address) {
            Ok(_) => panic!("wrong version was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn connect_rejects_wrong_hz() {
        let mut hello = server_hello();
        hello.simulation_hz += 1;
        let address = synthetic_server(Message::ServerHello(hello), false);
        let error = match NetworkSession::connect(&address) {
            Ok(_) => panic!("wrong frequency was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn connect_refused_returns_err() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        assert!(NetworkSession::connect(&address).is_err());
    }

    #[test]
    fn poll_commands_non_blocking_when_empty() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let mut session = NetworkSession::connect(&address).unwrap();
        assert!(session.poll_commands().unwrap().is_empty());
    }

    #[test]
    fn tick_snaps_to_server_tick() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let session = NetworkSession::connect(&address).unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        simulation.set_tick(session.server_tick);
        assert_eq!(simulation.tick(), 123);
    }

    #[test]
    fn server_disconnect_is_unexpected_eof() {
        let address = synthetic_server(Message::ServerHello(server_hello()), true);
        let mut session = NetworkSession::connect(&address).unwrap();
        thread::sleep(std::time::Duration::from_millis(20));
        let error = session.poll_commands().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

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
        wrong = base;
        wrong.player_slot = u32::from(u8::MAX) + 1;
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

    #[test]
    fn late_arrival_through_network_session_applies_overdue_command() {
        let command = AuthoritativeCommand {
            execute_tick: 0,
            player_slot: 7,
            sequence: 1,
            command: CommandData::SetDestination {
                unit_id: 1,
                destination: [0.0f32.to_bits(), 100.0f32.to_bits()],
            },
        };
        let address = synthetic_server_with_authoritative(command);
        let mut session = NetworkSession::connect(&address).unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::with_world_radius(1.0);
        simulation.world.units.truncate(1);
        simulation.world.units[0].owner = Some(spacegame2d_simulation::command::PlayerId(7));
        simulation.world.units[0].state.position = glam::Vec2::new(0.0, 0.9);
        simulation.world.units[0].state.heading_radians = 0.0;
        simulation.set_tick(1);
        let mut commands = Vec::new();
        for _ in 0..20 {
            commands = session.poll_commands().unwrap();
            if !commands.is_empty() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(commands.len(), 1);
        let mut scheduled = BTreeMap::new();
        for command in commands {
            scheduled
                .entry(command.execute_tick)
                .or_insert_with(Vec::new)
                .push(command);
        }
        apply_due_commands(&mut simulation, &mut scheduled);
        assert!(scheduled.is_empty());
        let mut events = Vec::new();
        for _ in 0..30 {
            events.extend(simulation.step());
        }
        assert!(events.iter().any(|event| matches!(
            event,
            spacegame2d_simulation::SimulationEvent::BoundaryCrossed { .. }
        )));
    }

    #[test]
    fn handshake_registers_player_before_broadcast_reset_is_applied() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let session = NetworkSession::connect(&address).unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        session.register_player(&mut simulation).unwrap();
        let reset = AuthoritativeCommand {
            execute_tick: 0,
            player_slot: session.player_slot,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        assert!(simulation.schedule_authoritative(&reset));
        simulation.step();
        assert!(matches!(
            simulation.commands.history().first(),
            Some(spacegame2d_simulation::command::RecordedCommand::ResetSimulation { .. })
        ));
    }
}
