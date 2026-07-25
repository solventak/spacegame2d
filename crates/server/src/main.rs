use std::{
    collections::{BTreeMap, VecDeque},
    env, io,
    net::SocketAddr,
    time::Duration,
};

use spacegame2d_protocol::{
    AuthoritativeCommand, ClientHello, CommandRequest, Message, SIMULATION_VERSION, encode_message,
};
use spacegame2d_simulation::{
    command::{PlayerId, command_from_data, valid_authoritative},
    simulation::SIMULATION_HZ,
    simulation::Simulation,
};
use tokio::net::{TcpListener, TcpStream};

pub const COMMAND_INPUT_DELAY: u64 = 2;

struct Client {
    stream: TcpStream,
    address: SocketAddr,
    slot: u32,
    connected: bool,
    decoder: spacegame2d_protocol::FrameDecoder,
    outgoing: VecDeque<Vec<u8>>,
}

fn kind(command: &spacegame2d_protocol::CommandData) -> &'static str {
    match command {
        spacegame2d_protocol::CommandData::SetDestination { .. } => "set_destination",
        spacegame2d_protocol::CommandData::ResetSimulation => "reset_simulation",
    }
}

fn valid_hello(hello: &ClientHello) -> bool {
    hello.simulation_version == SIMULATION_VERSION
}

fn execute_tick(receive_tick: u64) -> u64 {
    receive_tick.saturating_add(COMMAND_INPUT_DELAY)
}

fn valid_request(simulation: &Simulation, slot: u32, request: &CommandRequest) -> bool {
    let command = AuthoritativeCommand {
        execute_tick: 0,
        player_slot: slot,
        sequence: request.sequence,
        command: request.command.clone(),
    };
    valid_authoritative(simulation.world(), &command)
        && command_from_data(&request.command).is_some()
}

async fn handle_read(client: &mut Client, simulation: &mut Simulation) -> io::Result<Vec<Message>> {
    let mut bytes = [0u8; 4096];
    let mut messages = Vec::new();
    loop {
        match client.stream.try_read(&mut bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "client disconnected",
                ));
            }
            Ok(size) => messages.extend(client.decoder.push(&bytes[..size])?),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    let _ = simulation;
    Ok(messages)
}

fn queue(client: &mut Client, message: &Message) -> io::Result<()> {
    client.outgoing.push_back(encode_message(message)?);
    Ok(())
}

