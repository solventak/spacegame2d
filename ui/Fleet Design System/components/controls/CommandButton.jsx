import React from 'react';
import { Icon } from '../icons/Icon.jsx';

/** Rectangular command control. commit = the irreversible action, ghost = everything else. */
export function CommandButton({ children, variant = 'ghost', size = 'md', icon, disabled = false, onClick, style }) {
  const v = {
    commit: { bg: 'var(--sig-friendly-wash)', border: 'var(--sig-friendly-edge)', color: 'var(--cyan-300)' },
    danger: { bg: 'var(--sig-enemy-wash)', border: 'var(--sig-enemy-edge)', color: 'var(--coral-300)' },
    ghost: { bg: 'transparent', border: 'var(--line-hairline)', color: 'var(--text-2)' },
  }[variant];
  const pad = size === 'sm' ? '3px 8px' : '6px 12px';
  return (
    <button type="button" disabled={disabled} onClick={onClick} style={{ all: 'unset', boxSizing: 'border-box', cursor: disabled ? 'not-allowed' : 'pointer', display: 'inline-flex', alignItems: 'center', gap: 6, padding: pad, background: v.bg, border: '1px solid ' + v.border, borderRadius: 'var(--r-2)', font: 'var(--type-label)', fontSize: size === 'sm' ? 'var(--fs-micro)' : 'var(--fs-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: v.color, opacity: disabled ? .38 : 1, transition: 'background var(--dur-instant) var(--ease-out), color var(--dur-instant) var(--ease-out)', ...style }}>
      {icon && <Icon name={icon} size={size === 'sm' ? 10 : 12} />}
      {children}
    </button>
  );
}
