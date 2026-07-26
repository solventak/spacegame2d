/** Small geometric tactical glyph set: system type, fleet composition, stance, formation, target priority. */
export type GlyphName = 'core' | 'relay' | 'gate' | 'outpost' | 'drone' | 'capital' | 'aggressive' | 'defensive' | 'screen' | 'wedge' | 'dispersed' | 'priorityCapital' | 'warp' | 'unknown';
export interface GlyphProps {
  name?: GlyphName;
  size?: number;
  tone?: 'inherit' | 'friendly' | 'enemy' | 'neutral';
  strokeWidth?: number;
  /** Accessible label; omit for decorative use. */
  title?: string;
  style?: React.CSSProperties;
}
export declare function Glyph(props: GlyphProps): JSX.Element;
