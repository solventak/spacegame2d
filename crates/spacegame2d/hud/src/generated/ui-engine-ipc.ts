// Generated from crates/ui-protocol/schema/ui-engine-ipc.v1.schema.json. Do not edit by hand.
export const protocolVersion = 1 as const;
export type ProtocolErrorCode = 'malformedJson' | 'unsupportedVersion' | 'wrongDirection' | 'unknownMessageType' | 'missingRequiredField' | 'unknownField' | 'invalidFieldValue';
export type RequestId = string;
export type BridgeId = string;
export type LocalPlayer = { schemaVersion: 1; playerSlot: number; color: 'cyan' | 'coral'; colorHex: string };
export type ConnectionState =
  | { stage: 'idle'; address: string; reason: 'startup' | 'cancelled' | 'sessionLost' }
  | { stage: 'resolvingHost' | 'openingSocket' | 'handshaking'; requestId: RequestId; address: string }
  | { stage: 'connected'; requestId: RequestId; address: string; localPlayer: LocalPlayer }
  | { stage: 'failed'; requestId: RequestId; address: string; reason: 'timeout' | 'network' | 'rejected' | 'serverFull' | 'versionMismatch' };
export type UiToEngine =
  | { kind: 'uiReady'; protocolVersion: 1; bridgeId: BridgeId }
  | { kind: 'connectRequested'; protocolVersion: 1; bridgeId: BridgeId; requestId: RequestId; address: string }
  | { kind: 'connectionCancelled'; protocolVersion: 1; bridgeId: BridgeId; requestId: RequestId }
  | { kind: 'heartbeatAcknowledged'; protocolVersion: 1; bridgeId: BridgeId; sequence: number }
  | { kind: 'bridgeFaultReported'; protocolVersion: 1; bridgeId: BridgeId; code: ProtocolErrorCode };
export type EngineToUi =
  | { kind: 'connectionStateChanged'; protocolVersion: 1; bridgeId: BridgeId; state: ConnectionState }
  | { kind: 'protocolError'; protocolVersion: 1; bridgeId: BridgeId; code: ProtocolErrorCode }
  | { kind: 'heartbeat'; protocolVersion: 1; bridgeId: BridgeId; sequence: number };
