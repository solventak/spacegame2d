/**
 * Extremely slim top bar carrying the match's decisive objective.
 */
export interface ObjectiveCore { label: string; tone: 'friendly' | 'enemy'; shielded: boolean }
export interface ObjectiveBarProps {
  /** Compact friendly strategic state (top-left slot). */
  left?: React.ReactNode;
  /** Scouting confidence / last-seen enemy info (top-right slot). */
  right?: React.ReactNode;
  objective?: {
    name?: string;
    /** 0–1 capture progress. */
    value?: number;
    /** Capture duration in seconds (8 for the Shield Relay). */
    seconds?: number;
    /** Elapsed capture seconds. */
    elapsed?: number;
    mode?: 'fill' | 'contested' | 'decay';
    tone?: 'friendly' | 'enemy';
    /** Command Core shield states. */
    cores?: ObjectiveCore[];
  };
  style?: React.CSSProperties;
}
export declare function ObjectiveBar(props: ObjectiveBarProps): JSX.Element;
