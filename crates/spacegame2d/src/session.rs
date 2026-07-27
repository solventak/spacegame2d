use std::time::{Duration, Instant};

use spacegame2d_ui_protocol::{
    ConnectionFailureReason, ConnectionStateSnapshot, DisconnectedReason, LocalPlayerHudModel,
    PlayerColor, RequestId,
};

pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:4000";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionAttempt {
    pub id: RequestId,
    pub address: String,
    pub deadline: Instant,
}

pub enum ConnectionOutcome<S> {
    Connected { session: S, player_slot: u32 },
    Rejected(HandshakeOutcome),
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakeOutcome {
    ServerFull,
    VersionMismatch,
    Rejected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionProgress {
    OpeningSocket,
    Handshaking,
}

enum SessionPhase<S> {
    Idle {
        address: String,
        reason: DisconnectedReason,
    },
    ResolvingHost(ConnectionAttempt),
    OpeningSocket(ConnectionAttempt),
    Handshaking(ConnectionAttempt),
    Connected {
        request_id: RequestId,
        address: String,
        player_slot: u32,
        session: S,
    },
    Failed {
        request_id: RequestId,
        address: String,
        reason: ConnectionFailureReason,
    },
}

pub struct SessionLifecycle<S> {
    phase: SessionPhase<S>,
}

impl<S> SessionLifecycle<S> {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            phase: SessionPhase::Idle {
                address: address.into(),
                reason: DisconnectedReason::Startup,
            },
        }
    }
    pub fn connect(
        &mut self,
        id: RequestId,
        address: String,
        now: Instant,
    ) -> Option<ConnectionAttempt> {
        if matches!(
            self.phase,
            SessionPhase::ResolvingHost(_)
                | SessionPhase::OpeningSocket(_)
                | SessionPhase::Handshaking(_)
                | SessionPhase::Connected { .. }
        ) {
            return None;
        }
        let attempt = ConnectionAttempt {
            id,
            address,
            deadline: now + CONNECTION_TIMEOUT,
        };
        self.phase = SessionPhase::ResolvingHost(attempt.clone());
        Some(attempt)
    }
    pub fn progress(&mut self, id: &RequestId, progress: ConnectionProgress) -> bool {
        let attempt = match &self.phase {
            SessionPhase::ResolvingHost(attempt) | SessionPhase::OpeningSocket(attempt) => attempt,
            _ => return false,
        };
        if &attempt.id != id {
            return false;
        }
        let next = attempt.clone();
        self.phase = match progress {
            ConnectionProgress::OpeningSocket
                if matches!(self.phase, SessionPhase::ResolvingHost(_)) =>
            {
                SessionPhase::OpeningSocket(next)
            }
            ConnectionProgress::Handshaking
                if matches!(self.phase, SessionPhase::OpeningSocket(_)) =>
            {
                SessionPhase::Handshaking(next)
            }
            _ => return false,
        };
        true
    }
    pub fn cancel(&mut self, id: &RequestId) -> bool {
        let attempt = match &self.phase {
            SessionPhase::ResolvingHost(attempt)
            | SessionPhase::OpeningSocket(attempt)
            | SessionPhase::Handshaking(attempt)
                if &attempt.id == id =>
            {
                attempt.clone()
            }
            _ => return false,
        };
        self.phase = SessionPhase::Idle {
            address: attempt.address,
            reason: DisconnectedReason::Cancelled,
        };
        true
    }
    pub fn timeout(&mut self, now: Instant) -> bool {
        let attempt = match &self.phase {
            SessionPhase::ResolvingHost(attempt)
            | SessionPhase::OpeningSocket(attempt)
            | SessionPhase::Handshaking(attempt)
                if now >= attempt.deadline =>
            {
                attempt.clone()
            }
            _ => return false,
        };
        self.phase = SessionPhase::Failed {
            request_id: attempt.id,
            address: attempt.address,
            reason: ConnectionFailureReason::Timeout,
        };
        true
    }
    pub fn complete(&mut self, id: RequestId, outcome: ConnectionOutcome<S>) -> bool {
        let attempt = match &self.phase {
            SessionPhase::ResolvingHost(attempt)
            | SessionPhase::OpeningSocket(attempt)
            | SessionPhase::Handshaking(attempt)
                if attempt.id == id =>
            {
                attempt.clone()
            }
            _ => return false,
        };
        self.phase = match outcome {
            ConnectionOutcome::Connected {
                session,
                player_slot,
            } => SessionPhase::Connected {
                request_id: attempt.id,
                address: attempt.address,
                player_slot,
                session,
            },
            ConnectionOutcome::Rejected(HandshakeOutcome::ServerFull) => SessionPhase::Failed {
                request_id: attempt.id,
                address: attempt.address,
                reason: ConnectionFailureReason::ServerFull,
            },
            ConnectionOutcome::Rejected(HandshakeOutcome::VersionMismatch) => {
                SessionPhase::Failed {
                    request_id: attempt.id,
                    address: attempt.address,
                    reason: ConnectionFailureReason::VersionMismatch,
                }
            }
            ConnectionOutcome::Rejected(HandshakeOutcome::Rejected) => SessionPhase::Failed {
                request_id: attempt.id,
                address: attempt.address,
                reason: ConnectionFailureReason::Rejected,
            },
            ConnectionOutcome::Failed => SessionPhase::Failed {
                request_id: attempt.id,
                address: attempt.address,
                reason: ConnectionFailureReason::Network,
            },
        };
        true
    }
    pub fn session_lost(&mut self) -> bool {
        let (address, _) = match &self.phase {
            SessionPhase::Connected {
                address,
                request_id,
                ..
            } => (address.clone(), request_id),
            _ => return false,
        };
        self.phase = SessionPhase::Idle {
            address,
            reason: DisconnectedReason::SessionLost,
        };
        true
    }
    pub fn next_deadline(&self) -> Option<Instant> {
        match &self.phase {
            SessionPhase::ResolvingHost(attempt)
            | SessionPhase::OpeningSocket(attempt)
            | SessionPhase::Handshaking(attempt) => Some(attempt.deadline),
            _ => None,
        }
    }
    pub fn is_connected(&self) -> bool {
        match &self.phase {
            SessionPhase::Connected { session, .. } => {
                let _ = session;
                true
            }
            _ => false,
        }
    }
    pub fn ui_state(&self) -> ConnectionStateSnapshot {
        match &self.phase {
            SessionPhase::Idle { address, reason } => ConnectionStateSnapshot::Idle {
                address: address.clone(),
                reason: *reason,
            },
            SessionPhase::ResolvingHost(attempt) => ConnectionStateSnapshot::ResolvingHost {
                request_id: attempt.id.clone(),
                address: attempt.address.clone(),
            },
            SessionPhase::OpeningSocket(attempt) => ConnectionStateSnapshot::OpeningSocket {
                request_id: attempt.id.clone(),
                address: attempt.address.clone(),
            },
            SessionPhase::Handshaking(attempt) => ConnectionStateSnapshot::Handshaking {
                request_id: attempt.id.clone(),
                address: attempt.address.clone(),
            },
            SessionPhase::Connected {
                request_id,
                address,
                player_slot,
                ..
            } => ConnectionStateSnapshot::Connected {
                request_id: request_id.clone(),
                address: address.clone(),
                local_player: local_player(*player_slot),
            },
            SessionPhase::Failed {
                request_id,
                address,
                reason,
            } => ConnectionStateSnapshot::Failed {
                request_id: request_id.clone(),
                address: address.clone(),
                reason: *reason,
            },
        }
    }
}

