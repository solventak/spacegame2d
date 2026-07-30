use std::io;

use prost::Message as ProstMessage;

use crate::wire;
use crate::*;

#[test]
fn handshake_rejections_bump_simulation_version() {
    assert_eq!(SIMULATION_VERSION, 19);
}

#[test]
fn display_names_are_canonicalized_and_validated() {
    assert_eq!(
        DisplayName::try_from("  Cafe\u{301}  ").unwrap().as_str(),
        "Café"
    );
    assert_eq!(
        DisplayName::try_from("\u{1f680}").unwrap().as_str(),
        "\u{1f680}"
    );
    assert_eq!(
        DisplayName::try_from("   ").unwrap_err(),
        DisplayNameError::Required
    );
    assert_eq!(
        DisplayName::try_from("A\u{200d}B").unwrap_err(),
        DisplayNameError::ContainsControlOrFormat
    );
    assert_eq!(
        DisplayName::try_from("x".repeat(25)).unwrap_err(),
        DisplayNameError::TooLong
    );
}

fn destination() -> CommandData {
    CommandData::SetDestination {
        destination: [0x8000_0000, 0x0000_0001],
    }
}

#[test]
fn all_messages_round_trip() {
    let messages = [
        Message::ClientHello(ClientHello {
            simulation_version: SIMULATION_VERSION,
            capabilities: vec![Capability::StateChecksums, Capability::WorldSnapshots],
            display_name: "Rook".into(),
        }),
        Message::ServerHello(ServerHello {
            simulation_version: SIMULATION_VERSION,
            simulation_hz: 60,
            player_slot: 2,
            server_tick: Tick::from(9),
            fleet_size: 30,
            world_radius_bits: 64.0_f32.to_bits(),
            capabilities: vec![],
        }),
        Message::CommandRequest(CommandRequest {
            sequence: 4,
            command: destination(),
        }),
        Message::AuthoritativeCommand(AuthoritativeCommand {
            execute_tick: Tick::from(11),
            player_slot: 2,
            sequence: 4,
            command: CommandData::ResetSimulation,
        }),
        Message::CommandRejected(CommandRejected {
            sequence: 8,
            reason: CommandRejectionReason::DestinationOutsideArena,
        }),
        Message::StateChecksum(StateChecksum {
            tick: Tick::from(60),
            hash: (0..32).collect(),
        }),
        Message::HandshakeRejected(HandshakeRejected {
            reason: HandshakeRejectionReason::ServerFull,
        }),
        Message::SessionSnapshot(SessionSnapshot {
            local_player_slot: 1,
            participants: vec![SessionParticipant {
                player_slot: 1,
                display_name: "Rook".into(),
                color: PlayerColor::Cyan,
            }],
            opponent_presence: OpponentPresence::Waiting,
            presence_revision: 0,
            match_timing: MatchTiming::Inactive,
        }),
    ];
    for message in messages {
        let mut bytes = Vec::new();
        message.write(&mut bytes).unwrap();
        assert_eq!(Message::read(&mut bytes.as_slice()).unwrap(), message);
    }
}

#[test]
fn session_snapshot_rejects_noncanonical_or_inconsistent_state() {
    let mut snapshot = SessionSnapshot {
        local_player_slot: 1,
        participants: vec![SessionParticipant {
            player_slot: 1,
            display_name: "Cafe\u{301}".into(),
            color: PlayerColor::Cyan,
        }],
        opponent_presence: OpponentPresence::Waiting,
        presence_revision: 0,
        match_timing: MatchTiming::Inactive,
    };
    assert_eq!(
        snapshot.validate().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    snapshot.participants[0].display_name = "Café".into();
    snapshot.opponent_presence = OpponentPresence::Present;
    assert_eq!(
        snapshot.validate().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
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
    let mut frame = Vec::new();
    wire::Envelope::default().encode(&mut frame).unwrap();
    let mut bytes = (frame.len() as u32).to_be_bytes().to_vec();
    bytes.extend(frame);
    assert_eq!(
        Message::read(&mut bytes.as_slice()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn state_checksum_accepts_malformed_for_server_validation() {
    let envelope = wire::Envelope {
        payload: Some(wire::envelope::Payload::StateChecksum(
            wire::StateChecksum {
                tick: 60,
                hash: vec![0; 31],
            },
        )),
    };
    let mut body = Vec::new();
    envelope.encode(&mut body).unwrap();
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend(body);
    assert!(
        matches!(Message::read(&mut bytes.as_slice()).unwrap(), Message::StateChecksum(StateChecksum { hash, .. }) if hash.len() == 31)
    );
}

#[test]
fn missing_command_payload_reaches_empty_command_branch() {
    let bytes = [0, 0, 0, 6, 0x1a, 0x04, 0x08, 0x01, 0x12, 0x00];

    let error = Message::read(&mut bytes.as_slice()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "missing command payload");
}

#[test]
fn exact_float_bits_and_unknown_capabilities_survive() {
    let message = Message::CommandRequest(CommandRequest {
        sequence: 2,
        command: CommandData::SetDestination {
            destination: [f32::NAN.to_bits(), f32::INFINITY.to_bits()],
        },
    });
    let decoded = Message::read(&mut message.encode().unwrap().as_slice()).unwrap();
    assert_eq!(decoded, message);
    let envelope = wire::Envelope {
        payload: Some(wire::envelope::Payload::ClientHello(wire::ClientHello {
            simulation_version: SIMULATION_VERSION,
            supported_capabilities: vec![1, 999],
            display_name: "Rook".into(),
        })),
    };
    let mut body = Vec::new();
    envelope.encode(&mut body).unwrap();
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend(body);
    assert_eq!(
        Message::read(&mut bytes.as_slice()).unwrap(),
        Message::ClientHello(ClientHello {
            simulation_version: SIMULATION_VERSION,
            capabilities: vec![Capability::StateChecksums],
            display_name: "Rook".into(),
        })
    );
}
