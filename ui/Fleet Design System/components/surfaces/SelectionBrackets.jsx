import React from 'react';

/** Thin corner brackets: the universal Fleet mark for "this is selected". */
export function SelectionBrackets({ tone = 'friendly', size = 10, inset = 0, children, style }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone];
  const base = { position: 'absolute', width: size, height: size, pointerEvents: 'none' };
  return (
    <div style={{ position: 'relative', display: 'inline-block', ...style }}>
      {children}
      <i style={{ ...base, top: inset, left: inset, borderTop: '1px solid ' + c, borderLeft: '1px solid ' + c }} />
      <i style={{ ...base, top: inset, right: inset, borderTop: '1px solid ' + c, borderRight: '1px solid ' + c }} />
      <i style={{ ...base, bottom: inset, left: inset, borderBottom: '1px solid ' + c, borderLeft: '1px solid ' + c }} />
      <i style={{ ...base, bottom: inset, right: inset, borderBottom: '1px solid ' + c, borderRight: '1px solid ' + c }} />
    </div>
  );
}
