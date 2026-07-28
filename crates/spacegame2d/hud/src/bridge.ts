import type { EngineToUi, UiToEngine } from './generated/ui-engine-ipc';

interface HudBridge { subscribe(listener: (raw: unknown) => void): () => void; sendJson(raw: string): void; }
declare global { interface Window { __SPACEGAME_HUD__?: HudBridge; } }

const inboundKinds = new Set(['connectionStateChanged', 'protocolError', 'heartbeat']);
const outboundKinds = new Set(['uiReady', 'connectRequested', 'connectionCancelled', 'heartbeatAcknowledged', 'bridgeFaultReported']);
const isRecord = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const isId = (value: unknown) => typeof value === 'string' && /^[A-Za-z0-9_:-]{1,128}$/.test(value);
const isVersion = (value: Record<string, unknown>) => value.protocolVersion === 2;
const validState = (value: unknown): boolean => {
  if (!isRecord(value) || typeof value.stage !== 'string' || typeof value.address !== 'string') return false;
  if (value.stage === 'idle') return ['startup', 'cancelled', 'sessionLost'].includes(String(value.reason));
  if (!isId(value.requestId)) return false;
  if (typeof value.displayName !== 'string') return false;
  if (['resolvingHost', 'openingSocket', 'handshaking'].includes(value.stage)) return true;
  if (value.stage === 'connected') return isRecord(value.localPlayer) && value.localPlayer.schemaVersion === 1;
  return value.stage === 'failed' && ['timeout', 'network', 'rejected', 'serverFull', 'versionMismatch'].includes(String(value.reason));
};
export function validEngineMessage(value: unknown): value is EngineToUi {
  if (!isRecord(value) || !isVersion(value) || !inboundKinds.has(String(value.kind)) || !isId(value.bridgeId)) return false;
  if (value.kind === 'connectionStateChanged') return validState(value.state);
  if (value.kind === 'protocolError') return typeof value.code === 'string';
  return typeof value.sequence === 'number';
}
export function validUiMessage(value: unknown): value is UiToEngine {
  if (!isRecord(value) || !isVersion(value) || !outboundKinds.has(String(value.kind)) || !isId(value.bridgeId)) return false;
  if (value.kind === 'connectRequested') return isId(value.requestId) && typeof value.address === 'string' && value.address.trim().length > 0 && typeof value.displayName === 'string' && value.displayName.trim().length > 0;
  if (value.kind === 'connectionCancelled') return isId(value.requestId);
  return value.kind === 'uiReady' || typeof value.sequence === 'number' || typeof value.code === 'string';
}
export function subscribe(listener: (message: EngineToUi) => void): () => void {
  const bridge = window.__SPACEGAME_HUD__; if (!bridge) return () => {};
  return bridge.subscribe((raw) => { if (typeof raw !== 'string') return; try { const message: unknown = JSON.parse(raw); if (validEngineMessage(message)) listener(message); } catch { /* raw transport failures are ignored safely */ } });
}
export function send(message: UiToEngine): void { if (validUiMessage(message)) window.__SPACEGAME_HUD__?.sendJson(JSON.stringify(message)); }
