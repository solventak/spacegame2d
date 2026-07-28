// Generated from crates/ui-protocol/schema/ui-engine-ipc.v1.schema.json. Do not edit by hand.
export const protocolVersion = 3 as const;
export type ProtocolErrorCode = 'malformedJson' | 'unsupportedVersion' | 'wrongDirection' | 'unknownMessageType' | 'missingRequiredField' | 'unknownField' | 'invalidFieldValue';
export type RequestId = string;
export type BridgeId = string;
export type PlayerColor = 'cyan' | 'coral';
export type OpponentPresence = 'waiting' | 'present' | 'disconnected';
export type LocalPlayer = { schemaVersion: 1; playerSlot: number; color: PlayerColor; colorHex: string };
export type MatchParticipant = { playerSlot: number; displayName: string; color: PlayerColor; colorHex: string };
export type MatchClock = { startedAtTick: number; currentTick: number; ticksPerSecond: number; elapsedWholeSeconds: number };
export type MatchSessionState =
  | { stage: 'reset'; sequence: number; reason: 'startup' | 'newConnectionAttempt' | 'userDisconnected' | 'sessionLost' }
  | { stage: 'waiting'; sequence: number; localPlayer: MatchParticipant; opponentPresence: 'waiting'; presenceRevision: number }
  | { stage: 'active'; sequence: number; localPlayer: MatchParticipant; opponentPlayer: MatchParticipant; opponentPresence: 'present' | 'disconnected'; presenceRevision: number; clock: MatchClock };
export type ConnectionState =
  | { stage: 'idle'; address: string; reason: 'startup' | 'cancelled' | 'userDisconnected' | 'sessionLost' }
  | { stage: 'resolvingHost' | 'openingSocket' | 'handshaking'; requestId: RequestId; address: string; displayName: string }
  | { stage: 'connected'; requestId: RequestId; address: string; displayName: string; localPlayer: LocalPlayer }
  | { stage: 'failed'; requestId: RequestId; address: string; displayName: string; reason: 'timeout' | 'network' | 'rejected' | 'serverFull' | 'versionMismatch' };
export type UiToEngine =
  | { kind: 'uiReady'; protocolVersion: 3; bridgeId: BridgeId }
  | { kind: 'connectRequested'; protocolVersion: 3; bridgeId: BridgeId; requestId: RequestId; address: string; displayName: string }
  | { kind: 'connectionCancelled'; protocolVersion: 3; bridgeId: BridgeId; requestId: RequestId }
  | { kind: 'disconnectRequested'; protocolVersion: 3; bridgeId: BridgeId; requestId: RequestId }
  | { kind: 'heartbeatAcknowledged'; protocolVersion: 3; bridgeId: BridgeId; sequence: number }
  | { kind: 'bridgeFaultReported'; protocolVersion: 3; bridgeId: BridgeId; code: ProtocolErrorCode };
export type EngineToUi =
  | { kind: 'connectionStateChanged'; protocolVersion: 3; bridgeId: BridgeId; state: ConnectionState }
  | { kind: 'matchSessionStateChanged'; protocolVersion: 3; bridgeId: BridgeId; state: MatchSessionState }
  | { kind: 'protocolError'; protocolVersion: 3; bridgeId: BridgeId; code: ProtocolErrorCode }
  | { kind: 'heartbeat'; protocolVersion: 3; bridgeId: BridgeId; sequence: number };
