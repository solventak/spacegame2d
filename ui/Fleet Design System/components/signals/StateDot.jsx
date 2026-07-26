import React from 'react';

/** 6px allegiance dot. Hollow = unconfirmed, dashed = estimated. */
export function StateDot({ tone = 'neutral', certainty = 'confirmed', size = 6, style }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone];
  const border = certainty === 'estimated' ? '1px dashed ' + c : '1px solid ' + c;
  return <i style={{ display: 'inline-block', width: size, height: size, borderRadius: 'var(--r-full)', border, background: certainty === 'confirmed' ? c : 'transparent', opacity: certainty === 'stale' ? 0.5 : 1, ...style }} />;
}
