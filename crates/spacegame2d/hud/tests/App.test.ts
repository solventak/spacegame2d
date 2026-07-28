import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import App from '../src/App.svelte';
import { validEngineMessage, validUiMessage } from '../src/bridge';

const bridgeId = 'bridge-test';
const ready = () => ({ kind: 'uiReady' as const, protocolVersion: 2 as const, bridgeId });

function renderHud() {
  const sendJson = vi.fn();
  let listener: ((raw: unknown) => void) | undefined;
  window.__SPACEGAME_HUD__ = {
    sendJson,
    subscribe: (next) => {
      listener = next;
      return () => {};
    },
  };
  render(App);
  const readyMessage = JSON.parse(sendJson.mock.calls[0]?.[0] ?? '{}');
  const publish = async (state: Record<string, unknown>) => {
    listener?.(JSON.stringify({
      kind: 'connectionStateChanged',
      protocolVersion: 2,
      bridgeId: readyMessage.bridgeId,
      state,
    }));
    await tick();
  };
  return { sendJson, publish };
}

describe('HUD IPC', () => {
  afterEach(() => {
    cleanup();
    window.__SPACEGAME_HUD__ = undefined;
  });

  it('sends uiReady after subscribing and renders the connected compact HUD', async () => {
    const { sendJson, publish } = renderHud();
    expect(JSON.parse(sendJson.mock.calls[0]?.[0] ?? '{}').kind).toBe('uiReady');
    await publish({
      stage: 'connected',
      requestId: 'request-1',
      address: 'server.example:4000',
      displayName: 'Rook',
      localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' },
    });
    expect(screen.getByText('LOCAL COMMAND')).toBeTruthy();
  });

  it('uses a hostname-safe address field and sends a request-scoped connect command', async () => {
    const { sendJson } = renderHud();
    const input = screen.getByLabelText('Server address');
    const name = screen.getByLabelText('Display name');
    const connect = screen.getByRole('button', { name: 'CONNECT' });
    expect(connect).toHaveProperty('disabled', true);
    await fireEvent.input(input, { target: { value: 'play.example:4000' } });
    await fireEvent.input(name, { target: { value: '  Café  ' } });
    expect(screen.getByText('READY')).toBeTruthy();
    await fireEvent.click(connect);
    const message = JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}');
    expect(message.kind).toBe('connectRequested');
    expect(message.address).toBe('play.example:4000');
    expect(message.displayName).toBe('Café');
    expect(message.requestId).toMatch(/^request-/);
  });

  it('maps each in-progress state to an instrument status and enables abort', async () => {
    const { publish } = renderHud();
    for (const [stage, label] of [
      ['resolvingHost', 'RESOLVING HOST'],
      ['openingSocket', 'OPENING SOCKET'],
      ['handshaking', 'HANDSHAKING'],
    ]) {
      await publish({ stage, requestId: 'request-1', address: 'server.example:4000', displayName: 'Rook' });
      expect(screen.getByText(label)).toBeTruthy();
      expect(screen.getByRole('button', { name: 'ABORT' })).toBeTruthy();
    }
  });

  it('blocks invalid callsigns and locks the accepted value during an attempt', async () => {
    const { sendJson, publish } = renderHud();
    const name = screen.getByLabelText('Display name');
    const address = screen.getByLabelText('Server address');
    await fireEvent.input(name, { target: { value: 'x'.repeat(25) } });
    await fireEvent.input(address, { target: { value: 'server.example:4000' } });
    expect(screen.getByText('MAXIMUM 24 CHARACTERS')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'CONNECT' })).toHaveProperty('disabled', true);
    await fireEvent.input(name, { target: { value: '🚀' } });
    expect(screen.getByText('1/24')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'CONNECT' }));
    expect(JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}').displayName).toBe('🚀');
    await publish({ stage: 'handshaking', requestId: JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}').requestId, address: 'server.example:4000', displayName: '🚀' });
    expect(name).toHaveProperty('disabled', true);
  });

  it.each([
    ['timeout', 'NO RESPONSE'],
    ['network', 'NETWORK UNAVAILABLE'],
    ['rejected', 'LINK REJECTED'],
    ['serverFull', 'SERVER FULL'],
    ['versionMismatch', 'VERSION MISMATCH'],
  ])('renders the %s failure without inventing telemetry', async (reason, label) => {
    const { publish } = renderHud();
    await publish({ stage: 'failed', requestId: 'request-1', address: 'server.example:4000', displayName: 'Rook', reason });
    expect(screen.getByText(label)).toBeTruthy();
    expect(screen.getByText('RTT N/A')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'CONNECT' })).toBeTruthy();
  });

  it('validates strict directional messages', () => {
    expect(validEngineMessage({ kind: 'heartbeat', protocolVersion: 2, bridgeId, sequence: 1 })).toBe(true);
    expect(validEngineMessage({ kind: 'uiReady', protocolVersion: 2, bridgeId })).toBe(false);
    expect(validUiMessage(ready())).toBe(true);
    expect(validUiMessage({ kind: 'connectionStateChanged', protocolVersion: 2, bridgeId, state: {} })).toBe(false);
  });
});
