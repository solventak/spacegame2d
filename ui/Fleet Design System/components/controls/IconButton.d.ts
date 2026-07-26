/** Square hairline icon control for HUD affordances. */
export interface IconButtonProps {
  /** Lucide icon name. */
  icon: string;
  active?: boolean;
  size?: number;
  /** Accessible label + tooltip. */
  title?: string;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function IconButton(props: IconButtonProps): JSX.Element;
