/**
 * Translucent graphite HUD panel with hairline outline. The only container chrome in Fleet.
 */
export interface HudPanelProps {
  /** ALL-CAPS panel label rendered top-left. */
  title?: string;
  /** Small right-aligned qualifier (count, timestamp, confidence). */
  meta?: string;
  /** Outline allegiance. */
  tone?: 'neutral' | 'friendly' | 'enemy';
  /** Show selection corner brackets. */
  brackets?: boolean;
  /** Tighter padding for stacked sub-panels. */
  dense?: boolean;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}
export declare function HudPanel(props: HudPanelProps): JSX.Element;
