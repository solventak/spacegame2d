/** One front in the bottom-right roster, ordered by urgency. */
export interface FrontRowProps {
  name: string;
  /** Short state phrase, e.g. "contested", "inbound 12s", "quiet". */
  status: string;
  urgency?: 'critical' | 'active' | 'quiet';
  tone?: 'friendly' | 'enemy' | 'neutral';
  /** Tactical glyph for the front's system type. */
  glyph?: string;
  /** 0–1; omit for fronts with no objective in progress. */
  progress?: number;
  progressMode?: 'fill' | 'contested' | 'decay';
  selected?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function FrontRow(props: FrontRowProps): JSX.Element;
