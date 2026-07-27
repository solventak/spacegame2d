import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import App from '../src/App.svelte';
import { validEngineMessage, validUiMessage } from '../src/bridge';

const bridgeId = 'bridge-test';
const ready = () => ({ kind: 'uiReady' as const, protocolVersion: 1 as const, bridgeId });
describe('HUD IPC', () => {
  afterEach(() => { cleanup(); window.__SPACEGAME_HUD__ = undefined; });
  it('sends uiReady after subscribing and renders pushed snapshots', async () => {
    const sendJson = vi.fn(); let listener: ((raw: unknown) => void) | undefined;
    window.__SPACEGAME_HUD__ = { sendJson, subscribe: (next) => { listener = next; return () => {}; } };
    render(App);
    const readyMessage = JSON.parse(sendJson.mock.calls[0]?.[0] ?? '{}');
    expect(readyMessage.kind).toBe('uiReady');
    listener?.(JSON.stringify({ kind: 'connectionStateChanged', protocolVersion: 1, bridgeId: readyMessage.bridgeId, state: { stage: 'connected', requestId: 'request-1', address: 'x', localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } } }));
    await tick();
    expect(screen.getByText('LOCAL COMMAND')).toBeTruthy();
  });
  it('sends a request-scoped connect command', async () => {
    const sendJson = vi.fn(); window.__SPACEGAME_HUD__ = { sendJson, subscribe: () => () => {} }; render(App);
    await fireEvent.input(screen.getByLabelText('Server address'), { target: { value: 'play.example:4000' } }); await fireEvent.click(screen.getByText('Connect'));
    const message = JSON.parse(sendJson.mock.calls.at(-1)?.[0] ?? '{}'); expect(message.kind).toBe('connectRequested'); expect(message.address).toBe('play.example:4000'); expect(message.requestId).toMatch(/^request-/);
  });
  it('validates strict directional messages', () => {
    expect(validEngineMessage({ kind: 'heartbeat', protocolVersion: 1, bridgeId, sequence: 1 })).toBe(true);
    expect(validEngineMessage({ kind: 'uiReady', protocolVersion: 1, bridgeId })).toBe(false);
    expect(validUiMessage(ready())).toBe(true);
    expect(validUiMessage({ kind: 'connectionStateChanged', protocolVersion: 1, bridgeId, state: {} })).toBe(false);
  });
});
