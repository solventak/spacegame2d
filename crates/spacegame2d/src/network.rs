use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc,
    time::{Duration, Instant},
};

use spacegame2d_protocol::Tick;
use spacegame2d_protocol::{
    AuthoritativeCommand, Capability, ClientHello, CommandData, CommandRejected, CommandRequest,
    HandshakeRejectionReason, MatchTiming, Message, OpponentPresence, SIMULATION_VERSION,
    SessionParticipant, SessionSnapshot, StateChecksum,
};
use spacegame2d_simulation::{SimulationConfig, simulation::SIMULATION_HZ};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("connection rejected: {0:?}")]
    Rejected(HandshakeRejectionReason),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl ConnectError {
    #[cfg(test)]
    fn kind(&self) -> io::ErrorKind {
        match self {
            Self::Io(error) => error.kind(),
            Self::Rejected(_) => io::ErrorKind::PermissionDenied,
        }
    }
}

pub struct NetworkSession {
    stream: TcpStream,
    pub player_slot: u32,
    pub server_tick: Tick,
    simulation_config: SimulationConfig,
    checksum_enabled: bool,
    local_tick: Tick,
    initial_simulation: Option<spacegame2d_simulation::simulation::Simulation>,
    session_snapshot: SessionSnapshot,
    decoder: spacegame2d_protocol::FrameDecoder,
    outgoing: VecDeque<Vec<u8>>,
}

impl NetworkSession {
    pub fn connect(address: &str) -> Result<Self, ConnectError> {
        Self::connect_with_timeout(address, Duration::from_secs(5))
    }

    pub fn connect_with_timeout(address: &str, timeout: Duration) -> Result<Self, ConnectError> {
        Self::connect_with_timeout_and_progress(address, "Test Player", timeout, |_| {})
    }

