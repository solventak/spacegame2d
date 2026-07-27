import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import App from '../src/App.svelte';
import { readBootstrap } from '../src/bridge';
describe('HUD', () => {
  it('renders the Rust-provided cyan model', () => { render(App, { player: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } }); expect(screen.getByText('LOCAL COMMAND')).toBeTruthy(); expect(screen.getByText('01')).toBeTruthy(); expect(screen.getAllByText('CYAN')).toHaveLength(2); });
  it('renders coral and rejects malformed bootstrap data', () => { render(App, { player: { schemaVersion: 1, playerSlot: 2, color: 'coral', colorHex: '#FF6A47' } }); expect(screen.getAllByText('CORAL')).toHaveLength(2); window.__SPACEGAME_HUD__ = { localPlayer: { schemaVersion: 2, playerSlot: 0, color: 'cyan', colorHex: 'no' } } as never; expect(() => readBootstrap()).toThrow('Invalid HUD bootstrap model'); });

  it('mounts the production entrypoint with the Svelte 5 API', async () => {
    document.body.innerHTML = '<div id="app"></div>';
    window.__SPACEGAME_HUD__ = { localPlayer: { schemaVersion: 1, playerSlot: 1, color: 'cyan', colorHex: '#22CFE8' } };
    await import('../src/main');
    expect(document.body.textContent).toContain('PLAYER');
    expect(document.body.textContent).toContain('01');
  });
});
