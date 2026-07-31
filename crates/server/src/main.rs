use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, io,
    net::SocketAddr,
    time::Duration,
};

use spacegame2d_protocol::{
    AuthoritativeCommand, Capability, CommandData, CommandRejected, CommandRejectionReason,
    CommandRequest, HandshakeRejected, HandshakeRejectionReason, Message, SIMULATION_VERSION,
    StateChecksum, Tick,
};
use spacegame2d_simulation::{
    MAX_PLAYERS, SimulationConfig,
    command::{Command, PlayerId},
    simulation::SIMULATION_HZ,
    simulation::Simulation,
};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

mod session;
use session::MatchSession;

pub const COMMAND_INPUT_DELAY: Tick = Tick::new(2);

fn build_git_sha(value: Option<&str>) -> &str {
    value.unwrap_or("unknown")
}

struct Client {
    stream: TcpStream,
    address: SocketAddr,
    slot: u32,
    connected: bool,
    display_name: Option<String>,
    decoder: spacegame2d_protocol::FrameDecoder,
    outgoing: VecDeque<Vec<u8>>,
    checksum_enabled: bool,
    pending_checksums: VecDeque<(Tick, Vec<u8>)>,
    seen_checksums: BTreeSet<Tick>,
    closing_after_flush: bool,
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
    let mut match_session = MatchSession::default();
    let mut simulation = Simulation::new(config.clone());
    // Fleet ownership is part of the deterministic world, not connection
    // membership. Every client must see slot 1 as blue and slot 2 as coral
    // from its first snapshot; connection state still gates command validity.
    simulation.world.assign_mirror_owners();
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
            let (mut stream, address) = accepted;
            let Some(slot) = (1..=u32::try_from(MAX_PLAYERS).expect("player count fits u32")).find(
                |candidate| {
                    clients
                        .iter()
                        .all(|client: &Client| client.slot != *candidate)
                },
            ) else {
                tracing::warn!(event = "client_rejected", address = %address, "player slots exhausted");
                let frame = Message::HandshakeRejected(HandshakeRejected {
                    reason: HandshakeRejectionReason::ServerFull,
                })
                .encode()?;
                let _ = stream.write_all(&frame).await;
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
                display_name: None,
                checksum_enabled: false,
                pending_checksums: VecDeque::new(),
                seen_checksums: BTreeSet::new(),
                decoder: spacegame2d_protocol::FrameDecoder::new(),
                outgoing: VecDeque::new(),
                closing_after_flush: false,
            });
        }
        let mut remove = Vec::new();
        let mut broadcasts = Vec::new();
        let mut session_deliveries = Vec::new();
        let mut active_match_ended = false;
        let mut reset_cutover = false;
        for (index, client) in clients.iter_mut().enumerate() {
            match client.read_messages() {
                Ok(messages) => {
                    for message in messages {
                        if !client.connected {
                            let Message::ClientHello(hello) = message else {
                                client.queue(&Message::HandshakeRejected(HandshakeRejected {
                                    reason: HandshakeRejectionReason::InvalidHandshake,
                                }))?;
                                client.closing_after_flush = true;
                                continue;
                            };
                            let display_name = hello.display_name();
                            let reason = if hello.simulation_version != SIMULATION_VERSION {
                                Some(HandshakeRejectionReason::IncompatibleVersion)
                            } else if !hello.capabilities.contains(&Capability::StateChecksums)
                                || !hello.capabilities.contains(&Capability::WorldSnapshots)
                            {
                                Some(HandshakeRejectionReason::MissingRequiredCapability)
                            } else if display_name.is_err() {
                                Some(HandshakeRejectionReason::InvalidHandshake)
                            } else {
                                None
                            };
                            if let Some(reason) = reason {
                                tracing::warn!(event = "handshake_rejected", address = %client.address, ?reason);
                                client.queue(&Message::HandshakeRejected(HandshakeRejected {
                                    reason,
                                }))?;
                                client.closing_after_flush = true;
                                continue;
                            }
                            let display_name: String = display_name
                                .expect("validated display name")
                                .as_str()
                                .into();
                            client.display_name = Some(display_name.clone());
                            client.checksum_enabled = true;
                            client.queue(&Message::ServerHello(
                                spacegame2d_protocol::ServerHello {
                                    simulation_version: SIMULATION_VERSION,
                                    simulation_hz: SIMULATION_HZ,
                                    player_slot: client.slot,
                                    server_tick: simulation.tick(),
                                    fleet_size: config.fleet_size(),
                                    world_radius_bits: config.world_radius_meters().to_bits(),
                                    capabilities: vec![
                                        Capability::StateChecksums,
                                        Capability::WorldSnapshots,
                                    ],
                                },
                            ))?;
                            let pending_commands = scheduled
                                .range(simulation.tick()..)
                                .flat_map(|(_, commands)| commands.iter().cloned())
                                .collect();
                            let initial_world = simulation.initial_world_state(pending_commands);
                            tracing::info!(event = "world_snapshot_queued", tick = ?initial_world.tick, slot = client.slot, units = initial_world.units.len());
                            client.queue(&Message::InitialWorldState(initial_world))?;
                            client.connected = true;
                            session_deliveries.extend(
                                match_session
                                    .accept(client.slot, display_name, simulation.tick())
                                    .map_err(io::Error::other)?,
                            );
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
        for (index, client) in clients.iter_mut().enumerate() {
            if client.closing_after_flush && (client.flush().is_err() || client.outgoing.is_empty())
            {
                remove.push(index);
            }
        }
        remove.sort_unstable();
        remove.dedup();
        for index in remove.into_iter().rev() {
            let client = clients.remove(index);
            if client.connected {
                let departure = match_session.depart(client.slot);
                active_match_ended |= departure.active_match_ended;
                session_deliveries.extend(departure.deliveries);
            }
            if let Some(player_id) = PlayerId::new(u8::try_from(client.slot).unwrap_or(0)) {
                simulation.world.disconnect_player(player_id);
            }
        }
        if active_match_ended {
            scheduled.clear();
            broadcasts.clear();
            simulation.reset_match().map_err(io::Error::other)?;
            state_hashes.clear();
            active_match_ended = false;
        }
        for delivery in session_deliveries {
            if let Some(client) = clients
                .iter_mut()
                .find(|client| client.connected && client.slot == delivery.recipient_slot)
            {
                client.queue(&Message::SessionSnapshot(delivery.snapshot))?;
            }
        }
        for (encoded, cmd, address, slot) in broadcasts {
            let recipients = clients.iter().filter(|peer| peer.connected).count();
            for peer in clients.iter_mut().filter(|peer| peer.connected) {
                peer.outgoing.push_back(encoded.clone());
            }
            tracing::info!(event = "command_broadcast_queued", cmd = %cmd, recipients, address = %address, slot);
        }
        let mut flush_failed = Vec::new();
        for (index, client) in clients.iter_mut().enumerate() {
            if client.flush().is_err() {
                flush_failed.push(index);
            }
        }
        for index in flush_failed.into_iter().rev() {
            let client = clients.remove(index);
            if client.connected {
                let departure = match_session.depart(client.slot);
                active_match_ended |= departure.active_match_ended;
                for delivery in departure.deliveries {
                    if let Some(peer) = clients
                        .iter_mut()
                        .find(|peer| peer.connected && peer.slot == delivery.recipient_slot)
                    {
                        peer.queue(&Message::SessionSnapshot(delivery.snapshot))?;
                    }
                }
            }
            if let Some(player_id) = PlayerId::new(u8::try_from(client.slot).unwrap_or(0)) {
                simulation.world.disconnect_player(player_id);
            }
        }
        if active_match_ended {
            scheduled.clear();
            simulation.reset_match().map_err(io::Error::other)?;
            state_hashes.clear();
        }
        if let Some(commands) = scheduled.remove(&simulation.tick()) {
            for command in commands {
                simulation.schedule_authoritative(&command);
            }
        }
        let mut match_reset = false;
        for event in simulation
            .step()
            .map_err(|error| io::Error::other(error.to_string()))?
        {
            match event {
                spacegame2d_simulation::SimulationEvent::ShotFired {
                    tick,
                    shooter_id,
                    impact_entity,
                    ..
                } => {
                    tracing::info!(event = "shot_fired", tick = ?tick, shooter_id = shooter_id.0, ?impact_entity);
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
                spacegame2d_simulation::SimulationEvent::ObjectiveTransition {
                    tick,
                    owner,
                    relay_id,
                    core_id,
                    previous_state,
                    next_state,
                } => {
                    tracing::info!(event = "objective_transition", tick = ?tick, owner = owner.0, relay_id = relay_id.0, core_id = core_id.0, previous_state = ?previous_state, next_state = ?next_state);
                }
                spacegame2d_simulation::SimulationEvent::CoreHitProtected { tick, core_id } => {
                    tracing::info!(event = "core_hit_protected", tick = ?tick, core_id = core_id.0);
                }
                spacegame2d_simulation::SimulationEvent::MatchResult { tick, outcome } => {
                    match outcome {
                        spacegame2d_simulation::MatchResult::Victory {
                            winner,
                            loser,
                            destroyed_core,
                        } => {
                            tracing::info!(event = "match_victory", tick = ?tick, winner = winner.0, loser = loser.0, destroyed_core = destroyed_core.0)
                        }
                        spacegame2d_simulation::MatchResult::Draw { destroyed_cores } => {
                            tracing::info!(event = "match_draw", tick = ?tick, destroyed_cores = ?destroyed_cores)
                        }
                    }
                    match_reset = true;
                }
            }
        }
        if match_reset {
            scheduled.clear();
            simulation.commands.clear_pending();
        }
        let completed_tick = simulation.tick();
        if completed_tick.0.is_multiple_of(u64::from(SIMULATION_HZ)) {
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
        .unwrap_or_else(|| "0.0.0.0:4000".to_string())
        .parse()
        .expect("invalid bind address");
    let config = match simulation_config_from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("invalid simulation configuration: {error}");
            return;
        }
    };
    tracing::info!(
        event = "server_starting",
        git_sha = build_git_sha(option_env!("SPACEGAME_GIT_SHA")),
        simulation_version = SIMULATION_VERSION,
        address = %address,
        "server starting"
    );
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

    async fn connect_client(
        address: SocketAddr,
        display_name: &str,
    ) -> (
        tokio::net::TcpStream,
        spacegame2d_protocol::ServerHello,
        spacegame2d_protocol::InitialWorldState,
        spacegame2d_protocol::SessionSnapshot,
    ) {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream
            .write_all(
                &Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                    display_name: display_name.into(),
                })
                .encode()
                .unwrap(),
            )
            .await
            .unwrap();
        let Message::ServerHello(hello) = read_message(&mut stream).await else {
            panic!("expected server hello");
        };
        let Message::InitialWorldState(initial_world) = read_message(&mut stream).await else {
            panic!("expected initial world state");
        };
        let Message::SessionSnapshot(session) = read_message(&mut stream).await else {
            panic!("expected session snapshot");
        };
        (stream, hello, initial_world, session)
    }

    fn active_anchor(snapshot: &spacegame2d_protocol::SessionSnapshot) -> Tick {
        match snapshot.match_timing {
            spacegame2d_protocol::MatchTiming::Active { started_at_tick } => started_at_tick,
            spacegame2d_protocol::MatchTiming::Inactive => {
                panic!("expected active match timing")
            }
        }
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
                capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                display_name: "Rook".into(),
            }
            .is_compatible()
        );
        assert!(
            !(ClientHello {
                simulation_version: SIMULATION_VERSION - 1,
                capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                display_name: "Rook".into(),
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
    fn build_git_sha_uses_a_stable_local_fallback() {
        assert_eq!(build_git_sha(Some("0123456789abcdef")), "0123456789abcdef");
        assert_eq!(build_git_sha(None), "unknown");
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
    async fn invalid_display_name_is_rejected_before_acceptance() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;
                let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
                let hello = Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                    display_name: "\u{200d}".into(),
                });
                client.write_all(&hello.encode().unwrap()).await.unwrap();
                assert!(matches!(
                    read_message(&mut client).await,
                    Message::HandshakeRejected(HandshakeRejected {
                        reason: HandshakeRejectionReason::InvalidHandshake
                    })
                ));
                shutdown.send(true).unwrap();
                task.await.unwrap().unwrap();
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_match_lifecycle_preserves_then_replaces_the_start_anchor() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;

                let (mut first, _, _, first_waiting) = connect_client(address, "Rook").await;
                assert!(matches!(
                    first_waiting.match_timing,
                    spacegame2d_protocol::MatchTiming::Inactive
                ));

                let (mut second, _, _, second_active) = connect_client(address, "Nova").await;
                let Message::SessionSnapshot(first_active) = read_message(&mut first).await else {
                    panic!("expected first participant's active snapshot");
                };
                let original_anchor = active_anchor(&second_active);
                assert_eq!(active_anchor(&first_active), original_anchor);

                tokio::time::sleep(Duration::from_millis(50)).await;
                second.shutdown().await.unwrap();
                let Message::SessionSnapshot(first_disconnected) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!("expected survivor's disconnected snapshot");
                };
                assert_eq!(active_anchor(&first_disconnected), original_anchor);

                let (mut replacement, replacement_hello, _, replacement_active) =
                    connect_client(address, "Echo").await;
                let Message::SessionSnapshot(first_reconnected) = read_message(&mut first).await
                else {
                    panic!("expected survivor's replacement snapshot");
                };
                assert_eq!(active_anchor(&replacement_active), original_anchor);
                assert_eq!(active_anchor(&first_reconnected), original_anchor);
                assert!(replacement_hello.server_tick > original_anchor);
                assert!(replacement_hello.server_tick - original_anchor > Tick::from(0));

                first.shutdown().await.unwrap();
                replacement.shutdown().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                let (mut next_first, _, _, next_waiting) = connect_client(address, "Rook").await;
                assert!(matches!(
                    next_waiting.match_timing,
                    spacegame2d_protocol::MatchTiming::Inactive
                ));
                let (mut next_second, _, _, next_second_active) =
                    connect_client(address, "Nova").await;
                let Message::SessionSnapshot(next_first_active) =
                    read_message(&mut next_first).await
                else {
                    panic!("expected next match snapshot");
                };
                let next_anchor = active_anchor(&next_second_active);
                assert_eq!(active_anchor(&next_first_active), next_anchor);
                assert!(next_anchor > original_anchor);

                next_first.shutdown().await.unwrap();
                next_second.shutdown().await.unwrap();
                shutdown.send(true).unwrap();
                task.await.unwrap().unwrap();
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_all_departures_reset_the_world_before_the_next_match() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;
                let (mut first, _, _, _) = connect_client(address, "Rook").await;
                let (mut second, _, _, _) = connect_client(address, "Nova").await;
                let Message::SessionSnapshot(_) = read_message(&mut first).await else {
                    panic!("expected first participant's active snapshot");
                };

                first
                    .write_all(
                        &Message::CommandRequest(CommandRequest {
                            sequence: 1,
                            command: CommandData::SetDestination {
                                destination: [40.0f32.to_bits(), 10.0f32.to_bits()],
                            },
                        })
                        .encode()
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                let Message::AuthoritativeCommand(_) = read_message(&mut first).await else {
                    panic!("expected command broadcast to first participant");
                };
                let Message::AuthoritativeCommand(_) = read_message(&mut second).await else {
                    panic!("expected command broadcast to second participant");
                };
                tokio::time::sleep(Duration::from_millis(100)).await;

                first.shutdown().await.unwrap();
                second.shutdown().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                let (mut next_first, _, next_world, _) = connect_client(address, "Rook").await;
                assert!(
                    next_world
                        .units
                        .iter()
                        .all(|unit| unit.destination_bits.is_none())
                );

                next_first.shutdown().await.unwrap();
                shutdown.send(true).unwrap();
                task.await.unwrap().unwrap();
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn real_tcp_handshake_tick_advancement_broadcast_and_disconnect() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;
                let mut first = tokio::net::TcpStream::connect(address).await.unwrap();
                let hello = Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                    display_name: "Rook".into(),
                });
                first.write_all(&hello.encode().unwrap()).await.unwrap();
                let Message::ServerHello(first_hello) = read_message(&mut first).await else {
                    panic!()
                };
                let Message::InitialWorldState(first_snapshot) = read_message(&mut first).await
                else {
                    panic!()
                };
                let Message::SessionSnapshot(first_session) = read_message(&mut first).await else {
                    panic!()
                };
                assert!(matches!(
                    first_session.opponent_presence,
                    spacegame2d_protocol::OpponentPresence::Waiting
                ));

                tokio::time::sleep(Duration::from_millis(50)).await;
                let mut second = tokio::net::TcpStream::connect(address).await.unwrap();
                let second_hello_request = Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
                    display_name: "Cafe\u{301}".into(),
                });
                second
                    .write_all(&second_hello_request.encode().unwrap())
                    .await
                    .unwrap();
                let Message::ServerHello(second_hello) = read_message(&mut second).await else {
                    panic!()
                };
                let Message::InitialWorldState(second_snapshot) = read_message(&mut second).await
                else {
                    panic!()
                };
                let Message::SessionSnapshot(second_session) = read_message(&mut second).await
                else {
                    panic!()
                };
                assert_eq!(first_hello.player_slot, 1);
                assert_eq!(second_hello.player_slot, 2);
                first_hello.validate(SIMULATION_HZ).unwrap();
                second_hello.validate(SIMULATION_HZ).unwrap();
                assert!(second_hello.server_tick > first_hello.server_tick);
                let Message::SessionSnapshot(first_update) =
                    tokio::time::timeout(Duration::from_secs(1), read_message(&mut first))
                        .await
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(first_update.participants, second_session.participants);
                assert_eq!(
                    first_update
                        .participants
                        .iter()
                        .map(|participant| participant.display_name.as_str())
                        .collect::<Vec<_>>(),
                    vec!["Rook", "Café"]
                );
                assert!(matches!(
                    first_update.opponent_presence,
                    spacegame2d_protocol::OpponentPresence::Present
                ));

                // Ownership is match state, not connection state: player one
                // must render player two's fleet as coral before player two has
                // connected, otherwise the two simulations diverge.
                let first_client_mirror = Simulation::restore_initial_world_state(
                    &first_snapshot,
                    SimulationConfig::default(),
                )
                .unwrap();
                assert!(
                    first_client_mirror.world.units[..FLEET_SIZE]
                        .iter()
                        .all(|unit| unit.owner == Some(PlayerId(1)))
                );
                assert!(
                    first_client_mirror.world.units[FLEET_SIZE..]
                        .iter()
                        .all(|unit| unit.owner == Some(PlayerId(2)))
                );

                // The late joiner restores the authoritative state instead of a
                // locally generated default world.
                let late_joiner_mirror = Simulation::restore_initial_world_state(
                    &second_snapshot,
                    SimulationConfig::default(),
                )
                .unwrap();
                assert_eq!(late_joiner_mirror.world.units.len(), MAX_UNITS);
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
                    .unit(UnitId(player_one_mirror.config().fleet_size() + 1))
                    .expect("first Player 2 unit in Player 1 mirror");
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
                let Message::HandshakeRejected(rejection) =
                    tokio::time::timeout(Duration::from_millis(200), try_read_message(&mut third))
                        .await
                        .unwrap()
                        .unwrap()
                else {
                    panic!()
                };
                assert_eq!(rejection.reason, HandshakeRejectionReason::ServerFull);

                first.shutdown().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                let mut recycled = tokio::net::TcpStream::connect(address).await.unwrap();
                recycled.write_all(&hello.encode().unwrap()).await.unwrap();
                let Message::ServerHello(recycled_hello) = read_message(&mut recycled).await else {
                    panic!()
                };
                assert_eq!(recycled_hello.player_slot, 1);
                let Message::InitialWorldState(_) = read_message(&mut recycled).await else {
                    panic!()
                };
                let Message::SessionSnapshot(_) = read_message(&mut recycled).await else {
                    panic!()
                };

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
                let mut command_after_disconnect = None;
                for _ in 0..3 {
                    let message =
                        tokio::time::timeout(Duration::from_secs(1), read_message(&mut second))
                            .await
                            .unwrap();
                    if let Message::AuthoritativeCommand(command) = message {
                        command_after_disconnect = Some(command);
                        break;
                    }
                }
                let command_after_disconnect =
                    command_after_disconnect.expect("reset command broadcast");
                assert_eq!(command_after_disconnect.sequence, 8);

                shutdown.send(true).unwrap();
                task.await.unwrap().unwrap();
            })
            .await;
    }
}
