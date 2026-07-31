/** Rectangular command control. */
export interface CommandButtonProps {
  children?: React.ReactNode;
  /** commit = irreversible action (cyan), danger = destructive (coral), ghost = default. */
  variant?: 'commit' | 'danger' | 'ghost';
  size?: 'sm' | 'md';
  /** Lucide icon name. */
  icon?: string;
  disabled?: boolean;
  onClick?: () => void;
  style?: React.CSSProperties;
}
export declare function CommandButton(props: CommandButtonProps): JSX.Element;
