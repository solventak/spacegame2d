import React from 'react';

/** Marks how trustworthy a piece of information is. Dashed outline = not confirmed. */
export function ConfidenceTag({ level = 'confirmed', children, tone = 'neutral', style }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone];
  const solid = level === 'confirmed';
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, padding: 'var(--pad-chip)', border: (solid ? '1px solid ' : '1px dashed ') + (solid ? c : 'var(--sig-neutral-edge)'), borderRadius: 'var(--r-2)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: solid ? c : 'var(--text-3)', opacity: level === 'stale' ? 0.75 : 1, ...style }}>
      {children || level}
    </span>
  );
}
