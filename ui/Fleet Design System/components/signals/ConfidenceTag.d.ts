/** Small tag stating how trustworthy a piece of information is. */
export interface ConfidenceTagProps {
  /** confirmed = solid outline; estimated / stale / unknown = dashed, gray. */
  level?: 'confirmed' | 'estimated' | 'stale' | 'unknown';
  tone?: 'friendly' | 'enemy' | 'neutral';
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function ConfidenceTag(props: ConfidenceTagProps): JSX.Element;