fn local_player(player_slot: u32) -> LocalPlayerHudModel {
    let color = if player_slot == 2 {
        PlayerColor::Coral
    } else {
        PlayerColor::Cyan
    };
    let color_hex = match color {
        PlayerColor::Cyan => "#22CFE8",
        PlayerColor::Coral => "#FF6A47",
    };
    LocalPlayerHudModel {
        schema_version: 1,
        player_slot,
        color,
        color_hex: color_hex.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> RequestId {
        RequestId::new(value.into()).unwrap()
    }
    #[test]
    fn cancellation_and_stale_completion_are_ignored() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("x");
        let attempt = lifecycle.connect(id("one"), "a".into(), now).unwrap();
        assert!(lifecycle.cancel(&id("one")));
        assert!(!lifecycle.complete(id("one"), ConnectionOutcome::Failed));
        assert!(matches!(
            lifecycle.ui_state(),
            ConnectionStateSnapshot::Idle {
                reason: DisconnectedReason::Cancelled,
                ..
            }
        ));
        let next = lifecycle.connect(id("two"), "b".into(), now).unwrap();
        assert_ne!(attempt.id, next.id);
    }
    #[test]
    fn progress_is_ordered_and_request_scoped() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("x");
        lifecycle.connect(id("one"), "a".into(), now);
        assert!(!lifecycle.progress(&id("two"), ConnectionProgress::OpeningSocket));
        assert!(lifecycle.progress(&id("one"), ConnectionProgress::OpeningSocket));
        assert!(lifecycle.progress(&id("one"), ConnectionProgress::Handshaking));
        assert!(!lifecycle.progress(&id("one"), ConnectionProgress::OpeningSocket));
    }
}
