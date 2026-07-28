import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import App from '../src/App.svelte';
import { validEngineMessage, validUiMessage } from '../src/bridge';

const bridgeId = 'bridge-test';
const player = (displayName: string, color: 'cyan' | 'coral') => ({ playerSlot: color === 'cyan' ? 1 : 2, displayName, color, colorHex: color === 'cyan' ? '#22CFE8' : '#FF6A47' });

function renderHud() {
  const sendJson = vi.fn(); let listener: ((raw: unknown) => void) | undefined;
  window.__SPACEGAME_HUD__ = { sendJson, subscribe: (next) => { listener = next; return () => {}; } };
  render(App);
  const currentBridge = JSON.parse(sendJson.mock.calls[0]?.[0] ?? '{}').bridgeId;
  const publish = async (kind: string, state: Record<string, unknown>) => {
    listener?.(JSON.stringify({ kind, protocolVersion: 3, bridgeId: currentBridge, state })); await tick();
  };
  return { sendJson, publish };
}

describe('HUD IPC', () => {
  afterEach(() => { cleanup(); window.__SPACEGAME_HUD__ = undefined; vi.useRealTimers(); });

  it('uses a hostname-safe address field and sends a request-scoped connect command', async () => {
    const { sendJson } = renderHud();
    await fireEvent.input(screen.getByLabelText('Server address'), { target: { value: 'play.example:4000' } });
    await fireEvent.input(screen.getByLabelText('Display name'), { target: { value: '  Café  ' } });
    await fireEvent.click(screen.getByRole('button', { name: 'CONNECT' }));
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}')).toMatchObject({ kind: 'connectRequested', protocolVersion: 3, address: 'play.example:4000', displayName: 'Café' });
  });

  it('keeps a connected solo player on the main menu and exposes Disconnect', async () => {
    const { sendJson, publish } = renderHud();
    await publish('connectionStateChanged', { stage: 'connected', requestId: 'request-1', address: 'server:4000', displayName: 'Rook', localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } });
    const waitingState = { stage: 'waiting', sequence: 1, localPlayer: player('Rook', 'cyan'), opponentPresence: 'waiting', presenceRevision: 0 };
    expect(validEngineMessage({ kind: 'matchSessionStateChanged', protocolVersion: 3, bridgeId, state: waitingState })).toBe(true);
    await publish('matchSessionStateChanged', waitingState);
    expect(screen.getByText('Waiting for opponent…')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'DISCONNECT' }));
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}')).toMatchObject({ kind: 'disconnectRequested', requestId: 'request-1' });
  });

  it('shows match data then docks into the persistent status bar', async () => {
    vi.useFakeTimers(); const { publish } = renderHud();
    await publish('connectionStateChanged', { stage: 'connected', requestId: 'request-1', address: 'server:4000', displayName: 'Rook', localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } });
    await publish('matchSessionStateChanged', { stage: 'active', sequence: 2, localPlayer: player('Rook', 'cyan'), opponentPlayer: player('Vale', 'coral'), opponentPresence: 'present', presenceRevision: 1, clock: { startedAtTick: 60, currentTick: 120, ticksPerSecond: 60, elapsedWholeSeconds: 1 } });
    expect(screen.getByText('Match accepted')).toBeTruthy(); expect(screen.getByText('Vale')).toBeTruthy();
    await vi.advanceTimersByTimeAsync(1300); await tick();
    expect(screen.getByText('PRESENT')).toBeTruthy(); expect(screen.getByText('00:01')).toBeTruthy();
  });

  it('rejects partial match state and accepts versioned directional messages', () => {
    expect(validEngineMessage({ kind: 'heartbeat', protocolVersion: 3, bridgeId, sequence: 1 })).toBe(true);
    expect(validEngineMessage({ kind: 'matchSessionStateChanged', protocolVersion: 3, bridgeId, state: { stage: 'active', sequence: 1 } })).toBe(false);
    expect(validUiMessage({ kind: 'disconnectRequested', protocolVersion: 3, bridgeId, requestId: 'request-1' })).toBe(true);
  });
});