    pub fn connect_with_timeout_and_progress<F>(
        address: &str,
        display_name: &str,
        timeout: Duration,
        progress: F,
    ) -> Result<Self, ConnectError>
    where
        F: Fn(crate::session::ConnectionProgress),
    {
        let started = Instant::now();
        let addresses =
            resolve_addresses(address.to_owned(), remaining_timeout(started, timeout)?)?;
        progress(crate::session::ConnectionProgress::OpeningSocket);
        let mut last_error = None;
        let mut stream = None;
        for address in addresses {
            let remaining = remaining_timeout(started, timeout)?;
            match TcpStream::connect_timeout(&address, remaining) {
                Ok(value) => {
                    stream = Some(value);
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }
        let mut stream = stream.ok_or_else(|| {
            last_error.unwrap_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "no socket addresses")
            })
        })?;
        stream.set_nodelay(true)?;
        let remaining = remaining_timeout(started, timeout)?;
        stream.set_read_timeout(Some(remaining))?;
        stream.set_write_timeout(Some(remaining))?;
        progress(crate::session::ConnectionProgress::Handshaking);
        Message::ClientHello(ClientHello {
            simulation_version: SIMULATION_VERSION,
            capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
            display_name: display_name.into(),
        })
        .write(&mut stream)?;
        let hello = match Message::read(&mut stream)? {
            Message::ServerHello(value) => value,
            Message::HandshakeRejected(value) => return Err(ConnectError::Rejected(value.reason)),
            _ => return Err(invalid("expected ServerHello").into()),
        };
        hello.validate(SIMULATION_HZ)?;
        let simulation_config = SimulationConfig::try_from(hello.fleet_size)
            .and_then(|config| {
                config.with_world_radius_meters(f32::from_bits(hello.world_radius_bits))
            })
            .map_err(|error| invalid(&error.to_string()))?;
        let snapshot = match Message::read(&mut stream)? {
            Message::InitialWorldState(value) => value,
            Message::HandshakeRejected(value) => return Err(ConnectError::Rejected(value.reason)),
            _ => return Err(invalid("expected InitialWorldState").into()),
        };
        if snapshot.tick != hello.server_tick {
            return Err(invalid("server hello and snapshot ticks differ").into());
        }
        let initial_simulation =
            spacegame2d_simulation::simulation::Simulation::restore_initial_world_state(
                &snapshot,
                simulation_config.clone(),
            )
            .map_err(|error| invalid(&error.to_string()))?;
        let session_snapshot = match Message::read(&mut stream)? {
            Message::SessionSnapshot(value) => value,
            Message::HandshakeRejected(value) => return Err(ConnectError::Rejected(value.reason)),
            _ => return Err(invalid("expected SessionSnapshot").into()),
        };
        if session_snapshot.local_player_slot != hello.player_slot {
            return Err(invalid("server hello and session local slot differ").into());
        }
        tracing::info!(event = "world_snapshot_restored", tick = ?snapshot.tick, units = snapshot.units.len());
        stream.set_read_timeout(None)?;
        stream.set_write_timeout(None)?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            player_slot: hello.player_slot,
            server_tick: hello.server_tick,
            simulation_config,
            checksum_enabled: hello.capabilities.contains(&Capability::StateChecksums),
            local_tick: hello.server_tick,
            initial_simulation: Some(initial_simulation),
            session_snapshot,
            decoder: spacegame2d_protocol::FrameDecoder::new(),
            outgoing: VecDeque::new(),
        })
    }

    pub fn simulation_config(&self) -> SimulationConfig {
        self.simulation_config.clone()
    }
    pub fn take_initial_simulation(
        &mut self,
    ) -> io::Result<spacegame2d_simulation::simulation::Simulation> {
        self.initial_simulation
            .take()
            .ok_or_else(|| invalid("initial simulation already taken"))
    }

    pub fn register_player(
        &self,
        simulation: &mut spacegame2d_simulation::simulation::Simulation,
    ) -> io::Result<()> {
        let player = spacegame2d_simulation::command::PlayerId::try_from(self.player_slot)
            .map_err(|_| invalid("server assigned invalid player slot"))?;
        simulation.world.assign_mirror_owners();
        simulation.world.connect_player(player);
        Ok(())
    }

    pub fn send_set_destination(&mut self, sequence: u32, destination: [u32; 2]) -> io::Result<()> {
        self.send(sequence, CommandData::SetDestination { destination })
    }

    pub fn send_reset_simulation(&mut self, sequence: u32) -> io::Result<()> {
        self.send(sequence, CommandData::ResetSimulation)
    }

    pub fn set_local_tick(&mut self, tick: Tick) {
        self.local_tick = tick;
    }

    pub fn session_snapshot(&self) -> &SessionSnapshot {
        &self.session_snapshot
    }

    pub fn local_participant(&self) -> &SessionParticipant {
        self.session_snapshot.local_participant()
    }

    pub fn opponent_presence(&self) -> OpponentPresence {
        self.session_snapshot.opponent_presence
    }

    pub fn match_started_at(&self) -> Option<Tick> {
        match self.session_snapshot.match_timing {
            MatchTiming::Inactive => None,
            MatchTiming::Active { started_at_tick } => Some(started_at_tick),
        }
    }

    pub fn elapsed_match_ticks(&self) -> Option<Tick> {
        self.match_started_at().map(|tick| self.local_tick - tick)
    }

    pub fn elapsed_match_seconds(&self) -> Option<u64> {
        self.elapsed_match_ticks()
            .map(|ticks| ticks.0 / u64::from(SIMULATION_HZ))
    }

    pub fn send_state_checksum(&mut self, tick: Tick, hash: [u8; 32]) -> io::Result<()> {
        if self.checksum_enabled {
            self.outgoing.push_back(
                Message::StateChecksum(StateChecksum {
                    tick,
                    hash: hash.to_vec(),
                })
                .encode()?,
            );
            self.flush_outgoing()?;
        }
        Ok(())
    }

    fn send(&mut self, sequence: u32, command: CommandData) -> io::Result<()> {
        self.outgoing.push_back(
            Message::CommandRequest(CommandRequest {
                sequence,
                command: command.clone(),
            })
            .encode()?,
        );
        self.flush_outgoing()?;
        tracing::info!(
            event = "command_sent",
            cmd = %format!("{}:{}", self.player_slot, sequence),
            local_tick = ?self.local_tick,
            kind = ?command
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

    pub fn poll_events(&mut self) -> io::Result<Vec<ServerEvent>> {
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
                        match message {
                            Message::AuthoritativeCommand(command) => {
                                tracing::info!(event = "authoritative_received", execute_tick = ?command.execute_tick, server_tick = ?self.server_tick, local_tick = ?self.local_tick, kind = ?command.command);
                                result.push(ServerEvent::Authoritative(command));
                            }
                            Message::CommandRejected(rejection) => {
                                tracing::warn!(event = "command_rejection_received", local_tick = ?self.local_tick, sequence = rejection.sequence, reason = ?rejection.reason);
                                result.push(ServerEvent::Rejected(rejection));
                            }
                            Message::SessionSnapshot(snapshot) => {
                                if snapshot.local_player_slot != self.player_slot {
                                    return Err(invalid("session update local slot differs"));
                                }
                                if snapshot.presence_revision
                                    > self.session_snapshot.presence_revision
                                {
                                    self.session_snapshot = snapshot.clone();
                                    result.push(ServerEvent::SessionStateChanged(snapshot));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(result),
                Err(error) => return Err(error),
            }
        }
    }
}

fn remaining_timeout(started: Instant, timeout: Duration) -> io::Result<Duration> {
    timeout
        .checked_sub(started.elapsed())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))
}

