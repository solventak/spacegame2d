//! Versioned, directional IPC contract for the embedded UI and native engine.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const UI_ENGINE_PROTOCOL_VERSION: u16 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct BridgeId(String);

impl BridgeId {
    pub fn new(value: String) -> Result<Self, ProtocolValidationError> {
        validate_id("bridgeId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    pub fn new(value: String) -> Result<Self, ProtocolValidationError> {
        validate_id("requestId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_id(name: &'static str, value: &str) -> Result<(), ProtocolValidationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
    {
        return Err(ProtocolValidationError::InvalidId(name));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProtocolErrorCode {
    MalformedJson,
    UnsupportedVersion,
    WrongDirection,
    UnknownMessageType,
    MissingRequiredField,
    UnknownField,
    InvalidFieldValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DisconnectedReason {
    Startup,
    Cancelled,
    UserDisconnected,
    SessionLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionFailureReason {
    Timeout,
    Network,
    Rejected,
    ServerFull,
    VersionMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PlayerColor {
    Cyan,
    Coral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OpponentPresence {
    Waiting,
    Present,
    Disconnected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalPlayerHudModel {
    pub schema_version: u8,
    pub player_slot: u32,
    pub color: PlayerColor,
    pub color_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatchParticipantHudModel {
    pub player_slot: u32,
    pub display_name: String,
    pub color: PlayerColor,
    pub color_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatchClockHudModel {
    pub started_at_tick: u64,
    pub current_tick: u64,
    pub ticks_per_second: u32,
    pub elapsed_whole_seconds: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MatchSessionResetReason {
    Startup,
    NewConnectionAttempt,
    UserDisconnected,
    SessionLost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "stage",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MatchSessionState {
    Reset {
        sequence: u64,
        reason: MatchSessionResetReason,
    },
    Waiting {
        sequence: u64,
        local_player: MatchParticipantHudModel,
        opponent_presence: OpponentPresence,
        presence_revision: u64,
    },
    Active {
        sequence: u64,
        local_player: MatchParticipantHudModel,
        opponent_player: MatchParticipantHudModel,
        opponent_presence: OpponentPresence,
        presence_revision: u64,
        clock: MatchClockHudModel,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "stage",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ConnectionStateSnapshot {
    Idle {
        address: String,
        reason: DisconnectedReason,
    },
    ResolvingHost {
        request_id: RequestId,
        address: String,
        display_name: String,
    },
    OpeningSocket {
        request_id: RequestId,
        address: String,
        display_name: String,
    },
    Handshaking {
        request_id: RequestId,
        address: String,
        display_name: String,
    },
    Connected {
        request_id: RequestId,
        address: String,
        display_name: String,
        local_player: LocalPlayerHudModel,
    },
    Failed {
        request_id: RequestId,
        address: String,
        display_name: String,
        reason: ConnectionFailureReason,
    },
}

impl ConnectionStateSnapshot {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
    pub fn request_id(&self) -> Option<&RequestId> {
        match self {
            Self::Idle { .. } => None,
            Self::ResolvingHost { request_id, .. }
            | Self::OpeningSocket { request_id, .. }
            | Self::Handshaking { request_id, .. }
            | Self::Connected { request_id, .. }
            | Self::Failed { request_id, .. } => Some(request_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum UiToEngineMessage {
    UiReady {
        protocol_version: u16,
        bridge_id: BridgeId,
    },
    ConnectRequested {
        protocol_version: u16,
        bridge_id: BridgeId,
        request_id: RequestId,
        address: String,
        display_name: String,
    },
    ConnectionCancelled {
        protocol_version: u16,
        bridge_id: BridgeId,
        request_id: RequestId,
    },
    DisconnectRequested {
        protocol_version: u16,
        bridge_id: BridgeId,
        request_id: RequestId,
    },
    HeartbeatAcknowledged {
        protocol_version: u16,
        bridge_id: BridgeId,
        sequence: u64,
    },
    BridgeFaultReported {
        protocol_version: u16,
        bridge_id: BridgeId,
        code: ProtocolErrorCode,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EngineToUiMessage {
    ConnectionStateChanged {
        protocol_version: u16,
        bridge_id: BridgeId,
        state: ConnectionStateSnapshot,
    },
    MatchSessionStateChanged {
        protocol_version: u16,
        bridge_id: BridgeId,
        state: MatchSessionState,
    },
    ProtocolError {
        protocol_version: u16,
        bridge_id: BridgeId,
        code: ProtocolErrorCode,
    },
    Heartbeat {
        protocol_version: u16,
        bridge_id: BridgeId,
        sequence: u64,
    },
}

pub trait VersionedMessage {
    fn protocol_version(&self) -> u16;
}
impl VersionedMessage for UiToEngineMessage {
    fn protocol_version(&self) -> u16 {
        match self {
            Self::UiReady {
                protocol_version, ..
            }
            | Self::ConnectRequested {
                protocol_version, ..
            }
            | Self::ConnectionCancelled {
                protocol_version, ..
            }
            | Self::DisconnectRequested {
                protocol_version, ..
            }
            | Self::HeartbeatAcknowledged {
                protocol_version, ..
            }
            | Self::BridgeFaultReported {
                protocol_version, ..
            } => *protocol_version,
        }
    }
}
impl VersionedMessage for EngineToUiMessage {
    fn protocol_version(&self) -> u16 {
        match self {
            Self::ConnectionStateChanged {
                protocol_version, ..
            }
            | Self::MatchSessionStateChanged {
                protocol_version, ..
            }
            | Self::ProtocolError {
                protocol_version, ..
            }
            | Self::Heartbeat {
                protocol_version, ..
            } => *protocol_version,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolValidationError {
    #[error("invalid {0}")]
    InvalidId(&'static str),
    #[error("unsupported protocol version {0}")]
    UnsupportedVersion(u16),
}
#[derive(Debug, Error)]
pub enum ProtocolDecodeError {
    #[error("{code:?}: {detail}")]
    Invalid {
        code: ProtocolErrorCode,
        detail: String,
    },
    #[error(transparent)]
    Validation(#[from] ProtocolValidationError),
}

impl UiToEngineMessage {
    pub fn decode(raw: &str) -> Result<Self, ProtocolDecodeError> {
        let message: Self =
            serde_json::from_str(raw).map_err(|error| ProtocolDecodeError::Invalid {
                code: classify_decode_error(raw, &error.to_string()),
                detail: error.to_string(),
            })?;
        message.validate()?;
        Ok(message)
    }
    pub fn validate(&self) -> Result<(), ProtocolValidationError> {
        if self.protocol_version() != UI_ENGINE_PROTOCOL_VERSION {
            return Err(ProtocolValidationError::UnsupportedVersion(
                self.protocol_version(),
            ));
        }
        match self {
            Self::UiReady { bridge_id, .. }
            | Self::ConnectRequested { bridge_id, .. }
            | Self::ConnectionCancelled { bridge_id, .. }
            | Self::DisconnectRequested { bridge_id, .. }
            | Self::HeartbeatAcknowledged { bridge_id, .. }
            | Self::BridgeFaultReported { bridge_id, .. } => {
                validate_id("bridgeId", bridge_id.as_str())?
            }
        }
        match self {
            Self::ConnectRequested {
                request_id,
                address,
                display_name,
                ..
            } => {
                validate_id("requestId", request_id.as_str())?;
                if address.trim().is_empty() {
                    return Err(ProtocolValidationError::InvalidId("address"));
                }
                if display_name.trim().is_empty() {
                    return Err(ProtocolValidationError::InvalidId("displayName"));
                }
            }
            Self::ConnectionCancelled { request_id, .. }
            | Self::DisconnectRequested { request_id, .. } => {
                validate_id("requestId", request_id.as_str())?
            }
            _ => {}
        }
        Ok(())
    }
}
impl EngineToUiMessage {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

fn classify_decode_error(raw: &str, detail: &str) -> ProtocolErrorCode {
    if serde_json::from_str::<serde_json::Value>(raw).is_err() {
        return ProtocolErrorCode::MalformedJson;
    }
    if [
        "connectionStateChanged",
        "matchSessionStateChanged",
        "protocolError",
        "heartbeat",
    ]
    .iter()
    .any(|kind| raw.contains(&format!("\"kind\":\"{kind}\"")))
    {
        return ProtocolErrorCode::WrongDirection;
    }
    if detail.contains("unknown field") {
        ProtocolErrorCode::UnknownField
    } else if detail.contains("missing field") {
        ProtocolErrorCode::MissingRequiredField
    } else if detail.contains("unknown variant") {
        ProtocolErrorCode::UnknownMessageType
    } else {
        ProtocolErrorCode::InvalidFieldValue
    }
}

pub fn schema_bundle() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "spacegame2d UI-engine IPC",
        "protocolVersion": UI_ENGINE_PROTOCOL_VERSION,
        "uiToEngine": schema_for!(UiToEngineMessage),
        "engineToUi": schema_for!(EngineToUiMessage),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_fields_and_wrong_direction() {
        let raw = r#"{"kind":"uiReady","protocolVersion":1,"bridgeId":"bridge-1","extra":true}"#;
        assert!(matches!(
            UiToEngineMessage::decode(raw),
            Err(ProtocolDecodeError::Invalid {
                code: ProtocolErrorCode::UnknownField,
                ..
            })
        ));
        let raw = r#"{"kind":"heartbeat","protocolVersion":1,"bridgeId":"bridge-1","sequence":1}"#;
        assert!(matches!(
            UiToEngineMessage::decode(raw),
            Err(ProtocolDecodeError::Invalid {
                code: ProtocolErrorCode::WrongDirection,
                ..
            })
        ));
    }
    #[test]
    fn rejects_unsupported_versions() {
        let raw = r#"{"kind":"uiReady","protocolVersion":4,"bridgeId":"bridge-1"}"#;
        assert!(matches!(
            UiToEngineMessage::decode(raw),
            Err(ProtocolDecodeError::Validation(
                ProtocolValidationError::UnsupportedVersion(4)
            ))
        ));
    }
    #[test]
    fn connect_requires_a_display_name() {
        let raw = r#"{"kind":"connectRequested","protocolVersion":3,"bridgeId":"bridge-1","requestId":"request-1","address":"server:4000"}"#;
        assert!(matches!(
            UiToEngineMessage::decode(raw),
            Err(ProtocolDecodeError::Invalid {
                code: ProtocolErrorCode::MissingRequiredField,
                ..
            })
        ));
        let raw = r#"{"kind":"connectRequested","protocolVersion":3,"bridgeId":"bridge-1","requestId":"request-1","address":"server:4000","displayName":"   "}"#;
        assert!(matches!(
            UiToEngineMessage::decode(raw),
            Err(ProtocolDecodeError::Validation(
                ProtocolValidationError::InvalidId("displayName")
            ))
        ));
    }
}
