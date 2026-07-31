import React from 'react';
import { StateDot } from '../signals/StateDot.jsx';
import { Glyph } from '../signals/Glyph.jsx';
import { Meter } from '../signals/Meter.jsx';

/** One entry in the front roster, ordered by urgency. */
export function FrontRow({ name, status, urgency = 'quiet', tone = 'neutral', glyph = 'gate', progress, progressMode = 'fill', selected = false, onClick, style }) {
  const urg = { critical: 'var(--sig-enemy)', active: 'var(--sig-friendly)', quiet: 'var(--sig-neutral)' }[urgency];
  return (
    <button type="button" onClick={onClick} style={{ all: 'unset', cursor: 'pointer', display: 'grid', gridTemplateColumns: '3px 14px 1fr auto', alignItems: 'center', gap: 'var(--sp-2)', padding: 'var(--pad-row)', background: selected ? 'var(--surface-row-active)' : 'transparent', borderBottom: '1px solid var(--line-hairline)', transition: 'background var(--dur-instant) var(--ease-out)', ...style }}>
      <i style={{ width: 2, height: 18, background: urg, opacity: urgency === 'quiet' ? .45 : 1 }} />
      <Glyph name={glyph} size={12} tone={tone === 'neutral' ? 'neutral' : tone} />
      <span style={{ display: 'grid', gap: 3, minWidth: 0 }}>
        <span style={{ font: 'var(--type-body)', fontFamily: 'var(--font-condensed)', fontWeight: 'var(--fw-semi)', letterSpacing: 'var(--tracking-wide)', color: 'var(--text-1)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{name}</span>
        {progress != null && <Meter value={progress} tone={tone === 'neutral' ? 'neutral' : tone} mode={progressMode} height={2} />}
      </span>
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: urgency === 'quiet' ? 'var(--text-3)' : urg, whiteSpace: 'nowrap' }}>
        <StateDot tone={tone} certainty={urgency === 'quiet' ? 'stale' : 'confirmed'} size={5} />{status}
      </span>
    </button>
  );
}
