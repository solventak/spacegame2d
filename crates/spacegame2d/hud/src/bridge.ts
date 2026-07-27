import type { HudBootstrap, LocalPlayerHudModel } from './model';
declare global { interface Window { __SPACEGAME_HUD__?: HudBootstrap; } }
function validPlayer(value: unknown): value is LocalPlayerHudModel {
  if (!value || typeof value !== 'object') return false;
  const player = value as Record<string, unknown>;
  return player.schemaVersion === 1 && Number.isInteger(player.playerSlot) && (player.playerSlot as number) > 0 && (player.color === 'cyan' || player.color === 'coral') && typeof player.colorHex === 'string' && /^#[0-9A-F]{6}$/i.test(player.colorHex);
}
export function readBootstrap(): HudBootstrap {
  const bootstrap = window.__SPACEGAME_HUD__;
  if (!bootstrap || !validPlayer(bootstrap.localPlayer)) throw new Error('Invalid HUD bootstrap model');
  return bootstrap;
}
