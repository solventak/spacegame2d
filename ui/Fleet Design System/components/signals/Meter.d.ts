/** Linear progress / strength meter whose fill treatment encodes state. */
export interface MeterProps {
  /** 0–1. */
  value?: number;
  tone?: 'friendly' | 'enemy' | 'neutral';
  /** fill = advancing, contested = paused (diagonal stripes), decay = draining after abandonment. */
  mode?: 'fill' | 'contested' | 'decay';
  height?: number;
  width?: number | string;
  showTrack?: boolean;
  style?: React.CSSProperties;
}
export declare function Meter(props: MeterProps): JSX.Element;
