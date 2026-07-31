import React from 'react';

/** Linear meter. mode carries the meaning: fill / contested (stripes) / decay (reverse-draining gray). */
export function Meter({ value = 0, tone = 'friendly', mode = 'fill', height = 3, width, showTrack = true, style }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone];
  const stripe = tone === 'enemy' ? 'var(--stripe-contested-enemy)' : 'var(--stripe-contested-friendly)';
  const fill = mode === 'contested' ? stripe : mode === 'decay' ? 'var(--stripe-decay)' : c;
  return (
    <div style={{ position: 'relative', width: width || '100%', height, background: showTrack ? 'rgba(255,255,255,.07)' : 'transparent', borderRadius: 'var(--r-full)', overflow: 'hidden', ...style }}>
      <div style={{ position: 'absolute', inset: 0, right: 'auto', width: Math.max(0, Math.min(1, value)) * 100 + '%', background: fill, transition: 'width var(--dur-slow) var(--ease-linear)' }} />
    </div>
  );
}
