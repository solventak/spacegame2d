use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, io,
    net::SocketAddr,
    time::Duration,
};

use spacegame2d_protocol::{
    AuthoritativeCommand, Capability, CommandData, CommandRejected, CommandRejectionReason,
    CommandRequest, Message, SIMULATION_VERSION, StateChecksum, Tick,
};
use spacegame2d_simulation::{
    MAX_PLAYERS, SimulationConfig,
    command::{Command, PlayerId},
    simulation::SIMULATION_HZ,
    simulation::Simulation,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

pub const COMMAND_INPUT_DELAY: Tick = Tick::new(2);

struct Client {
    stream: TcpStream,
    address: SocketAddr,
    slot: u32,
    connected: bool,
    decoder: spacegame2d_protocol::FrameDecoder,
    outgoing: VecDeque<Vec<u8>>,
    checksum_enabled: bool,
    pending_checksums: VecDeque<(Tick, Vec<u8>)>,
    seen_checksums: BTreeSet<Tick>,
}

impl Client {
    fn rejection_reason(
        simulation: &Simulation,
        slot: u32,
        request: &CommandRequest,
    ) -> Option<CommandRejectionReason> {
        let command = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: slot,
            sequence: request.sequence,
            command: request.command.clone(),
        };
        if simulation.world().validate_authoritative(&command).is_err() {
            return Some(CommandRejectionReason::UnauthorizedFleet);
        }
        let CommandData::SetDestination { destination } = &request.command else {
            return None;
        };
        let x = f32::from_bits(destination[0]);
        let y = f32::from_bits(destination[1]);
        if !x.is_finite() || !y.is_finite() {
            return Some(CommandRejectionReason::NonFiniteDestination);
        }
        if x.hypot(y) > simulation.world_radius() {
            return Some(CommandRejectionReason::DestinationOutsideArena);
        }
        if Box::<dyn Command>::try_from(&command).is_err() {
            return Some(CommandRejectionReason::InvalidCommand);
        }
        None
    }

    #[cfg(test)]
    fn valid_request(simulation: &Simulation, slot: u32, request: &CommandRequest) -> bool {
        Self::rejection_reason(simulation, slot, request).is_none()
    }

    fn read_messages(&mut self) -> io::Result<Vec<Message>> {
        let mut bytes = [0u8; 4096];
        let mut messages = Vec::new();
        loop {
            match self.stream.try_read(&mut bytes) {
                Ok(0) => {
                    if messages.is_empty() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "client disconnected",
                        ));
                    }
                    break;
                }
                Ok(size) => messages.extend(self.decoder.push(&bytes[..size])?),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(messages)
    }

    fn queue(&mut self, message: &Message) -> io::Result<()> {
        self.outgoing.push_back(message.encode()?);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        while let Some(frame) = self.outgoing.front_mut() {
            match self.stream.try_write(frame) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "client write closed",
                    ));
                }
                Ok(size) if size == frame.len() => {
                    self.outgoing.pop_front();
                }
                Ok(size) => {
                    frame.drain(..size);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

pub async fn run(listener: TcpListener, shutdown: watch::Receiver<bool>) -> io::Result<()> {
    run_with_config(listener, shutdown, SimulationConfig::default()).await
}

pub async fn run_with_config(
    listener: TcpListener,
    mut shutdown: watch::Receiver<bool>,
    config: SimulationConfig,
) -> io::Result<()> {
    let bound = listener.local_addr()?;
    tracing::info!(event = "server_listening", address = %bound, "server listening");
    let mut clients = Vec::new();
    let mut simulation = Simulation::new(config);
    let mut scheduled = BTreeMap::<Tick, Vec<AuthoritativeCommand>>::new();
    let mut state_hashes = BTreeMap::<Tick, [u8; 32]>::new();
    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / SIMULATION_HZ as f64));
    loop {
        tokio::select! {
            _ = interval.tick() => {}
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    return Ok(());
                }
            }
        }
        loop {
            let accepted = match tokio::time::timeout(Duration::ZERO, listener.accept()).await {
                Ok(result) => result?,
                Err(_) => break,
            };
            let (stream, address) = accepted;
            let Some(slot) = (1..=u32::try_from(MAX_PLAYERS).expect("player count fits u32")).find(
                |candidate| {
                    clients
                        .iter()
                        .all(|client: &Client| client.slot != *candidate)
                },
            ) else {
                tracing::warn!(event = "client_rejected", address = %address, "player slots exhausted");
                continue;
            };
            let player_id = PlayerId::new(u8::try_from(slot).expect("slot is within u8 range"))
                .expect("slot is nonzero");
            stream.set_nodelay(true)?;
            simulation.world.connect_player(player_id);
            if !simulation.world.assign_player_fleet(player_id) {
                tracing::warn!(event = "fleet_assignment_failed", address = %address, slot, "could not assign fleet to player");
                continue;
            }
            clients.push(Client {
                stream,
                address,
                slot,
                connected: false,
                checksum_enabled: false,
                pending_checksums: VecDeque::new(),
                seen_checksums: BTreeSet::new(),
                decoder: spacegame2d_protocol::FrameDecoder::new(),
                outgoing: VecDeque::new(),
            });
        }
        let mut remove = Vec::new();
        let mut broadcasts = Vec::new();
        let mut reset_cutover = false;
        for (index, client) in clients.iter_mut().enumerate() {
            match client.read_messages() {
                Ok(messages) => {
                    for message in messages {
                        if !client.connected {
                            let Message::ClientHello(hello) = message else {
                                remove.push(index);
                                continue;
                            };
                            if !hello.is_compatible()
                                || !hello.capabilities.contains(&Capability::StateChecksums)
                            {
                                tracing::warn!(event = "handshake_rejected", address = %client.address, "wrong simulation version");
                                remove.push(index);
                                continue;
                            }
                            client.checksum_enabled = true;
                            client.queue(&Message::ServerHello(
                                spacegame2d_protocol::ServerHello {
                                    simulation_version: SIMULATION_VERSION,
                                    simulation_hz: SIMULATION_HZ,
                                    player_slot: client.slot,
                                    server_tick: simulation.tick(),
                                    fleet_size: config.fleet_size(),
                                    world_radius_bits: config.world_radius_meters().to_bits(),
                                    capabilities: vec![Capability::StateChecksums],
                                },
                            ))?;
                            client.connected = true;
                        } else if let Message::StateChecksum(StateChecksum { tick, hash }) = message
                        {
                            if client.checksum_enabled {
                                if hash.len() != 32 {
                                    log_checksum_result(
                                        &client.address,
                                        client.slot,
                                        ChecksumResult::Malformed {
                                            tick,
                                            length: hash.len(),
                                        },
                                    );
                                    continue;
                                }
                                if client.seen_checksums.contains(&tick) {
                                    log_checksum_result(
                                        &client.address,
                                        client.slot,
                                        ChecksumResult::Duplicate { tick },
                                    );
                                } else if let Some(server_hash) = state_hashes.get(&tick) {
                                    client.seen_checksums.insert(tick);
                                    let result = if server_hash.as_slice() == hash.as_slice() {
                                        ChecksumResult::Match { tick }
                                    } else {
                                        ChecksumResult::Divergence {
                                            tick,
                                            server_hash: *server_hash,
                                            client_hash: hash.clone(),
                                        }
                                    };
                                    log_checksum_result(&client.address, client.slot, result);
                                } else if tick.0 % u64::from(SIMULATION_HZ) != 0 {
                                    client.seen_checksums.insert(tick);
                                    log_checksum_result(
                                        &client.address,
                                        client.slot,
                                        ChecksumResult::UnknownTick { tick },
                                    );
                                } else if tick > simulation.tick() {
                                    client.pending_checksums.push_back((tick, hash));
                                    while client.pending_checksums.len() > 16 {
                                        client.pending_checksums.pop_front();
                                    }
                                }
                            }
                        } else if let Message::CommandRequest(request) = message {
                            if reset_cutover {
                                tracing::info!(event = "command_rejected", tick = ?simulation.tick(), slot = client.slot, "command ignored after reset cutover");
                                continue;
                            }
                            let receive_tick = simulation.tick();
                            let cmd = format!("{}:{}", client.slot, request.sequence);
                            tracing::info!(event = "command_received", cmd = %cmd, tick = ?receive_tick, kind = ?request.command, address = %client.address, slot = client.slot);
                            if let Some(reason) =
                                Client::rejection_reason(&simulation, client.slot, &request)
                            {
                                tracing::warn!(event = "command_rejected", cmd = %cmd, tick = ?receive_tick, address = %client.address, slot = client.slot, reason = ?reason, "invalid command");
                                client.queue(&Message::CommandRejected(CommandRejected {
                                    sequence: request.sequence,
                                    reason,
                                }))?;
                                continue;
                            }
                            let is_reset = matches!(
                                request.command,
                                spacegame2d_protocol::CommandData::ResetSimulation
                            );
                            let authoritative = AuthoritativeCommand {
                                execute_tick: if is_reset {
                                    receive_tick
                                } else {
                                    receive_tick.increment(COMMAND_INPUT_DELAY)
                                },
                                player_slot: client.slot,
                                sequence: request.sequence,
                                command: request.command,
                            };
                            if is_reset {
                                scheduled.clear();
                                broadcasts.clear();
                                simulation.commands.clear_pending();
                                reset_cutover = true;
                            }
                            tracing::info!(event = "command_scheduled", cmd = %cmd, receive_tick = ?receive_tick, execute_tick = ?authoritative.execute_tick, tick = ?receive_tick, kind = ?authoritative.command, address = %client.address, slot = client.slot);
                            let encoded =
                                Message::AuthoritativeCommand(authoritative.clone()).encode()?;
                            scheduled
                                .entry(authoritative.execute_tick)
                                .or_default()
                                .push(authoritative.clone());
                            broadcasts.push((encoded, cmd, client.address, client.slot));
                        }
                    }
                }
                Err(error) => {
                    tracing::info!(event = "client_disconnected", address = %client.address, reason = %error);
                    remove.push(index);
                }
            }
        }
        for index in remove.into_iter().rev() {
            let client = clients.remove(index);
            if let Some(player_id) = PlayerId::new(u8::try_from(client.slot).unwrap_or(0)) {
                simulation.world.disconnect_player(player_id);
            }
        }
        for (encoded, cmd, address, slot) in broadcasts {
            let recipients = clients.iter().filter(|peer| peer.connected).count();
            for peer in clients.iter_mut().filter(|peer| peer.connected) {
                peer.outgoing.push_back(encoded.clone());
            }
            tracing::info!(event = "command_broadcast_queued", cmd = %cmd, recipients, address = %address, slot);
        }
        for client in &mut clients {
            if client.flush().is_err() {
                client.connected = false;
            }
        }
        if let Some(commands) = scheduled.remove(&simulation.tick()) {
            for command in commands {
                simulation.schedule_authoritative(&command);
            }
        }
        for event in simulation
            .step()
            .map_err(|error| io::Error::other(error.to_string()))?
        {
            match event {
                spacegame2d_simulation::SimulationEvent::ShotFired {
                    tick,
                    shooter_id,
                    hit_unit_id,
                    ..
                } => {
                    tracing::info!(event = "shot_fired", tick = ?tick, shooter_id = shooter_id.0, hit_unit_id = ?hit_unit_id.map(|id| id.0));
                }
                spacegame2d_simulation::SimulationEvent::HullDepleted {
                    tick,
                    unit_id,
                    position,
                } => {
                    tracing::info!(event = "hull_depleted", tick = ?tick, unit_id = unit_id.0, position = ?position);
                }
                spacegame2d_simulation::SimulationEvent::BoundaryCrossed {
                    tick,
                    unit_id,
                    position,
                } => {
                    tracing::info!(event = "boundary_crossed", tick = ?tick, unit_id = unit_id.0, position = ?position);
                }
            }
        }
        let completed_tick = simulation.tick();
        if completed_tick.0 % u64::from(SIMULATION_HZ) == 0 {
            state_hashes.insert(completed_tick, simulation.state_hash());
            let oldest = completed_tick - Tick::from(u64::from(SIMULATION_HZ) * 10);
            state_hashes.retain(|tick, _| *tick >= oldest);
            compare_pending_checksums(&mut clients, &state_hashes, completed_tick);
        }
    }
}

