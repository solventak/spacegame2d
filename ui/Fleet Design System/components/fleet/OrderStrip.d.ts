/**
 * Bottom-centre order confirmation strip — proof that an irreversible order succeeded.
 */
export interface OrderStripProps {
  /** Fleet numbers included in the order. */
  fleets?: string[];
  /** Destination gate or system name. */
  destination?: string;
  /** preview = dashed, cancellable; committed = solid, locked. */
  state?: 'preview' | 'committed';
  /** Travel time, e.g. "18.0s". */
  travel?: string;
  /** Arrival clock, e.g. "T+04:12". */
  arrival?: string;
  onConfirm?: () => void;
  onCancel?: () => void;
  style?: React.CSSProperties;
}
export declare function OrderStrip(props: OrderStripProps): JSX.Element;
