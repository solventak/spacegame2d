import React from 'react';
import { Icon } from '../icons/Icon.jsx';

/** Square hairline icon control for HUD affordances. */
export function IconButton({ icon, active = false, size = 22, title, onClick, style }) {
  return (
    <button type="button" title={title} aria-label={title} onClick={onClick} style={{ all: 'unset', boxSizing: 'border-box', cursor: 'pointer', display: 'grid', placeItems: 'center', width: size, height: size, border: '1px solid ' + (active ? 'var(--sig-friendly-edge)' : 'var(--line-hairline)'), borderRadius: 'var(--r-1)', background: active ? 'var(--sig-friendly-wash)' : 'transparent', color: active ? 'var(--cyan-300)' : 'var(--text-2)', transition: 'all var(--dur-instant) var(--ease-out)', ...style }}>
      <Icon name={icon} size={Math.round(size * 0.55)} />
    </button>
  );
}