fn flush(client: &mut Client) -> io::Result<()> {
    while let Some(frame) = client.outgoing.front_mut() {
        match client.stream.try_write(frame) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "client write closed",
                ));
            }
            Ok(size) if size == frame.len() => {
                client.outgoing.pop_front();
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

async fn run(address: SocketAddr) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let bound = listener.local_addr()?;
    tracing::info!(event = "server_listening", address = %bound, "server listening");
    let mut clients = Vec::new();
    let mut next_slot = 1u32;
    let mut simulation = Simulation::default();
    let mut scheduled = BTreeMap::<u64, Vec<AuthoritativeCommand>>::new();
    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / SIMULATION_HZ as f64));
    loop {
        interval.tick().await;
        loop {
            let accepted = tokio::select! {
                biased;
                result = listener.accept() => Some(result?),
                _ = tokio::task::yield_now() => None,
            };
            let Some((stream, address)) = accepted else {
                break;
            };
            let Some(player_id) = u8::try_from(next_slot).ok().and_then(PlayerId::new) else {
                tracing::warn!(event = "client_rejected", address = %address, slot = next_slot, "player slot exhausted");
                continue;
            };
            stream.set_nodelay(true)?;
            if let Some(unit) = simulation
                .world
                .units
                .iter_mut()
                .find(|unit| unit.owner.is_none())
            {
                unit.owner = Some(player_id);
            }
            clients.push(Client {
                stream,
                address,
                slot: next_slot,
                connected: false,
                decoder: spacegame2d_protocol::FrameDecoder::new(),
                outgoing: VecDeque::new(),
            });
            next_slot = next_slot.saturating_add(1);
        }
        let mut remove = Vec::new();
        let mut broadcasts = Vec::new();
        for (index, client) in clients.iter_mut().enumerate() {
            match handle_read(client, &mut simulation).await {
                Ok(messages) => {
                    for message in messages {
                        if !client.connected {
                            let Message::ClientHello(hello) = message else {
                                remove.push(index);
                                continue;
                            };
                            if !valid_hello(&hello) {
                                tracing::warn!(event = "handshake_rejected", address = %client.address, "wrong simulation version");
                                remove.push(index);
                                continue;
                            }
                            queue(
                                client,
                                &Message::ServerHello(spacegame2d_protocol::ServerHello {
                                    simulation_version: SIMULATION_VERSION,
                                    simulation_hz: SIMULATION_HZ,
                                    player_slot: client.slot,
                                    server_tick: simulation.tick(),
                                    capabilities: Vec::new(),
                                }),
                            )?;
                            client.connected = true;
                        } else if let Message::CommandRequest(request) = message {
                            let receive_tick = simulation.tick();
                            let cmd = format!("{}:{}", client.slot, request.sequence);
                            tracing::info!(event = "command_received", cmd = %cmd, tick = receive_tick, kind = kind(&request.command), address = %client.address, slot = client.slot);
                            if !valid_request(&simulation, client.slot, &request) {
                                tracing::warn!(event = "command_rejected", cmd = %cmd, tick = receive_tick, address = %client.address, slot = client.slot, "invalid command");
                                continue;
                            }
                            let authoritative = AuthoritativeCommand {
                                execute_tick: execute_tick(receive_tick),
                                player_slot: client.slot,
                                sequence: request.sequence,
                                command: request.command,
                            };
                            tracing::info!(event = "command_scheduled", cmd = %cmd, receive_tick, execute_tick = authoritative.execute_tick, tick = receive_tick, kind = kind(&authoritative.command), address = %client.address, slot = client.slot);
                            let encoded = encode_message(&Message::AuthoritativeCommand(
                                authoritative.clone(),
                            ))?;
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
            clients.remove(index);
        }
        for (encoded, cmd, address, slot) in broadcasts {
            let recipients = clients.iter().filter(|peer| peer.connected).count();
            for peer in clients.iter_mut().filter(|peer| peer.connected) {
                peer.outgoing.push_back(encoded.clone());
            }
            tracing::info!(event = "command_broadcast_queued", cmd = %cmd, recipients, address = %address, slot);
        }
        for client in &mut clients {
            if flush(client).is_err() {
                client.connected = false;
            }
        }
        if let Some(commands) = scheduled.remove(&simulation.tick()) {
            for command in commands {
                simulation.schedule_authoritative(&command);
            }
        }
        for event in simulation.step() {
            let spacegame2d_simulation::SimulationEvent::BoundaryCrossed {
                tick,
                unit_id,
                position,
            } = event;
            tracing::info!(event = "boundary_crossed", tick, unit_id = unit_id.0, position = ?position);
        }
    }
}

#[tokio::main]
async fn main() {
    let _logging = spacegame2d_logging::init("spacegame2d-server", "info")
        .expect("failed to initialize logging");
    let address = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid bind address");
    if let Err(error) = run(address).await {
        tracing::error!(event = "server_stopped", error = %error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handshake_validation() {
        assert!(valid_hello(&ClientHello {
            simulation_version: 1,
            capabilities: vec![]
        }));
        assert!(!valid_hello(&ClientHello {
            simulation_version: 2,
            capabilities: vec![]
        }));
    }
    #[test]
    fn scheduling_tick_math() {
        assert_eq!(execute_tick(40), 42);
    }
    #[test]
    fn ownership_and_nan_are_rejected() {
        let sim = Simulation::default();
        let request = CommandRequest {
            sequence: 1,
            command: spacegame2d_protocol::CommandData::SetDestination {
                unit_id: 1,
                destination: [f32::NAN.to_bits(), 0],
            },
        };
        assert!(!valid_request(&sim, 1, &request));
    }
}
