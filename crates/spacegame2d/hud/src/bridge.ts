import type { EngineToUi, UiToEngine } from './generated/ui-engine-ipc';

interface HudBridge { subscribe(listener: (raw: unknown) => void): () => void; sendJson(raw: string): void; }
declare global { interface Window { __SPACEGAME_HUD__?: HudBridge; } }

const inboundKinds = new Set(['connectionStateChanged', 'matchSessionStateChanged', 'protocolError', 'heartbeat']);
const outboundKinds = new Set(['uiReady', 'connectRequested', 'connectionCancelled', 'disconnectRequested', 'heartbeatAcknowledged', 'bridgeFaultReported', 'hudLayoutRequested']);
const isRecord = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const isId = (value: unknown) => typeof value === 'string' && /^[A-Za-z0-9_:-]{1,128}$/.test(value);
const isInteger = (value: unknown) => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
const isVersion = (value: Record<string, unknown>) => value.protocolVersion === 3;
const isColor = (value: unknown) => value === 'cyan' || value === 'coral';
const isPresence = (value: unknown) => value === 'waiting' || value === 'present' || value === 'disconnected';
const validPlayer = (value: unknown): boolean => isRecord(value) && isInteger(value.playerSlot) && typeof value.displayName === 'string' && value.displayName.length > 0 && isColor(value.color) && typeof value.colorHex === 'string';
const validClock = (value: unknown): boolean => isRecord(value) && isInteger(value.startedAtTick) && isInteger(value.currentTick) && isInteger(value.ticksPerSecond) && Number(value.ticksPerSecond) > 0 && isInteger(value.elapsedWholeSeconds);
const validMatchState = (value: unknown): boolean => {
  if (!isRecord(value) || !isInteger(value.sequence) || typeof value.stage !== 'string') return false;
  if (value.stage === 'reset') return ['startup', 'newConnectionAttempt', 'userDisconnected', 'sessionLost'].includes(String(value.reason));
  if (!validPlayer(value.localPlayer) || !isInteger(value.presenceRevision) || !isPresence(value.opponentPresence)) return false;
  if (value.stage === 'waiting') return value.opponentPresence === 'waiting';
  return value.stage === 'active' && validPlayer(value.opponentPlayer) && validClock(value.clock) && ['present', 'disconnected'].includes(String(value.opponentPresence));
};
const validState = (value: unknown): boolean => {
  if (!isRecord(value) || typeof value.stage !== 'string' || typeof value.address !== 'string') return false;
  if (value.stage === 'idle') return ['startup', 'cancelled', 'userDisconnected', 'sessionLost'].includes(String(value.reason));
  if (!isId(value.requestId) || typeof value.displayName !== 'string') return false;
  if (['resolvingHost', 'openingSocket', 'handshaking'].includes(value.stage)) return true;
  if (value.stage === 'connected') return isRecord(value.localPlayer) && value.localPlayer.schemaVersion === 1;
  return value.stage === 'failed' && ['timeout', 'network', 'rejected', 'serverFull', 'versionMismatch'].includes(String(value.reason));
};
export function validEngineMessage(value: unknown): value is EngineToUi {
  if (!isRecord(value) || !isVersion(value) || !inboundKinds.has(String(value.kind)) || !isId(value.bridgeId)) return false;
  if (value.kind === 'connectionStateChanged') return validState(value.state);
  if (value.kind === 'matchSessionStateChanged') return validMatchState(value.state);
  if (value.kind === 'protocolError') return typeof value.code === 'string';
  return isInteger(value.sequence);
}
export function validUiMessage(value: unknown): value is UiToEngine {
  if (!isRecord(value) || !isVersion(value) || !outboundKinds.has(String(value.kind)) || !isId(value.bridgeId)) return false;
  if (value.kind === 'connectRequested') return isId(value.requestId) && typeof value.address === 'string' && value.address.trim().length > 0 && typeof value.displayName === 'string' && value.displayName.trim().length > 0;
  if (value.kind === 'connectionCancelled' || value.kind === 'disconnectRequested') return isId(value.requestId);
  if (value.kind === 'hudLayoutRequested') return ['join', 'compact'].includes(String(value.phase)) ? value.transitionDurationMs === undefined : value.phase === 'docking' && isInteger(value.transitionDurationMs) && Number(value.transitionDurationMs) > 0;
  return value.kind === 'uiReady' || isInteger(value.sequence) || typeof value.code === 'string';
}
export function subscribe(listener: (message: EngineToUi) => void): () => void {
  const bridge = window.__SPACEGAME_HUD__; if (!bridge) return () => {};
  return bridge.subscribe((raw) => { if (typeof raw !== 'string') return; try { const message: unknown = JSON.parse(raw); if (validEngineMessage(message)) listener(message); } catch { /* raw transport failures are ignored safely */ } });
}
export function send(message: UiToEngine): void { if (validUiMessage(message)) window.__SPACEGAME_HUD__?.sendJson(JSON.stringify(message)); }
