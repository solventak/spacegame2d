import React from 'react';

/** Label + tabular value + unit. The atom of every Fleet panel. */
export function Readout({ label, value, unit, tone = 'default', size = 'md', align = 'left', stale = false, style }) {
  const col = { default: 'var(--text-1)', friendly: 'var(--text-friendly)', enemy: 'var(--text-enemy)', neutral: 'var(--text-2)' }[tone];
  const fs = { sm: 'var(--fs-body)', md: 'var(--fs-readout)', lg: 'var(--fs-hero)' }[size];
  return (
    <div style={{ display: 'grid', gap: 2, justifyItems: align === 'right' ? 'end' : 'start', ...style }}>
      {label && <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>{label}</span>}
      <span style={{ font: 'var(--type-readout)', fontSize: fs, color: stale ? 'var(--text-stale)' : col, letterSpacing: 'var(--tracking-num)', fontVariantNumeric: 'tabular-nums', display: 'inline-flex', alignItems: 'baseline', gap: 3 }}>
        {value}{unit && <em style={{ font: 'var(--type-label)', fontStyle: 'normal', color: 'var(--text-3)' }}>{unit}</em>}
      </span>
    </div>
  );
}
