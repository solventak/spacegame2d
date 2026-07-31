use std::io::{self, Read, Write};

use prost::Message as ProstMessage;

use crate::command::{AuthoritativeCommand, CommandRejected, CommandRequest};
use crate::error::invalid;
use crate::framing::MAX_FRAME_BYTES;
use crate::handshake::{ClientHello, HandshakeRejected, ServerHello};
use crate::session::SessionSnapshot;
use crate::snapshot::{InitialWorldState, StateChecksum};
use crate::wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    ClientHello(ClientHello),
    ServerHello(ServerHello),
    CommandRequest(CommandRequest),
    AuthoritativeCommand(AuthoritativeCommand),
    CommandRejected(CommandRejected),
    StateChecksum(StateChecksum),
    HandshakeRejected(HandshakeRejected),
    InitialWorldState(InitialWorldState),
    SessionSnapshot(SessionSnapshot),
}

impl Message {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut body = Vec::new();
        wire::Envelope::from(self)
            .encode(&mut body)
            .map_err(|error| invalid(&error.to_string()))?;
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

    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut header = [0; 4];
        reader.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header);
        if length == 0 || length > MAX_FRAME_BYTES {
            return Err(invalid("invalid frame size"));
        }
        let mut body = vec![0; length as usize];
        reader.read_exact(&mut body)?;
        Self::decode_body(&body)
    }

    pub(crate) fn decode_body(body: &[u8]) -> io::Result<Self> {
        Self::try_from(wire::Envelope::decode(body).map_err(|error| invalid(&error.to_string()))?)
    }
}
