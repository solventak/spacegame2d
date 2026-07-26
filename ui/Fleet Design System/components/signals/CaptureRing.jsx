import React from 'react';

/** Large segmented circular capture ring drawn directly on an objective in the world. */
export function CaptureRing({ value = 0, segments = 8, tone = 'friendly', mode = 'fill', size = 132, label, sub, style }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone];
  const r = size / 2 - 8, cx = size / 2, gap = 3.2, step = 360 / segments;
  const filled = Math.max(0, Math.min(1, value)) * segments;
  const arcs = Array.from({ length: segments }, (_, i) => {
    const a0 = -90 + i * step + gap / 2, a1 = -90 + (i + 1) * step - gap / 2;
    const p = (a) => [cx + r * Math.cos(a * Math.PI / 180), cx + r * Math.sin(a * Math.PI / 180)];
    const [x0, y0] = p(a0), [x1, y1] = p(a1);
    const on = i < Math.floor(filled), partial = i === Math.floor(filled) && filled % 1 > 0.05;
    return { d: `M ${x0} ${y0} A ${r} ${r} 0 0 1 ${x1} ${y1}`, on, partial };
  });
  const stroke = mode === 'decay' ? 'var(--sig-neutral)' : c;
  return (
    <div style={{ position: 'relative', width: size, height: size, ...style }}>
      <svg width={size} height={size} style={{ display: 'block', overflow: 'visible' }}>
        {mode === 'contested' && (
          <defs>
            <pattern id={'cr-stripe-' + tone} width="6" height="6" patternTransform="rotate(-45)" patternUnits="userSpaceOnUse">
              <rect width="3" height="6" fill={c} />
            </pattern>
          </defs>
        )}
        {arcs.map((a, i) => (
          <path key={i} d={a.d} fill="none" strokeLinecap="butt" strokeWidth={a.on || a.partial ? 6 : 2}
            stroke={a.on ? (mode === 'contested' ? `url(#cr-stripe-${tone})` : stroke) : a.partial ? stroke : 'rgba(160,178,196,.20)'}
            strokeDasharray={a.partial ? '3 3' : undefined}
            style={{ opacity: mode === 'decay' && a.on ? 0.7 : 1, transition: 'stroke-width var(--dur-fast) var(--ease-out)' }} />
        ))}
      </svg>
      <div style={{ position: 'absolute', inset: 0, display: 'grid', placeContent: 'center', textAlign: 'center', gap: 2 }}>
        {label && <div style={{ font: 'var(--type-hero)', color: mode === 'decay' ? 'var(--sig-neutral)' : c, letterSpacing: 'var(--tracking-num)', fontVariantNumeric: 'tabular-nums' }}>{label}</div>}
        {sub && <div style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>{sub}</div>}
      </div>
    </div>
  );
}