fn hex_hash(hash: &[u8]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, PartialEq, Eq)]
enum ChecksumResult {
    Match {
        tick: Tick,
    },
    Divergence {
        tick: Tick,
        server_hash: [u8; 32],
        client_hash: Vec<u8>,
    },
    Duplicate {
        tick: Tick,
    },
    Late {
        tick: Tick,
    },
    UnknownTick {
        tick: Tick,
    },
    Malformed {
        tick: Tick,
        length: usize,
    },
}

fn log_checksum_result(address: &SocketAddr, slot: u32, result: ChecksumResult) {
    match result {
        ChecksumResult::Match { .. } => {}
        ChecksumResult::Divergence {
            tick,
            server_hash,
            client_hash,
        } => {
            tracing::warn!(event = "state_divergence", address = %address, slot, tick = ?tick, server_hash = %hex_hash(&server_hash), client_hash = %hex_hash(&client_hash), "client simulation diverged from server")
        }
        ChecksumResult::Duplicate { tick } => {
            tracing::warn!(event = "state_checksum_report_warning", address = %address, slot, tick = ?tick, classification = "duplicate", "duplicate checksum report")
        }
        ChecksumResult::Late { tick } => {
            tracing::warn!(event = "state_checksum_report_warning", address = %address, slot, tick = ?tick, classification = "late", "late checksum report")
        }
        ChecksumResult::UnknownTick { tick } => {
            tracing::warn!(event = "state_checksum_report_warning", address = %address, slot, tick = ?tick, classification = "unknown_tick", "checksum report for unknown tick")
        }
        ChecksumResult::Malformed { tick, length } => {
            tracing::warn!(event = "state_checksum_report_warning", address = %address, slot, tick = ?tick, length, classification = "malformed", "ignored malformed checksum report")
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ChecksumDivergence {
    tick: Tick,
    server_hash: [u8; 32],
    client_hash: Vec<u8>,
}

fn checksum_divergence(
    tick: Tick,
    server_hash: &[u8; 32],
    client_hash: &[u8],
) -> Option<ChecksumDivergence> {
    (server_hash.as_slice() != client_hash).then(|| ChecksumDivergence {
        tick,
        server_hash: *server_hash,
        client_hash: client_hash.to_vec(),
    })
}

fn compare_pending_checksums(
    clients: &mut [Client],
    state_hashes: &BTreeMap<Tick, [u8; 32]>,
    completed_tick: Tick,
) {
    for client in clients.iter_mut() {
        let pending = std::mem::take(&mut client.pending_checksums);
        for (tick, hash) in pending {
            if let Some(server_hash) = state_hashes.get(&tick) {
                if let Some(divergence) = checksum_divergence(tick, server_hash, &hash) {
                    tracing::warn!(
                        event = "state_divergence",
                        address = %client.address,
                        slot = client.slot,
                        tick = ?divergence.tick,
                        server_hash = %hex_hash(&divergence.server_hash),
                        client_hash = %hex_hash(&divergence.client_hash),
                        "client simulation diverged from server"
                    );
                }
            } else if tick > completed_tick {
                client.pending_checksums.push_back((tick, hash));
            } else {
                log_checksum_result(&client.address, client.slot, ChecksumResult::Late { tick });
            }
        }
    }
}

fn simulation_config_from_env() -> Result<SimulationConfig, String> {
    let fleet_size = match env::var_os("SPACEGAME_FLEET_SIZE") {
        Some(value) => value
            .to_str()
            .ok_or_else(|| "SPACEGAME_FLEET_SIZE is not valid UTF-8".to_owned())?
            .parse::<u32>()
            .map_err(|_| "SPACEGAME_FLEET_SIZE must be a positive integer".to_owned())?,
        None => spacegame2d_simulation::DEFAULT_FLEET_SIZE,
    };
    let config = SimulationConfig::try_from(fleet_size).map_err(|error| error.to_string())?;
    let Some(value) = env::var_os("SPACEGAME_WORLD_RADIUS_METERS") else {
        return Ok(config);
    };
    let radius = value
        .to_str()
        .ok_or_else(|| "SPACEGAME_WORLD_RADIUS_METERS is not valid UTF-8".to_owned())?
        .parse::<f32>()
        .map_err(|_| "SPACEGAME_WORLD_RADIUS_METERS must be a finite positive number".to_owned())?;
    config
        .with_world_radius_meters(radius)
        .map_err(|error| error.to_string())
}

#[tokio::main]
async fn main() {
    let _logging = spacegame2d_logging::init("spacegame2d-server", "info")
        .expect("failed to initialize logging");
    let address: SocketAddr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid bind address");
    let config = match simulation_config_from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("invalid simulation configuration: {error}");
            return;
        }
    };
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(event = "server_stopped", error = %error);
            return;
        }
    };
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    if let Err(error) = run_with_config(listener, shutdown_rx, config).await {
        tracing::error!(event = "server_stopped", error = %error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_protocol::{ClientHello, FrameDecoder};
    use spacegame2d_simulation::command::{FLEET_SIZE, MAX_UNITS, UnitId};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::watch,
    };

    async fn read_message(stream: &mut tokio::net::TcpStream) -> Message {
        let mut header = [0; 4];
        stream.read_exact(&mut header).await.unwrap();
        let size = u32::from_be_bytes(header) as usize;
        let mut body = vec![0; size];
        stream.read_exact(&mut body).await.unwrap();
        let mut decoder = FrameDecoder::new();
        decoder.push(&header).unwrap();
        decoder.push(&body).unwrap().pop().unwrap()
    }

    async fn try_read_message(stream: &mut tokio::net::TcpStream) -> io::Result<Message> {
        let mut header = [0; 4];
        stream.read_exact(&mut header).await?;
        let size = u32::from_be_bytes(header) as usize;
        let mut body = vec![0; size];
        stream.read_exact(&mut body).await?;
        let mut decoder = FrameDecoder::new();
        decoder.push(&header).map_err(io::Error::other)?;
        decoder
            .push(&body)
            .map_err(io::Error::other)?
            .pop()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "empty frame"))
    }

    async fn start_server() -> (
        SocketAddr,
        watch::Sender<bool>,
        tokio::task::JoinHandle<io::Result<()>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, receiver) = watch::channel(false);
        let task = tokio::task::spawn_local(run(listener, receiver));
        (address, shutdown, task)
    }

    fn build_mirror(tick: Tick) -> Simulation {
        let mut sim = Simulation::default();
        sim.world.assign_mirror_owners();
        sim.set_tick(tick);
        sim
    }
    #[test]
    fn handshake_validation() {
        assert!(
            ClientHello {
                simulation_version: SIMULATION_VERSION,
                capabilities: vec![Capability::StateChecksums]
            }
            .is_compatible()
        );
        assert!(
            !(ClientHello {
                simulation_version: SIMULATION_VERSION - 1,
                capabilities: vec![Capability::StateChecksums]
            }
            .is_compatible())
        );
    }
    #[test]
    fn deliberate_checksum_difference_is_detected() {
        let expected = [0u8; 32];
        let mut different = expected;
        different[0] = 1;
        let divergence = checksum_divergence(Tick::from(60), &expected, &different).unwrap();
        assert_eq!(divergence.tick, Tick::from(60));
        assert_eq!(divergence.server_hash, expected);
        assert_eq!(divergence.client_hash, different);
    }

    #[test]
    fn scheduling_tick_math() {
        assert_eq!(
            Tick::from(40).increment(COMMAND_INPUT_DELAY),
            Tick::from(42)
        );
    }
    #[test]
    fn ownership_and_nan_are_rejected() {
        let sim = Simulation::default();
        let request = CommandRequest {
            sequence: 1,
            command: spacegame2d_protocol::CommandData::SetDestination {
                destination: [f32::NAN.to_bits(), 0],
            },
        };
        assert!(!Client::valid_request(&sim, 1, &request));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn real_tcp_handshake_tick_advancement_broadcast_and_disconnect() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;
                let mut first = tokio::net::TcpStream::connect(address).await.unwrap();
                let hello = Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![Capability::StateChecksums],
                });
                first.write_all(&hello.encode().unwrap()).await.unwrap();
                let Message::ServerHello(first_hello) = read_message(&mut first).await else {
                    panic!()
                };

                tokio::time::sleep(Duration::from_millis(50)).await;
                let mut second = tokio::net::TcpStream::connect(address).await.unwrap();
                second.write_all(&hello.encode().unwrap()).await.unwrap();
                let Message::ServerHello(second_hello) = read_message(&mut second).await else {
                    panic!()
                };
                assert_eq!(first_hello.player_slot, 1);
                assert_eq!(second_hello.player_slot, 2);
                first_hello.validate(SIMULATION_HZ).unwrap();
                second_hello.validate(SIMULATION_HZ).unwrap();
                assert!(second_hello.server_tick > first_hello.server_tick);

                // Late-joiner receives the same deterministic 60-unit layout and
                // ownership as the first client.
                let first_client_mirror = build_mirror(first_hello.server_tick);
                let late_joiner_mirror = build_mirror(second_hello.server_tick);
                assert_eq!(late_joiner_mirror.world.units.len(), MAX_UNITS);
                assert_eq!(
                    late_joiner_mirror.world.units.len(),
                    first_client_mirror.world.units.len()
                );
                assert!(
                    late_joiner_mirror.world.units[..FLEET_SIZE]
                        .iter()
                        .all(|unit| unit.owner == Some(PlayerId(1)))
                );
                assert!(
                    late_joiner_mirror.world.units[FLEET_SIZE..]
                        .iter()
                        .all(|unit| unit.owner == Some(PlayerId(2)))
                );
                let first_layout: Vec<_> = first_client_mirror
                    .world
                    .units
                    .iter()
                    .map(|unit| (unit.id, unit.owner, unit.state))
                    .collect();
                let late_layout: Vec<_> = late_joiner_mirror
                    .world
                    .units
                    .iter()
                    .map(|unit| (unit.id, unit.owner, unit.state))
                    .collect();
                assert_eq!(first_layout, late_layout);

                let owned_request = Message::CommandRequest(CommandRequest {
                    sequence: 6,
                    command: spacegame2d_protocol::CommandData::SetDestination {
                        destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
                    },
                });
                first
                    .write_all(&owned_request.encode().unwrap())
                    .await
                    .unwrap();
                let Message::AuthoritativeCommand(owned_command) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut second))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(owned_command.player_slot, 1);
                assert_eq!(
                    owned_command.command,
                    spacegame2d_protocol::CommandData::SetDestination {
                        destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
                    }
                );
                let Message::AuthoritativeCommand(_) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!()
                };

                // Player 2 issues a command for one of its own units; Player 1
                // must receive the broadcast and be able to apply it to its mirror.
                let p2_request = Message::CommandRequest(CommandRequest {
                    sequence: 10,
                    command: spacegame2d_protocol::CommandData::SetDestination {
                        destination: [5.0f32.to_bits(), 6.0f32.to_bits()],
                    },
                });
                second
                    .write_all(&p2_request.encode().unwrap())
                    .await
                    .unwrap();
                let Message::AuthoritativeCommand(p2_command) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(p2_command.player_slot, 2);
                assert_eq!(
                    p2_command.command,
                    spacegame2d_protocol::CommandData::SetDestination {
                        destination: [5.0f32.to_bits(), 6.0f32.to_bits()],
                    }
                );
                let Message::AuthoritativeCommand(_) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut second))
                        .await
                        .unwrap()
                else {
                    panic!()
                };

                let mut player_one_mirror = build_mirror(p2_command.execute_tick);
                assert!(player_one_mirror.schedule_authoritative_trusted(&p2_command));
                player_one_mirror.step().unwrap();
                let mirrored_unit = player_one_mirror
                    .world
                    .unit(UnitId(31))
                    .expect("unit 31 in Player 1 mirror");
                assert_eq!(mirrored_unit.owner, Some(PlayerId(2)));
                let destination = mirrored_unit.autopilot.destination().unwrap();
                assert!((destination.x - 5.0).abs() < f32::EPSILON);
                assert!((destination.y - 6.0).abs() < f32::EPSILON);

                let rejected_request = Message::CommandRequest(CommandRequest {
                    sequence: 9,
                    command: spacegame2d_protocol::CommandData::SetDestination {
                        destination: [80.0f32.to_bits(), 0.0f32.to_bits()],
                    },
                });
                first
                    .write_all(&rejected_request.encode().unwrap())
                    .await
                    .unwrap();
                let Message::CommandRejected(rejection) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(rejection.sequence, 9);
                assert_eq!(
                    rejection.reason,
                    CommandRejectionReason::DestinationOutsideArena
                );
                assert!(
                    tokio::time::timeout(Duration::from_millis(100), read_message(&mut second))
                        .await
                        .is_err()
                );

                let request = Message::CommandRequest(CommandRequest {
                    sequence: 7,
                    command: spacegame2d_protocol::CommandData::ResetSimulation,
                });
                second.write_all(&request.encode().unwrap()).await.unwrap();
                let Message::AuthoritativeCommand(first_command) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                let Message::AuthoritativeCommand(second_command) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut second))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(first_command, second_command);
                assert_eq!(first_command.sequence, 7);
                assert!(
                    first_command.execute_tick >= second_hello.server_tick + COMMAND_INPUT_DELAY
                );

                let mut third = tokio::net::TcpStream::connect(address).await.unwrap();
                third.write_all(&hello.encode().unwrap()).await.unwrap();
                let third_result =
                    tokio::time::timeout(Duration::from_millis(200), try_read_message(&mut third))
                        .await;
                assert!(matches!(third_result, Err(_) | Ok(Err(_))));

                first.shutdown().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                let mut recycled = tokio::net::TcpStream::connect(address).await.unwrap();
                recycled.write_all(&hello.encode().unwrap()).await.unwrap();
                let Message::ServerHello(recycled_hello) = read_message(&mut recycled).await else {
                    panic!()
                };
                assert_eq!(recycled_hello.player_slot, 1);

                second
                    .write_all(
                        &Message::CommandRequest(CommandRequest {
                            sequence: 8,
                            command: spacegame2d_protocol::CommandData::ResetSimulation,
                        })
                        .encode()
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                let Message::AuthoritativeCommand(command_after_disconnect) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut second))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(command_after_disconnect.sequence, 8);

                shutdown.send(true).unwrap();
                task.await.unwrap().unwrap();
            })
            .await;
    }
}
