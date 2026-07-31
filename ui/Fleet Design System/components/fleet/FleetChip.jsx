import React from 'react';
import { Meter } from '../signals/Meter.jsx';
import { Glyph } from '../signals/Glyph.jsx';
import { Icon } from '../icons/Icon.jsx';

/** Compact fleet row: number, composition glyphs, strength bar, commitment state. */
export function FleetChip({ id, strength = 1, state = 'idle', drones, capitals, tone = 'friendly', selected = false, onClick, style }) {
  const c = tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const stateLabel = { idle: 'holding', moving: 'en route', committed: 'committed', engaged: 'engaged', planned: 'planned' }[state] || state;
  const planned = state === 'planned';
  return (
    <button type="button" onClick={onClick} style={{ all: 'unset', cursor: 'pointer', display: 'grid', gridTemplateColumns: '34px 1fr auto', alignItems: 'center', gap: 'var(--sp-2)', padding: 'var(--pad-row)', background: selected ? 'var(--surface-row-active)' : 'var(--surface-row)', border: '1px solid ' + (selected ? 'var(--sig-friendly-edge)' : 'transparent'), borderRadius: 'var(--r-1)', transition: 'background var(--dur-instant) var(--ease-out)', ...style }}>
      <span style={{ font: 'var(--type-readout)', color: c, letterSpacing: 'var(--tracking-num)' }}>{id}</span>
      <span style={{ display: 'grid', gap: 4 }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--text-3)' }}>
          {capitals ? <><Glyph name="capital" size={10} /><em style={{ font: 'var(--type-label)', fontStyle: 'normal' }}>{capitals}</em></> : null}
          {drones ? <><Glyph name="drone" size={10} /><em style={{ font: 'var(--type-label)', fontStyle: 'normal' }}>{drones}</em></> : null}
        </span>
        <Meter value={strength} tone={tone} mode={planned ? 'contested' : 'fill'} height={2} />
      </span>
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: state === 'committed' ? c : 'var(--text-3)' }}>
        {state === 'committed' && <Icon name="lock" size={9} />}
        {planned && <Glyph name="unknown" size={9} />}
        {stateLabel}
      </span>
    </button>
  );
}
