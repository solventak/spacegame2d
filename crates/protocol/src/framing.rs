use std::io;

use crate::error::invalid;
use crate::message::Message;

pub const MAX_FRAME_BYTES: u32 = 1024 * 1024;

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
            messages.push(Message::decode_body(&body)?);
        }
        Ok(messages)
    }
}
