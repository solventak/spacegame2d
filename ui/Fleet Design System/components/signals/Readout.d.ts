/** Label + tabular numeric value + unit — the atom of every Fleet panel. */
export interface ReadoutProps {
  label?: string;
  value: React.ReactNode;
  /** Unit suffix, rendered small and dim. */
  unit?: string;
  tone?: 'default' | 'friendly' | 'enemy' | 'neutral';
  size?: 'sm' | 'md' | 'lg';
  align?: 'left' | 'right';
  /** Dim the value to signal known-outdated information. */
  stale?: boolean;
  style?: React.CSSProperties;
}
export declare function Readout(props: ReadoutProps): JSX.Element;
