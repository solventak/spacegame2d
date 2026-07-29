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
  const rendered = render(App);
  const currentBridge = JSON.parse(sendJson.mock.calls[0]?.[0] ?? '{}').bridgeId;
  const publish = async (kind: string, state: Record<string, unknown>) => {
    listener?.(JSON.stringify({ kind, protocolVersion: 3, bridgeId: currentBridge, state })); await tick();
  };
  return { sendJson, publish, container: rendered.container };
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

  it('renders the designed reveal before docking into the full status bar', async () => {
    vi.useFakeTimers(); const { sendJson, publish, container } = renderHud();
    await publish('connectionStateChanged', { stage: 'connected', requestId: 'request-1', address: 'server:4000', displayName: 'Rook', localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } });
    await publish('matchSessionStateChanged', { stage: 'active', sequence: 2, localPlayer: player('Rook', 'cyan'), opponentPlayer: player('Vale', 'coral'), opponentPresence: 'present', presenceRevision: 1, clock: { startedAtTick: 60, currentTick: 120, ticksPerSecond: 60, elapsedWholeSeconds: 1 } });
    expect(screen.getByText('Match accepted')).toBeTruthy();
    expect(screen.getByText('Cyan · Rook')).toBeTruthy();
    expect(screen.getByText('Vale')).toBeTruthy();
    expect(screen.getByText('PRESENT')).toBeTruthy();
    expect(container.querySelector('.match-reveal-card')).toBeTruthy();
    expect(container.querySelector('.friendly-mark')).toBeTruthy();
    expect(container.querySelector('.participant.friendly')).toBeTruthy();
    expect(container.querySelector('.participant.enemy')).toBeTruthy();
    expect(container.querySelector('.reveal-divider')).toBeTruthy();
    expect(container.querySelector('.status-bar')).toBeNull();
    expect(screen.queryByRole('button', { name: 'DISCONNECT' })).toBeNull();
    expect(screen.queryByText('T+00:01')).toBeNull();
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}')).toMatchObject({ kind: 'hudLayoutRequested', phase: 'join' });

    await publish('matchSessionStateChanged', { stage: 'active', sequence: 3, localPlayer: player('Rook', 'cyan'), opponentPlayer: player('Vale', 'coral'), opponentPresence: 'present', presenceRevision: 1, clock: { startedAtTick: 60, currentTick: 180, ticksPerSecond: 60, elapsedWholeSeconds: 2 } });
    await vi.advanceTimersByTimeAsync(3999); await tick();
    expect(container.querySelector('.match-hud.reveal')).toBeTruthy();
    expect(container.querySelector('.status-bar')).toBeNull();

    await vi.advanceTimersByTimeAsync(1); await tick();
    expect(container.querySelector('.match-hud.docking')).toBeTruthy();
    expect(container.querySelector('.match-reveal-card')).toBeTruthy();
    expect(container.querySelector('.status-bar')).toBeTruthy();
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}')).toMatchObject({ kind: 'hudLayoutRequested', phase: 'docking', transitionDurationMs: 600 });

    await vi.advanceTimersByTimeAsync(600); await tick();
    expect(container.querySelector('.match-hud.compact')).toBeTruthy();
    expect(container.querySelector('.match-reveal-card')).toBeNull();
    expect(screen.getByText('T+00:02')).toBeTruthy();
    expect(screen.getByText('CYAN · Rook')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'DISCONNECT' })).toBeTruthy();
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}')).toMatchObject({ kind: 'hudLayoutRequested', phase: 'compact' });

    await publish('matchSessionStateChanged', { stage: 'active', sequence: 4, localPlayer: player('Rook', 'cyan'), opponentPlayer: player('Vale', 'coral'), opponentPresence: 'disconnected', presenceRevision: 2, clock: { startedAtTick: 60, currentTick: 240, ticksPerSecond: 60, elapsedWholeSeconds: 3 } });
    expect(screen.getByText('DISCONNECTED')).toBeTruthy();
    expect(screen.getByText('Vale')).toBeTruthy();
    expect(screen.getByText('T+00:03')).toBeTruthy();
    expect(container.querySelector('.match-hud.compact')).toBeTruthy();

    await publish('matchSessionStateChanged', { stage: 'active', sequence: 5, localPlayer: player('Rook', 'cyan'), opponentPlayer: player('Ash', 'coral'), opponentPresence: 'present', presenceRevision: 3, clock: { startedAtTick: 60, currentTick: 300, ticksPerSecond: 60, elapsedWholeSeconds: 4 } });
    expect(screen.getByText('Ash')).toBeTruthy();
    expect(screen.queryByText('Vale')).toBeNull();
    expect(container.querySelector('.match-hud.compact')).toBeTruthy();

    await publish('matchSessionStateChanged', { stage: 'reset', sequence: 6, reason: 'userDisconnected' });
    expect(screen.queryByText('Ash')).toBeNull();
    expect(screen.getByText('CONNECT TO SERVER')).toBeTruthy();
  });

  it('renders the accepted coral player as coral throughout the active-match HUD', async () => {
    vi.useFakeTimers(); const { publish, container } = renderHud();
    await publish('connectionStateChanged', { stage: 'connected', requestId: 'request-2', address: 'server:4000', displayName: 'Ember', localPlayer: { schemaVersion: 1, playerSlot: 2, color: 'coral', colorHex: '#FF6A47' } });
    await publish('matchSessionStateChanged', { stage: 'active', sequence: 1, localPlayer: player('Ember', 'coral'), opponentPlayer: player('Rook', 'cyan'), opponentPresence: 'present', presenceRevision: 1, clock: { startedAtTick: 60, currentTick: 120, ticksPerSecond: 60, elapsedWholeSeconds: 1 } });

    expect(screen.getByText('Coral · Ember')).toBeTruthy();
    expect(container.querySelector('.match-reveal-card .participant.enemy')).toBeTruthy();
    expect(container.querySelector('.match-reveal-card .friendly-mark .enemy')).toBeTruthy();

    await vi.advanceTimersByTimeAsync(4000); await tick();
    expect(screen.getByText('CORAL · Ember')).toBeTruthy();
    expect(container.querySelector('.bar-local svg.enemy')).toBeTruthy();
  });

  it('rejects partial match state and accepts versioned directional messages', () => {
    expect(validEngineMessage({ kind: 'heartbeat', protocolVersion: 3, bridgeId, sequence: 1 })).toBe(true);
    expect(validEngineMessage({ kind: 'matchSessionStateChanged', protocolVersion: 3, bridgeId, state: { stage: 'active', sequence: 1 } })).toBe(false);
    expect(validUiMessage({ kind: 'disconnectRequested', protocolVersion: 3, bridgeId, requestId: 'request-1' })).toBe(true);
  });
});
