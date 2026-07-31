/** Editable doctrine setting — stance, formation, or target priority. */
export interface DoctrinePillProps {
  /** Tooltip / field name, e.g. "stance". */
  label?: string;
  /** Current value, e.g. "aggressive". */
  value: string;
  /** Glyph name from the tactical set. */
  glyph?: string;
  active?: boolean;
  /** Multi-fleet selection with differing values — renders dashed "mixed". */
  mixed?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function DoctrinePill(props: DoctrinePillProps): JSX.Element;
