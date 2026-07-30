mod command;
mod error;
mod framing;
mod handshake;
mod identity;
mod message;
mod session;
mod snapshot;
#[cfg(test)]
mod tests;
mod tick;
mod wire;

pub use command::{
    AuthoritativeCommand, CommandData, CommandRejected, CommandRejectionReason, CommandRequest,
};
pub use framing::{FrameDecoder, MAX_FRAME_BYTES};
pub use handshake::{
    Capability, ClientHello, HandshakeRejected, HandshakeRejectionReason, ServerHello,
};
pub use identity::{DisplayName, DisplayNameError, MAX_DISPLAY_NAME_GRAPHEMES};
pub use message::Message;
pub use session::{
    MatchTiming, OpponentPresence, PlayerColor, SessionParticipant, SessionSnapshot,
};
pub use snapshot::{InitialWorldState, StateChecksum, WorldUnit};
pub use tick::Tick;

pub const SIMULATION_VERSION: u32 = 19;
