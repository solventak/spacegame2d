import React from 'react';
import { Glyph } from '../signals/Glyph.jsx';

/** Editable doctrine setting: stance, formation, or target priority. */
export function DoctrinePill({ label, value, glyph, active = false, mixed = false, onClick, style }) {
  return (
    <button type="button" onClick={onClick} title={label} style={{ all: 'unset', cursor: 'pointer', display: 'inline-flex', alignItems: 'center', gap: 5, padding: 'var(--pad-chip)', background: active ? 'var(--sig-friendly-wash)' : 'transparent', border: '1px ' + (mixed ? 'dashed' : 'solid') + ' ' + (active ? 'var(--sig-friendly-edge)' : 'var(--line-hairline)'), borderRadius: 'var(--r-2)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: mixed ? 'var(--text-3)' : active ? 'var(--cyan-300)' : 'var(--text-2)', transition: 'all var(--dur-instant) var(--ease-out)', ...style }}>
      {glyph && <Glyph name={glyph} size={11} />}
      {mixed ? 'mixed' : value}
    </button>
  );
}
