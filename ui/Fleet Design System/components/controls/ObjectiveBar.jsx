import React from 'react';
import { Meter } from '../signals/Meter.jsx';
import { Glyph } from '../signals/Glyph.jsx';
import { Icon } from '../icons/Icon.jsx';

/** Extremely slim top bar: friendly state left, decisive objective centre, scouting confidence right. */
export function ObjectiveBar({ left, right, objective = {}, style }) {
  const { name = 'Shield Relay', value = 0, seconds = 8, elapsed = 0, mode = 'fill', tone = 'friendly', cores = [] } = objective;
  const c = mode === 'decay' ? 'var(--sig-neutral)' : tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const stateWord = mode === 'contested' ? 'contested' : mode === 'decay' ? 'decaying' : tone === 'enemy' ? 'enemy capturing' : 'capturing';
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr auto 1fr', alignItems: 'center', gap: 'var(--sp-4)', height: 'var(--hud-topbar-h)', padding: '0 var(--sp-3)', background: 'var(--surface-panel)', borderBottom: '1px solid var(--line-hairline)', backdropFilter: 'var(--blur-panel)', ...style }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--sp-4)', minWidth: 0 }}>{left}</div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--sp-3)' }}>
        <Glyph name="relay" size={13} tone={mode === 'decay' ? 'neutral' : tone} />
        <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-2)', whiteSpace: 'nowrap' }}>{name}</span>
        <span style={{ display: 'grid', gap: 3, width: 190 }}>
          <Meter value={value} tone={tone} mode={mode} height={3} />
          <span style={{ display: 'flex', justifyContent: 'space-between', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: c }}>
            <em style={{ fontStyle: 'normal' }}>{stateWord}</em>
            <em style={{ fontStyle: 'normal', fontFamily: 'var(--font-mono)', fontVariantNumeric: 'tabular-nums', color: 'var(--text-2)' }}>{elapsed.toFixed(1)} / {seconds}s</em>
          </span>
        </span>
        {cores.map((core) => (
          <span key={core.label} style={{ display: 'inline-flex', alignItems: 'center', gap: 4, padding: 'var(--pad-chip)', border: '1px solid ' + (core.tone === 'enemy' ? 'var(--sig-enemy-edge)' : 'var(--sig-friendly-edge)'), borderRadius: 'var(--r-2)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: core.tone === 'enemy' ? 'var(--coral-300)' : 'var(--cyan-300)' }}>
            <Icon name={core.shielded ? 'shield' : 'shield-off'} size={10} />{core.label}
            <em style={{ fontStyle: 'normal', color: core.shielded ? 'var(--text-3)' : core.tone === 'enemy' ? 'var(--coral-300)' : 'var(--cyan-300)' }}>{core.shielded ? 'shielded' : 'exposed'}</em>
          </span>
        ))}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: 'var(--sp-4)', minWidth: 0 }}>{right}</div>
    </div>
  );
}
