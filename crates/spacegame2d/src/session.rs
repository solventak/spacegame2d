use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::hud::LocalPlayerHudModel;

pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

pub const DEFAULT_SERVER_ADDRESS: &str = "127.0.0.1:4000";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UiCommand {
    Connect { schema_version: u8, address: String },
    Cancel { schema_version: u8 },
}

impl UiCommand {
    pub fn is_supported(&self) -> bool {
        match self {
            Self::Connect { schema_version, .. } | Self::Cancel { schema_version } => {
                *schema_version == 1
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DisconnectedReason {
    Startup,
    Cancelled,
    SessionLost,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionFailureReason {
    Timeout,
    Network,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiStateEnvelope {
    pub schema_version: u8,
    #[serde(flatten)]
    pub state: UiState,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UiState {
    Disconnected {
        address: String,
        reason: DisconnectedReason,
    },
    Connecting {
        address: String,
    },
    Connected {
        address: String,
        local_player: LocalPlayerHudModel,
    },
    ConnectionFailed {
        address: String,
        reason: ConnectionFailureReason,
    },
    Rejected {
        address: String,
    },
    ServerFull {
        address: String,
    },
    VersionMismatch {
        address: String,
    },
}

impl UiState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionAttempt {
    pub id: u64,
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

enum SessionPhase<S> {
    Disconnected {
        address: String,
        reason: DisconnectedReason,
    },
    Connecting(ConnectionAttempt),
    Connected {
        address: String,
        player_slot: u32,
        session: S,
    },
    ConnectionFailed {
        address: String,
        reason: ConnectionFailureReason,
    },
    Rejected {
        address: String,
    },
    ServerFull {
        address: String,
    },
    VersionMismatch {
        address: String,
    },
}

pub struct SessionLifecycle<S> {
    phase: SessionPhase<S>,
    next_attempt_id: u64,
}

impl<S> SessionLifecycle<S> {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            phase: SessionPhase::Disconnected {
                address: address.into(),
                reason: DisconnectedReason::Startup,
            },
            next_attempt_id: 0,
        }
    }

    pub fn connect(&mut self, address: String, now: Instant) -> Option<ConnectionAttempt> {
        if matches!(
            self.phase,
            SessionPhase::Connecting(_) | SessionPhase::Connected { .. }
        ) {
            return None;
        }
        self.next_attempt_id = self.next_attempt_id.checked_add(1)?;
        let attempt = ConnectionAttempt {
            id: self.next_attempt_id,
            address,
            deadline: now + CONNECTION_TIMEOUT,
        };
        self.phase = SessionPhase::Connecting(attempt.clone());
        Some(attempt)
    }

    pub fn cancel(&mut self) -> bool {
        let SessionPhase::Connecting(attempt) = &self.phase else {
            return false;
        };
        self.phase = SessionPhase::Disconnected {
            address: attempt.address.clone(),
            reason: DisconnectedReason::Cancelled,
        };
        true
    }

    pub fn timeout(&mut self, now: Instant) -> bool {
        let SessionPhase::Connecting(attempt) = &self.phase else {
            return false;
        };
        if now < attempt.deadline {
            return false;
        }
        self.phase = SessionPhase::ConnectionFailed {
            address: attempt.address.clone(),
            reason: ConnectionFailureReason::Timeout,
        };
        true
    }

    pub fn complete(&mut self, id: u64, outcome: ConnectionOutcome<S>) -> bool {
        let SessionPhase::Connecting(attempt) = &self.phase else {
            return false;
        };
        if attempt.id != id {
            return false;
        }
        let address = attempt.address.clone();
        self.phase = match outcome {
            ConnectionOutcome::Connected {
                session,
                player_slot,
            } => SessionPhase::Connected {
                address,
                player_slot,
                session,
            },
            ConnectionOutcome::Rejected(HandshakeOutcome::ServerFull) => {
                SessionPhase::ServerFull { address }
            }
            ConnectionOutcome::Rejected(HandshakeOutcome::VersionMismatch) => {
                SessionPhase::VersionMismatch { address }
            }
            ConnectionOutcome::Rejected(HandshakeOutcome::Rejected) => {
                SessionPhase::Rejected { address }
            }
            ConnectionOutcome::Failed => SessionPhase::ConnectionFailed {
                address,
                reason: ConnectionFailureReason::Network,
            },
        };
        true
    }

    pub fn session_lost(&mut self) -> bool {
        let address = match &self.phase {
            SessionPhase::Connected { address, .. } => address.clone(),
            _ => return false,
        };
        self.phase = SessionPhase::Disconnected {
            address,
            reason: DisconnectedReason::SessionLost,
        };
        true
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        match &self.phase {
            SessionPhase::Connecting(attempt) => Some(attempt.deadline),
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

    pub fn ui_state(&self) -> UiStateEnvelope {
        let state = match &self.phase {
            SessionPhase::Disconnected { address, reason } => UiState::Disconnected {
                address: address.clone(),
                reason: *reason,
            },
            SessionPhase::Connecting(attempt) => UiState::Connecting {
                address: attempt.address.clone(),
            },
            SessionPhase::Connected {
                address,
                player_slot,
                ..
            } => UiState::Connected {
                address: address.clone(),
                local_player: LocalPlayerHudModel::for_slot(*player_slot),
            },
            SessionPhase::ConnectionFailed { address, reason } => UiState::ConnectionFailed {
                address: address.clone(),
                reason: *reason,
            },
            SessionPhase::Rejected { address } => UiState::Rejected {
                address: address.clone(),
            },
            SessionPhase::ServerFull { address } => UiState::ServerFull {
                address: address.clone(),
            },
            SessionPhase::VersionMismatch { address } => UiState::VersionMismatch {
                address: address.clone(),
            },
        };
        UiStateEnvelope {
            schema_version: 1,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_and_stale_completion_leave_an_editable_form() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("127.0.0.1:4000");
        let attempt = lifecycle.connect("example:4000".into(), now).unwrap();
        assert!(lifecycle.cancel());
        assert!(!lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Connected {
                session: (),
                player_slot: 1,
            }
        ));
        assert_eq!(
            lifecycle.ui_state().state,
            UiState::Disconnected {
                address: "example:4000".into(),
                reason: DisconnectedReason::Cancelled,
            }
        );
    }

    #[test]
    fn timeout_and_typed_rejections_have_distinct_states() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("localhost:4000");
        let attempt = lifecycle.connect("one:4000".into(), now).unwrap();
        assert!(lifecycle.timeout(attempt.deadline));
        assert!(matches!(
            lifecycle.ui_state().state,
            UiState::ConnectionFailed {
                reason: ConnectionFailureReason::Timeout,
                ..
            }
        ));
        let attempt = lifecycle.connect("two:4000".into(), now).unwrap();
        assert!(lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Rejected(HandshakeOutcome::ServerFull)
        ));
        assert!(matches!(
            lifecycle.ui_state().state,
            UiState::ServerFull { .. }
        ));
        let attempt = lifecycle.connect("three:4000".into(), now).unwrap();
        assert!(lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Rejected(HandshakeOutcome::VersionMismatch)
        ));
        assert!(matches!(
            lifecycle.ui_state().state,
            UiState::VersionMismatch { .. }
        ));
        let attempt = lifecycle.connect("four:4000".into(), now).unwrap();
        assert!(lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Rejected(HandshakeOutcome::Rejected)
        ));
        assert!(matches!(
            lifecycle.ui_state().state,
            UiState::Rejected { .. }
        ));
    }

    #[test]
    fn camel_case_ui_commands_deserialize_and_validate_the_schema() {
        let command: UiCommand =
            serde_json::from_str(r#"{"schemaVersion":1,"kind":"connect","address":"x:4000"}"#)
                .unwrap();
        assert_eq!(
            command,
            UiCommand::Connect {
                schema_version: 1,
                address: "x:4000".into(),
            }
        );
        assert!(command.is_supported());
    }

    #[test]
    fn connected_state_uses_the_camel_case_bridge_contract() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("localhost:4000");
        let attempt = lifecycle.connect("localhost:4000".into(), now).unwrap();
        assert!(lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Connected {
                session: (),
                player_slot: 1,
            }
        ));
        let json = serde_json::to_string(&lifecycle.ui_state()).unwrap();
        assert!(json.contains("\"schemaVersion\":1"));
        assert!(json.contains("\"localPlayer\""));
    }

    #[test]
    fn established_session_loss_returns_to_the_connection_form() {
        let now = Instant::now();
        let mut lifecycle = SessionLifecycle::<()>::new("localhost:4000");
        let attempt = lifecycle.connect("example:4000".into(), now).unwrap();
        assert!(lifecycle.complete(
            attempt.id,
            ConnectionOutcome::Connected {
                session: (),
                player_slot: 1,
            }
        ));
        assert!(lifecycle.session_lost());
        assert_eq!(
            lifecycle.ui_state().state,
            UiState::Disconnected {
                address: "example:4000".into(),
                reason: DisconnectedReason::SessionLost,
            }
        );
    }
}
