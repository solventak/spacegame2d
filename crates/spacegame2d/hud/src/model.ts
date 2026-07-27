export type PlayerColor = 'cyan' | 'coral';
export interface LocalPlayerHudModel { schemaVersion: 1; playerSlot: number; color: PlayerColor; colorHex: string; }
export type UiState =
  | { schemaVersion: 1; kind: 'disconnected'; address: string; reason: 'startup' | 'cancelled' | 'sessionLost' }
  | { schemaVersion: 1; kind: 'connecting'; address: string }
  | { schemaVersion: 1; kind: 'connected'; address: string; localPlayer: LocalPlayerHudModel }
  | { schemaVersion: 1; kind: 'connectionFailed'; address: string; reason: 'timeout' | 'network' }
  | { schemaVersion: 1; kind: 'rejected'; address: string }
  | { schemaVersion: 1; kind: 'serverFull'; address: string }
  | { schemaVersion: 1; kind: 'versionMismatch'; address: string };
export type UiCommand =
  | { schemaVersion: 1; kind: 'connect'; address: string }
  | { schemaVersion: 1; kind: 'cancel' };
