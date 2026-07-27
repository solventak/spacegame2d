import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import App from '../src/App.svelte';
import { readState, validState } from '../src/bridge';

const connected = { schemaVersion: 1 as const, kind: 'connected' as const, address: 'localhost:4000', localPlayer: { schemaVersion: 1 as const, playerSlot: 1, color: 'cyan' as const, colorHex: '#22CFE8' } };
const disconnected = { schemaVersion: 1 as const, kind: 'disconnected' as const, address: 'localhost:4000', reason: 'startup' as const };

describe('HUD', () => {
  afterEach(() => { cleanup(); window.__SPACEGAME_HUD__ = undefined; });
  it('renders the connected compact player model', () => {
    render(App, { initialState: connected });
    expect(screen.getByText('LOCAL COMMAND')).toBeTruthy();
    expect(screen.getByText('01')).toBeTruthy();
    expect(screen.getAllByText('CYAN')).toHaveLength(2);
  });

  it('renders an editable connection form and sends Connect', async () => {
    const send = vi.fn();
    window.__SPACEGAME_HUD__ = { getState: () => disconnected, subscribe: () => () => {}, send };
    render(App, { initialState: disconnected });
    const input = screen.getByLabelText('Server address');
    await fireEvent.input(input, { target: { value: 'play.example:4000' } });
    await fireEvent.click(screen.getByText('Connect'));
    expect(send).toHaveBeenCalledWith({ schemaVersion: 1, kind: 'connect', address: 'play.example:4000' });
  });

  it('renders Connecting with disabled address and Cancel', () => {
    render(App, { initialState: { schemaVersion: 1, kind: 'connecting', address: 'localhost:4000' } });
    expect((screen.getByLabelText('Server address') as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText('Cancel')).toBeTruthy();
  });

  it('renders useful messages for every failed connection outcome', () => {
    for (const [state, message] of [
      [{ schemaVersion: 1, kind: 'connectionFailed', address: 'x', reason: 'timeout' }, 'Connection failed. Check the address and try again.'],
      [{ schemaVersion: 1, kind: 'rejected', address: 'x' }, 'The server rejected this connection.'],
      [{ schemaVersion: 1, kind: 'serverFull', address: 'x' }, 'The server is full. Try again later.'],
      [{ schemaVersion: 1, kind: 'versionMismatch', address: 'x' }, 'Client and server versions do not match. Update the client.'],
    ] as const) {
      const view = render(App, { initialState: state });
      expect(view.getByText(message)).toBeTruthy();
      expect(view.getByText('Connect')).toBeTruthy();
      view.unmount();
    }
  });

  it('renders each disconnected reason as an editable form', () => {
    for (const [reason, message] of [
      ['startup', 'Enter a server address to join.'],
      ['cancelled', 'Enter a server address to join.'],
      ['sessionLost', 'Disconnected from the server.'],
    ] as const) {
      const view = render(App, { initialState: { schemaVersion: 1, kind: 'disconnected', address: 'x', reason } });
      expect(view.getByText(message)).toBeTruthy();
      expect((view.getByLabelText('Server address') as HTMLInputElement).disabled).toBe(false);
      expect(view.getByText('Connect')).toBeTruthy();
      view.unmount();
    }
  });

  it('accepts every supported bridge state', () => {
    window.__SPACEGAME_HUD__ = { getState: () => disconnected, subscribe: () => () => {}, send: () => {} };
    expect(readState()).toEqual(disconnected);
    for (const state of [
      disconnected,
      { schemaVersion: 1, kind: 'connecting', address: 'x' },
      connected,
      { schemaVersion: 1, kind: 'connectionFailed', address: 'x', reason: 'timeout' },
      { schemaVersion: 1, kind: 'rejected', address: 'x' },
      { schemaVersion: 1, kind: 'serverFull', address: 'x' },
      { schemaVersion: 1, kind: 'versionMismatch', address: 'x' },
    ] as const) expect(validState(state)).toBe(true);
    expect(validState({ schemaVersion: 2, kind: 'disconnected', address: 'x' })).toBe(false);
  });
});