fn resolve_addresses(address: String, timeout: Duration) -> io::Result<Vec<std::net::SocketAddr>> {
    resolve_with_timeout(timeout, move || {
        address.to_socket_addrs().map(Iterator::collect)
    })
}

fn resolve_with_timeout<F>(timeout: Duration, resolve: F) -> io::Result<Vec<std::net::SocketAddr>>
where
    F: FnOnce() -> io::Result<Vec<std::net::SocketAddr>> + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = sender.send(resolve());
    });
    receiver
        .recv_timeout(timeout)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                io::Error::new(io::ErrorKind::TimedOut, "DNS lookup timed out")
            }
            mpsc::RecvTimeoutError::Disconnected => io::Error::other("DNS lookup worker stopped"),
        })?
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerEvent {
    Authoritative(AuthoritativeCommand),
    Rejected(CommandRejected),
    SessionStateChanged(SessionSnapshot),
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_protocol::{PlayerColor, ServerHello};
    use std::collections::BTreeMap;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    fn synthetic_server(response: Message, disconnect: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = Message::read(&mut stream).unwrap();
            response.write(&mut stream).unwrap();
            if let Message::ServerHello(hello) = response {
                Message::InitialWorldState(initial_world_state(&hello))
                    .write(&mut stream)
                    .unwrap();
                Message::SessionSnapshot(session_snapshot(&hello))
                    .write(&mut stream)
                    .unwrap();
            }
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
            let _ = Message::read(&mut stream).unwrap();
            Message::ServerHello(server_hello())
                .write(&mut stream)
                .unwrap();
            Message::InitialWorldState(initial_world_state(&server_hello()))
                .write(&mut stream)
                .unwrap();
            Message::SessionSnapshot(session_snapshot(&server_hello()))
                .write(&mut stream)
                .unwrap();
            Message::AuthoritativeCommand(command)
                .write(&mut stream)
                .unwrap();
            thread::sleep(std::time::Duration::from_millis(100));
        });
        address
    }

    fn synthetic_server_with_messages(messages: Vec<Message>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = Message::read(&mut stream).unwrap();
            Message::ServerHello(server_hello())
                .write(&mut stream)
                .unwrap();
            Message::InitialWorldState(initial_world_state(&server_hello()))
                .write(&mut stream)
                .unwrap();
            Message::SessionSnapshot(session_snapshot(&server_hello()))
                .write(&mut stream)
                .unwrap();
            for message in messages {
                message.write(&mut stream).unwrap();
            }
            thread::sleep(std::time::Duration::from_millis(100));
        });
        address
    }

    fn synthetic_server_collecting(count: usize) -> (String, mpsc::Receiver<Vec<Message>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = Message::read(&mut stream).unwrap();
            Message::ServerHello(server_hello())
                .write(&mut stream)
                .unwrap();
            Message::InitialWorldState(initial_world_state(&server_hello()))
                .write(&mut stream)
                .unwrap();
            Message::SessionSnapshot(session_snapshot(&server_hello()))
                .write(&mut stream)
                .unwrap();
            let mut messages = Vec::with_capacity(count);
            for _ in 0..count {
                messages.push(Message::read(&mut stream).unwrap());
            }
            sender.send(messages).unwrap();
        });
        (address, receiver)
    }

    fn synthetic_server_capturing_hello() -> (String, mpsc::Receiver<ClientHello>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let Message::ClientHello(hello) = Message::read(&mut stream).unwrap() else {
                panic!("expected client hello");
            };
            sender.send(hello).unwrap();
            Message::ServerHello(server_hello())
                .write(&mut stream)
                .unwrap();
            Message::InitialWorldState(initial_world_state(&server_hello()))
                .write(&mut stream)
                .unwrap();
            Message::SessionSnapshot(session_snapshot(&server_hello()))
                .write(&mut stream)
                .unwrap();
            thread::sleep(std::time::Duration::from_millis(100));
        });
        (address, receiver)
    }

    fn server_hello() -> ServerHello {
        ServerHello {
            simulation_version: SIMULATION_VERSION,
            simulation_hz: SIMULATION_HZ,
            player_slot: 1,
            server_tick: Tick::from(123),
            fleet_size: 30,
            world_radius_bits: 64.0_f32.to_bits(),
            capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
        }
    }

    fn session_snapshot(hello: &ServerHello) -> SessionSnapshot {
        SessionSnapshot {
            local_player_slot: hello.player_slot,
            participants: vec![SessionParticipant {
                player_slot: hello.player_slot,
                display_name: "Test Player".into(),
                color: PlayerColor::Cyan,
            }],
            opponent_presence: OpponentPresence::Waiting,
            presence_revision: 0,
            match_timing: MatchTiming::Inactive,
        }
    }

    fn active_session_snapshot(revision: u64) -> SessionSnapshot {
        SessionSnapshot {
            local_player_slot: 1,
            participants: vec![
                SessionParticipant {
                    player_slot: 1,
                    display_name: "Rook".into(),
                    color: PlayerColor::Cyan,
                },
                SessionParticipant {
                    player_slot: 2,
                    display_name: "Nova".into(),
                    color: PlayerColor::Coral,
                },
            ],
            opponent_presence: OpponentPresence::Present,
            presence_revision: revision,
            match_timing: MatchTiming::Active {
                started_at_tick: Tick::from(100),
            },
        }
    }

    fn initial_world_state(hello: &ServerHello) -> spacegame2d_protocol::InitialWorldState {
        let config = SimulationConfig::try_from(hello.fleet_size)
            .unwrap()
            .with_world_radius_meters(f32::from_bits(hello.world_radius_bits))
            .unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::new(config);
        simulation.set_tick(hello.server_tick);
        simulation.initial_world_state(vec![])
    }

    #[test]
    fn connect_returns_slot_and_tick() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let session = NetworkSession::connect(&address).unwrap();
        assert_eq!(session.player_slot, 1);
        assert_eq!(session.server_tick, Tick::new(123));
        assert_eq!(session.simulation_config().world_radius_meters(), 64.0);
    }

    #[test]
    fn connect_sends_the_frozen_canonical_display_name() {
        let (address, received) = synthetic_server_capturing_hello();
        NetworkSession::connect_with_timeout_and_progress(
            &address,
            "Café",
            Duration::from_secs(1),
            |_| {},
        )
        .unwrap();
        assert_eq!(received.recv().unwrap().display_name, "Café");
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
    fn connect_returns_typed_handshake_rejection() {
        let address = synthetic_server(
            Message::HandshakeRejected(spacegame2d_protocol::HandshakeRejected {
                reason: HandshakeRejectionReason::ServerFull,
            }),
            false,
        );
        assert!(matches!(
            NetworkSession::connect(&address),
            Err(ConnectError::Rejected(HandshakeRejectionReason::ServerFull))
        ));
    }

    #[test]
    fn connect_refused_returns_err() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        assert!(NetworkSession::connect(&address).is_err());
    }

    #[test]
    fn address_resolution_obeys_the_connection_deadline() {
        let started = Instant::now();
        let error = resolve_with_timeout(Duration::from_millis(10), || {
            thread::sleep(Duration::from_millis(50));
            Ok(vec![])
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(45));
    }

    #[test]
    fn poll_commands_non_blocking_when_empty() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let mut session = NetworkSession::connect(&address).unwrap();
        assert!(session.poll_events().unwrap().is_empty());
    }

    #[test]
    fn tick_snaps_to_server_tick() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let session = NetworkSession::connect(&address).unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        simulation.set_tick(session.server_tick);
        assert_eq!(simulation.tick(), Tick::new(123));
    }

    #[test]
    fn server_disconnect_is_unexpected_eof() {
        let address = synthetic_server(Message::ServerHello(server_hello()), true);
        let mut session = NetworkSession::connect(&address).unwrap();
        thread::sleep(std::time::Duration::from_millis(20));
        let error = session.poll_events().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn validate_server_hello_rejects_version_and_hz() {
        let base = ServerHello {
            simulation_version: SIMULATION_VERSION,
            simulation_hz: SIMULATION_HZ,
            player_slot: 1,
            server_tick: Tick::from(42),
            fleet_size: 30,
            world_radius_bits: 64.0_f32.to_bits(),
            capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
        };
        let mut wrong = base.clone();
        wrong.simulation_version += 1;
        assert_eq!(
            wrong.validate(SIMULATION_HZ).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        wrong = base.clone();
        wrong.capabilities.clear();
        assert_eq!(
            wrong.validate(SIMULATION_HZ).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        wrong = base.clone();
        wrong.simulation_hz += 1;
        assert_eq!(
            wrong.validate(SIMULATION_HZ).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        wrong = base;
        wrong.player_slot = u32::from(u8::MAX) + 1;
        assert_eq!(
            wrong.validate(SIMULATION_HZ).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        for world_radius_bits in [0.0_f32.to_bits(), f32::NAN.to_bits(), (-1.0_f32).to_bits()] {
            let mut wrong = ServerHello {
                simulation_version: SIMULATION_VERSION,
                simulation_hz: SIMULATION_HZ,
                player_slot: 1,
                server_tick: Tick::from(42),
                fleet_size: 30,
                world_radius_bits,
                capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
            };
            assert_eq!(
                wrong.validate(SIMULATION_HZ).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
            wrong.world_radius_bits = 64.0_f32.to_bits();
        }
    }

    #[test]
    fn register_player_rejects_invalid_slot() {
        let address = synthetic_server(Message::ServerHello(server_hello()), false);
        let mut session = NetworkSession::connect(&address).unwrap();
        session.player_slot = u32::from(u8::MAX) + 1;
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        let error = session.register_player(&mut simulation).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn outbound_commands_and_checksum_are_encoded_and_flushed() {
        let (address, received) = synthetic_server_collecting(3);
        let mut session = NetworkSession::connect(&address).unwrap();
        session.set_local_tick(Tick::from(200));
        session.send_set_destination(4, [10, 20]).unwrap();
        session.send_reset_simulation(5).unwrap();
        session
            .send_state_checksum(Tick::from(200), [7; 32])
            .unwrap();
        assert!(session.outgoing.is_empty());
        assert_eq!(
            received.recv().unwrap(),
            vec![
                Message::CommandRequest(CommandRequest {
                    sequence: 4,
                    command: CommandData::SetDestination {
                        destination: [10, 20]
                    }
                }),
                Message::CommandRequest(CommandRequest {
                    sequence: 5,
                    command: CommandData::ResetSimulation
                }),
                Message::StateChecksum(StateChecksum {
                    tick: Tick::from(200),
                    hash: vec![7; 32]
                }),
            ]
        );
    }

    #[test]
    fn poll_events_returns_authoritative_and_rejected_messages_and_ignores_others() {
        let authoritative = AuthoritativeCommand {
            execute_tick: Tick::from(9),
            player_slot: 1,
            sequence: 3,
            command: CommandData::ResetSimulation,
        };
        let rejected = CommandRejected {
            sequence: 4,
            reason: spacegame2d_protocol::CommandRejectionReason::InvalidCommand,
        };
        let address = synthetic_server_with_messages(vec![
            Message::StateChecksum(StateChecksum {
                tick: Tick::from(9),
                hash: vec![1],
            }),
            Message::AuthoritativeCommand(authoritative.clone()),
            Message::CommandRejected(rejected.clone()),
        ]);
        let mut session = NetworkSession::connect(&address).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let mut events = Vec::new();
        while std::time::Instant::now() < deadline {
            events.extend(session.poll_events().unwrap());
            if events.len() == 2 {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            events,
            vec![
                ServerEvent::Authoritative(authoritative),
                ServerEvent::Rejected(rejected)
            ]
        );
    }

    #[test]
    fn session_updates_ignore_stale_revisions_and_derive_elapsed_seconds() {
        let newer = active_session_snapshot(2);
        let stale = active_session_snapshot(1);
        let address = synthetic_server_with_messages(vec![
            Message::SessionSnapshot(newer.clone()),
            Message::SessionSnapshot(stale),
        ]);
        let mut session = NetworkSession::connect(&address).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let mut events = Vec::new();
        while std::time::Instant::now() < deadline {
            events.extend(session.poll_events().unwrap());
            if !events.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(events, vec![ServerEvent::SessionStateChanged(newer)]);
        assert_eq!(session.session_snapshot().presence_revision, 2);
        session.set_local_tick(Tick::from(159));
        assert_eq!(session.elapsed_match_seconds(), Some(0));
        session.set_local_tick(Tick::from(160));
        assert_eq!(session.elapsed_match_seconds(), Some(1));
        session.set_local_tick(Tick::from(219));
        assert_eq!(session.elapsed_match_seconds(), Some(1));
    }

    #[test]
    fn apply_due_commands_only_at_matching_tick() {
        let mut simulation = spacegame2d_simulation::simulation::Simulation::default();
        simulation.world.units[0].owner = Some(spacegame2d_simulation::command::PlayerId(1));
        let command = AuthoritativeCommand {
            execute_tick: Tick::from(2),
            player_slot: 1,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        };
        let mut scheduled = BTreeMap::from([(Tick::from(2), vec![command])]);
        simulation.apply_due_commands(&mut scheduled);
        assert!(simulation.commands.history().is_empty());
        simulation.step().unwrap();
        simulation.step().unwrap();
        simulation.apply_due_commands(&mut scheduled);
        simulation.step().unwrap();
        assert_eq!(simulation.commands.history().len(), 1);
    }

    #[test]
    fn late_arrival_through_network_session_applies_overdue_command() {
        let command = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [0.0f32.to_bits(), 100.0f32.to_bits()],
            },
        };
        let address = synthetic_server_with_authoritative(command);
        let mut session = NetworkSession::connect(&address).unwrap();
        let mut simulation = spacegame2d_simulation::simulation::Simulation::with_world_radius(1.0);
        simulation.world.units.truncate(1);
        simulation.world.units[0].owner = Some(spacegame2d_simulation::command::PlayerId(1));
        simulation.world.units[0].state.position = glam::Vec2::new(0.0, 0.9);
        simulation.world.units[0].state.heading_radians = 0.0;
        simulation.set_tick(Tick::from(1));
        let mut commands = Vec::new();
        for _ in 0..20 {
            commands = session.poll_events().unwrap();
            if !commands.is_empty() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(commands.len(), 1);
        let mut scheduled: BTreeMap<Tick, Vec<AuthoritativeCommand>> = BTreeMap::new();
        for event in commands {
            let ServerEvent::Authoritative(command) = event else {
                continue;
            };
            scheduled
                .entry(command.execute_tick)
                .or_default()
                .push(command);
        }
        simulation.apply_due_commands(&mut scheduled);
        assert!(scheduled.is_empty());
        let mut events = Vec::new();
        for _ in 0..30 {
            events.extend(simulation.step().unwrap());
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
            execute_tick: Tick::default(),
            player_slot: session.player_slot,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        assert!(simulation.schedule_authoritative(&reset));
        simulation.step().unwrap();
        assert!(matches!(
            simulation.commands.history().first(),
            Some(spacegame2d_simulation::command::RecordedCommand::ResetSimulation { .. })
        ));
    }
}
