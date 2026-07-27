import type { UiCommand, UiState } from './model';
interface HudBridge { getState(): unknown; subscribe(listener: (state: unknown) => void): () => void; send(command: UiCommand): void; }
declare global { interface Window { __SPACEGAME_HUD__?: HudBridge; } }

export function validState(value: unknown): value is UiState {
  if (!value || typeof value !== 'object') return false;
  const state = value as Record<string, unknown>;
  if (state.schemaVersion !== 1 || typeof state.kind !== 'string' || typeof state.address !== 'string') return false;
  return ['disconnected', 'connecting', 'connected', 'connectionFailed', 'rejected', 'serverFull', 'versionMismatch'].includes(state.kind);
}
export function readState(): UiState {
  const state = window.__SPACEGAME_HUD__?.getState();
  if (!validState(state)) throw new Error('Invalid HUD state');
  return state;
}
export function subscribeState(listener: (state: UiState) => void): () => void {
  const bridge = window.__SPACEGAME_HUD__;
  if (!bridge || typeof bridge.subscribe !== 'function') return () => {};
  return bridge.subscribe((state) => { if (validState(state)) listener(state); });
}
export function send(command: UiCommand): void { window.__SPACEGAME_HUD__?.send(command); }
