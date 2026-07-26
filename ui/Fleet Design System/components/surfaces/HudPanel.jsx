import React from 'react';

/** Translucent graphite panel with hairline outline — the only chrome container in Fleet. */
export function HudPanel({ title, meta, tone = 'neutral', brackets = false, dense = false, style, children, ...rest }) {
  const edge = { neutral: 'var(--line-hairline)', friendly: 'var(--sig-friendly-edge)', enemy: 'var(--sig-enemy-edge)' }[tone];
  return (
    <section {...rest} style={{ position: 'relative', background: 'var(--surface-panel)', border: '1px solid ' + edge, borderRadius: 'var(--r-1)', backdropFilter: 'var(--blur-panel)', boxShadow: 'var(--shadow-panel)', padding: dense ? 'var(--pad-panel-tight)' : 'var(--pad-panel)', ...style }}>
      {(title || meta) && (
        <header style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 'var(--sp-2)', marginBottom: dense ? 'var(--sp-2)' : 'var(--sp-3)' }}>
          {title && <h2 style={{ margin: 0, font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-2)' }}>{title}</h2>}
          {meta && <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>{meta}</span>}
        </header>
      )}
      {children}
      {brackets && <Brackets color={edge} />}
    </section>
  );
}

function Brackets({ color }) {
  const base = { position: 'absolute', width: 'var(--bracket-len)', height: 'var(--bracket-len)', pointerEvents: 'none' };
  const c = 'var(--line-bracket)';
  return (
    <>
      <i style={{ ...base, top: -1, left: -1, borderTop: '1px solid ' + c, borderLeft: '1px solid ' + c }} />
      <i style={{ ...base, top: -1, right: -1, borderTop: '1px solid ' + c, borderRight: '1px solid ' + c }} />
      <i style={{ ...base, bottom: -1, left: -1, borderBottom: '1px solid ' + c, borderLeft: '1px solid ' + c }} />
      <i style={{ ...base, bottom: -1, right: -1, borderBottom: '1px solid ' + c, borderRight: '1px solid ' + c }} />
    </>
  );
}
