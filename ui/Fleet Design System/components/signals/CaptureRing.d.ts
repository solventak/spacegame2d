/**
 * Segmented circular capture ring, drawn on the objective in the world.
 */
export interface CaptureRingProps {
  /** 0–1 capture progress. */
  value?: number;
  /** Segment count — 8 matches the 8-second Shield Relay. */
  segments?: number;
  tone?: 'friendly' | 'enemy' | 'neutral';
  /** fill = capturing, contested = paused (striped segments), decay = reverse drain after abandonment. */
  mode?: 'fill' | 'contested' | 'decay';
  size?: number;
  /** Centre readout, e.g. "5.2". */
  label?: string;
  /** Centre caption under the readout. */
  sub?: string;
  style?: React.CSSProperties;
}
export declare function CaptureRing(props: CaptureRingProps): JSX.Element;
