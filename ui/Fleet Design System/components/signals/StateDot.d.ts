/** 6px allegiance dot; certainty is carried by fill and border style. */
export interface StateDotProps {
  tone?: 'friendly' | 'enemy' | 'neutral';
  /** confirmed = solid fill, estimated = dashed hollow, stale = dimmed hollow. */
  certainty?: 'confirmed' | 'estimated' | 'stale';
  size?: number;
  style?: React.CSSProperties;
}
export declare function StateDot(props: StateDotProps): JSX.Element;
