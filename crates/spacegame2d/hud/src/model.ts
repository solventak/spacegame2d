export type PlayerColor = 'cyan' | 'coral';
export interface LocalPlayerHudModel { schemaVersion: 1; playerSlot: number; color: PlayerColor; colorHex: string; }
export interface HudBootstrap { localPlayer: LocalPlayerHudModel; }
