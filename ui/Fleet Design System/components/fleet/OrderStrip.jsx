import React from 'react';
import { Glyph } from '../signals/Glyph.jsx';
import { Icon } from '../icons/Icon.jsx';

/** Bottom-centre order confirmation strip — the player's proof an irreversible order landed. */
export function OrderStrip({ fleets = [], destination, state = 'preview', travel, arrival, onConfirm, onCancel, style }) {
  const committed = state === 'committed';
  const c = committed ? 'var(--sig-friendly)' : 'var(--sig-neutral)';
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--sp-3)', padding: '7px 10px', background: 'var(--surface-panel)', border: '1px ' + (committed ? 'solid' : 'dashed') + ' ' + (committed ? 'var(--sig-friendly-edge)' : 'var(--sig-neutral-edge)'), borderRadius: 'var(--r-1)', backdropFilter: 'var(--blur-panel)', boxShadow: 'var(--shadow-panel)', ...style }}>
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5, font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: c }}>
        {committed ? <Icon name="lock" size={11} /> : <Glyph name="unknown" size={11} />}
        {committed ? 'warp committed' : 'preview order'}
      </span>
      <i style={{ width: 1, height: 18, background: 'var(--line-hairline)' }} />
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
        {fleets.map((f) => <em key={f} style={{ font: 'var(--type-readout)', fontStyle: 'normal', fontSize: 'var(--fs-body)', color: 'var(--text-1)' }}>{f}</em>)}
        <Glyph name="warp" size={14} tone={committed ? 'friendly' : 'neutral'} />
        <em style={{ font: 'var(--type-readout)', fontStyle: 'normal', fontSize: 'var(--fs-body)', color: committed ? 'var(--cyan-300)' : 'var(--text-2)' }}>{destination}</em>
      </span>
      <i style={{ width: 1, height: 18, background: 'var(--line-hairline)' }} />
      <span style={{ display: 'inline-flex', alignItems: 'baseline', gap: 10, font: 'var(--type-readout)', fontSize: 'var(--fs-body)', color: 'var(--text-2)', fontVariantNumeric: 'tabular-nums' }}>
        <span>{travel}<em style={{ font: 'var(--type-label)', fontStyle: 'normal', color: 'var(--text-3)', marginLeft: 3 }}>travel</em></span>
        <span style={{ color: committed ? 'var(--text-1)' : 'var(--text-3)' }}>{arrival}<em style={{ font: 'var(--type-label)', fontStyle: 'normal', color: 'var(--text-3)', marginLeft: 3 }}>arrival</em></span>
      </span>
      {!committed && (onConfirm || onCancel) && (
        <span style={{ display: 'inline-flex', gap: 6, marginLeft: 2 }}>
          {onCancel && <button type="button" onClick={onCancel} style={{ all: 'unset', cursor: 'pointer', padding: 'var(--pad-chip)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>cancel</button>}
          {onConfirm && <button type="button" onClick={onConfirm} style={{ all: 'unset', cursor: 'pointer', padding: 'var(--pad-chip)', border: '1px solid var(--sig-friendly-edge)', borderRadius: 'var(--r-2)', background: 'var(--sig-friendly-wash)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--cyan-300)' }}>commit warp</button>}
        </span>
      )}
    </div>
  );
}
