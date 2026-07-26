/** Thin corner brackets wrapper — the universal Fleet selection mark. */
export interface SelectionBracketsProps {
  tone?: 'friendly' | 'enemy' | 'neutral';
  /** Bracket arm length in px. */
  size?: number;
  /** Offset from the child's bounds; negative pushes outward. */
  inset?: number;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
export declare function SelectionBrackets(props: SelectionBracketsProps): JSX.Element;
