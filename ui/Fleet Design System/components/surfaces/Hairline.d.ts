/** 1px hairline rule for splitting panel sections. */
export interface HairlineProps {
  /** Render as a vertical divider. */
  vertical?: boolean;
  /** Inset in px along the rule's long axis. */
  inset?: number;
  /** Use the stronger hairline value. */
  strong?: boolean;
  style?: React.CSSProperties;
}
export declare function Hairline(props: HairlineProps): JSX.Element;
