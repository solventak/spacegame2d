/** Compact fleet row: number, composition, strength, commitment state. */
export interface FleetChipProps {
  /** Fleet number, e.g. "04". */
  id: string;
  /** 0–1 relative strength. */
  strength?: number;
  state?: 'idle' | 'moving' | 'committed' | 'engaged' | 'planned';
  /** Drone count, formatted by the caller. */
  drones?: string | number;
  /** Capital ship count. */
  capitals?: string | number;
  tone?: 'friendly' | 'enemy';
  selected?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function FleetChip(props: FleetChipProps): JSX.Element;
