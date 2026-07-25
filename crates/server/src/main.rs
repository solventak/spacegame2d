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
use tokio::sync::watch;

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

fn handle_read(client: &mut Client) -> io::Result<Vec<Message>> {
    let mut bytes = [0u8; 4096];
    let mut messages = Vec::new();
    loop {
        match client.stream.try_read(&mut bytes) {
            Ok(0) => {
                if messages.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "client disconnected",
                    ));
                }
                break;
            }
            Ok(size) => messages.extend(client.decoder.push(&bytes[..size])?),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
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

pub async fn run(listener: TcpListener, mut shutdown: watch::Receiver<bool>) -> io::Result<()> {
    let bound = listener.local_addr()?;
    tracing::info!(event = "server_listening", address = %bound, "server listening");
    let mut clients = Vec::new();
    let mut simulation = Simulation::default();
    let mut scheduled = BTreeMap::<u64, Vec<AuthoritativeCommand>>::new();
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
            let Some(slot) = (1..=u32::from(u8::MAX)).find(|candidate| {
                clients
                    .iter()
                    .all(|client: &Client| client.slot != *candidate)
            }) else {
                tracing::warn!(event = "client_rejected", address = %address, "player slots exhausted");
                continue;
            };
            let player_id = PlayerId::new(u8::try_from(slot).expect("slot is within u8 range"))
                .expect("slot is nonzero");
            stream.set_nodelay(true)?;
            simulation.world.connect_player(player_id);
            simulation.world.assign_player_unit(player_id);
            clients.push(Client {
                stream,
                address,
                slot,
                connected: false,
                decoder: spacegame2d_protocol::FrameDecoder::new(),
                outgoing: VecDeque::new(),
            });
        }
        let mut remove = Vec::new();
        let mut broadcasts = Vec::new();
        for (index, client) in clients.iter_mut().enumerate() {
            match handle_read(client) {
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
    let address: SocketAddr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid bind address");
    let listener = match TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(event = "server_stopped", error = %error);
            return;
        }
    };
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    if let Err(error) = run(listener, shutdown_rx).await {
        tracing::error!(event = "server_stopped", error = %error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_protocol::{FrameDecoder, encode_message};
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

    #[tokio::test(flavor = "current_thread")]
    async fn real_tcp_handshake_tick_advancement_broadcast_and_disconnect() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (address, shutdown, task) = start_server().await;
                let mut first = tokio::net::TcpStream::connect(address).await.unwrap();
                let hello = Message::ClientHello(ClientHello {
                    simulation_version: SIMULATION_VERSION,
                    capabilities: vec![],
                });
                first
                    .write_all(&encode_message(&hello).unwrap())
                    .await
                    .unwrap();
                let Message::ServerHello(first_hello) = read_message(&mut first).await else {
                    panic!()
                };

                tokio::time::sleep(Duration::from_millis(50)).await;
                let mut second = tokio::net::TcpStream::connect(address).await.unwrap();
                second
                    .write_all(&encode_message(&hello).unwrap())
                    .await
                    .unwrap();
                let Message::ServerHello(second_hello) = read_message(&mut second).await else {
                    panic!()
                };
                assert_eq!(first_hello.player_slot, 1);
                assert_eq!(second_hello.player_slot, 2);
                assert!(second_hello.server_tick > first_hello.server_tick);

                let request = Message::CommandRequest(CommandRequest {
                    sequence: 7,
                    command: spacegame2d_protocol::CommandData::ResetSimulation,
                });
                second
                    .write_all(&encode_message(&request).unwrap())
                    .await
                    .unwrap();
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

                first.shutdown().await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;

                let mut recycled = tokio::net::TcpStream::connect(address).await.unwrap();
                recycled
                    .write_all(&encode_message(&hello).unwrap())
                    .await
                    .unwrap();
                let Message::ServerHello(recycled_hello) = read_message(&mut recycled).await else {
                    panic!()
                };
                assert_eq!(recycled_hello.player_slot, 1);

                second
                    .write_all(
                        &encode_message(&Message::CommandRequest(CommandRequest {
                            sequence: 8,
                            command: spacegame2d_protocol::CommandData::ResetSimulation,
                        }))
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
