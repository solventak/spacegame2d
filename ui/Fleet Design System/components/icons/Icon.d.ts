/** Interface icon from the inlined Lucide set; paints in currentColor. */
export type IconName = 'lock' | 'unlock' | 'shield' | 'shield-off' | 'radar' | 'crosshair' | 'chevron-right' | 'x' | 'triangle-alert' | 'eye-off' | 'git-branch' | 'activity' | 'clock' | 'circle-dot';
export interface IconProps {
  /** Lucide icon name from the inlined set. */
  name: IconName;
  size?: number;
  /** Override the stroke colour; defaults to currentColor. */
  strokeColor?: string;
  strokeWidth?: number;
  title?: string;
  style?: React.CSSProperties;
}
export declare function Icon(props: IconProps): JSX.Element | null;
