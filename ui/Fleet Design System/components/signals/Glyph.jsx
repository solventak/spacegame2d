import React from 'react';

/** Small geometric tactical glyph: system type, fleet composition, stance, formation, target priority. */
const PATHS = {
  core: (s) => <><rect x={s*.18} y={s*.18} width={s*.64} height={s*.64} /><rect x={s*.38} y={s*.38} width={s*.24} height={s*.24} fill="currentColor" stroke="none" /></>,
  relay: (s) => <><circle cx={s/2} cy={s/2} r={s*.32} /><path d={`M${s/2} ${s*.06} V${s*.22} M${s/2} ${s*.78} V${s*.94} M${s*.06} ${s/2} H${s*.22} M${s*.78} ${s/2} H${s*.94}`} /></>,
  gate: (s) => <><path d={`M${s*.5} ${s*.1} L${s*.9} ${s*.5} L${s*.5} ${s*.9} L${s*.1} ${s*.5} Z`} /></>,
  outpost: (s) => <><path d={`M${s*.5} ${s*.12} L${s*.88} ${s*.86} L${s*.12} ${s*.86} Z`} /></>,
  drone: (s) => <><circle cx={s*.28} cy={s*.5} r={s*.1} fill="currentColor" stroke="none" /><circle cx={s*.56} cy={s*.32} r={s*.1} fill="currentColor" stroke="none" /><circle cx={s*.56} cy={s*.68} r={s*.1} fill="currentColor" stroke="none" /></>,
  capital: (s) => <><path d={`M${s*.12} ${s*.5} L${s*.5} ${s*.2} L${s*.88} ${s*.5} L${s*.5} ${s*.8} Z`} fill="currentColor" stroke="none" /></>,
  aggressive: (s) => <><path d={`M${s*.15} ${s*.75} L${s*.85} ${s*.25} M${s*.85} ${s*.25} H${s*.55} M${s*.85} ${s*.25} V${s*.55}`} /></>,
  defensive: (s) => <><path d={`M${s*.5} ${s*.12} L${s*.85} ${s*.28} V${s*.55} C${s*.85} ${s*.75} ${s*.68} ${s*.85} ${s*.5} ${s*.9} C${s*.32} ${s*.85} ${s*.15} ${s*.75} ${s*.15} ${s*.55} V${s*.28} Z`} /></>,
  screen: (s) => <><path d={`M${s*.12} ${s*.3} H${s*.88} M${s*.12} ${s*.7} H${s*.88}`} /></>,
  wedge: (s) => <><path d={`M${s*.5} ${s*.18} L${s*.86} ${s*.8} M${s*.5} ${s*.18} L${s*.14} ${s*.8}`} /></>,
  dispersed: (s) => <><circle cx={s*.25} cy={s*.3} r={s*.07} /><circle cx={s*.72} cy={s*.36} r={s*.07} /><circle cx={s*.4} cy={s*.72} r={s*.07} /><circle cx={s*.78} cy={s*.74} r={s*.07} /></>,
  priorityCapital: (s) => <><circle cx={s/2} cy={s/2} r={s*.34} /><path d={`M${s*.5} ${s*.16} V${s*.84} M${s*.16} ${s*.5} H${s*.84}`} /></>,
  warp: (s) => <><path d={`M${s*.14} ${s*.5} H${s*.8} M${s*.62} ${s*.3} L${s*.86} ${s*.5} L${s*.62} ${s*.7}`} /></>,
  unknown: (s) => <><circle cx={s/2} cy={s/2} r={s*.32} strokeDasharray="2 3" /></>,
};

export function Glyph({ name = 'unknown', size = 14, tone = 'inherit', strokeWidth = 1.25, style, title }) {
  const col = { inherit: 'currentColor', friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[tone] || 'currentColor';
  const draw = PATHS[name] || PATHS.unknown;
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ display: 'block', color: col, flex: 'none', ...style }} fill="none" stroke="currentColor" strokeWidth={strokeWidth} strokeLinecap="square" strokeLinejoin="miter" aria-label={title} role={title ? 'img' : 'presentation'}>
      {draw(size)}
    </svg>
  );
}

Glyph.names = Object.keys(PATHS);
