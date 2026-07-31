const { CaptureRing, Glyph, SelectionBrackets, Meter, ConfidenceTag, Icon, StateDot } = window.FleetDesignSystem_d68ed7;

const SYSTEMS = [
  { id: 'home',    name: 'Home Core',       glyph: 'core',    x: 9,  y: 64, tone: 'friendly', sub: 'shielded' },
  { id: 'meridian',name: 'Meridian Gate',   glyph: 'gate',    x: 24, y: 34, tone: 'friendly', sub: 'held' },
  { id: 'kestrel', name: 'Kestrel Gate',    glyph: 'gate',    x: 39, y: 68, tone: 'friendly', sub: 'held' },
  { id: 'vantage', name: 'Vantage Outpost', glyph: 'outpost', x: 65, y: 76, tone: 'neutral',  sub: 'unclaimed' },
  { id: 'thren',   name: 'Thren Gate',      glyph: 'gate',    x: 79, y: 30, tone: 'enemy',    sub: 'enemy held' },
  { id: 'opp',     name: 'Command Core',    glyph: 'core',    x: 91, y: 58, tone: 'enemy',    sub: 'exposed' },
];

function SystemNode({ s, selected, onSelect }) {
  const c = { friendly: 'var(--sig-friendly)', enemy: 'var(--sig-enemy)', neutral: 'var(--sig-neutral)' }[s.tone];
  const node = (
    <div onClick={() => onSelect(s.id)} style={{ display: 'grid', justifyItems: 'center', gap: 6, padding: 8, cursor: 'pointer' }}>
      <Glyph name={s.glyph} size={26} tone={s.tone} strokeWidth={1.25} />
      <div style={{ display: 'grid', justifyItems: 'center', gap: 2 }}>
        <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-1)', whiteSpace: 'nowrap' }}>{s.name}</span>
        <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: s.tone === 'neutral' ? 'var(--text-3)' : c, whiteSpace: 'nowrap' }}>{s.sub}</span>
      </div>
    </div>
  );
  return (
    <div style={{ position: 'absolute', left: s.x + '%', top: s.y + '%', transform: 'translate(-50%,-50%)' }}>
      {selected ? <SelectionBrackets inset={-2} size={12}>{node}</SelectionBrackets> : node}
    </div>
  );
}

function FleetMarker({ x, y, id, tone, count, certainty = 'confirmed', stance = 'aggressive' }) {
  const c = tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const est = certainty !== 'confirmed';
  return (
    <div style={{ position: 'absolute', left: x + '%', top: y + '%', transform: 'translate(-50%,-50%)', display: 'flex', alignItems: 'center', gap: 6, padding: '3px 7px', border: '1px ' + (est ? 'dashed' : 'solid') + ' ' + (est ? 'var(--sig-neutral-edge)' : c), background: 'rgba(7,10,15,.72)', borderRadius: 'var(--r-2)', whiteSpace: 'nowrap' }}>
      <Glyph name={est ? 'unknown' : stance} size={12} tone={est ? 'neutral' : tone} />
      <span style={{ font: 'var(--type-readout)', fontSize: 'var(--fs-body)', color: est ? 'var(--text-stale)' : c }}>{id}</span>
      <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>{count}</span>
    </div>
  );
}

function Route({ from, to, state }) {
  const a = SYSTEMS.find((s) => s.id === from), b = SYSTEMS.find((s) => s.id === to);
  if (!a || !b) return null;
  const committed = state === 'committed';
  const c = committed ? 'var(--sig-friendly)' : 'var(--sig-neutral)';
  const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  return (
    <>
      <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', pointerEvents: 'none', overflow: 'visible' }}>
        <defs>
          <marker id="rt-arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
            <path d="M0 0 L8 4 L0 8" fill="none" stroke={committed ? '#22CFE8' : '#98A4AE'} strokeWidth="1.4" />
          </marker>
        </defs>
        <line x1={a.x + '%'} y1={a.y + '%'} x2={b.x + '%'} y2={b.y + '%'} stroke={c} strokeWidth="1.25" strokeDasharray={committed ? undefined : '5 5'} markerEnd="url(#rt-arrow)" opacity={committed ? 1 : 0.8} />
      </svg>
      <div style={{ position: 'absolute', left: mid.x + '%', top: mid.y + '%', transform: 'translate(-50%,-50%)', display: 'flex', alignItems: 'center', gap: 5, padding: '2px 6px', background: 'rgba(7,10,15,.85)', border: '1px ' + (committed ? 'solid' : 'dashed') + ' ' + (committed ? 'var(--sig-friendly-edge)' : 'var(--sig-neutral-edge)'), borderRadius: 'var(--r-2)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: committed ? 'var(--cyan-300)' : 'var(--text-3)', whiteSpace: 'nowrap' }}>
        {committed ? <Icon name="lock" size={10} /> : <Glyph name="unknown" size={10} />}
        {committed ? 'warp locked' : 'planned'}
      </div>
    </>
  );
}

function Relay({ phase, elapsed }) {
  const cfg = {
    positioning: { value: 0, mode: 'fill', tone: 'neutral', sub: 'uncontested' },
    contested:   { value: 0.5, mode: 'contested', tone: 'enemy', sub: 'contested' },
    committed:   { value: 0.75, mode: 'fill', tone: 'friendly', sub: 'capturing' },
    decaying:    { value: 0.31, mode: 'decay', tone: 'neutral', sub: 'decaying' },
  }[phase];
  return (
    <div style={{ position: 'absolute', left: '52%', top: '44%', transform: 'translate(-50%,-50%)', display: 'grid', justifyItems: 'center', gap: 8 }}>
      <SelectionBrackets inset={-14} size={16} tone={cfg.tone === 'neutral' ? 'neutral' : cfg.tone}>
        <CaptureRing size={190} segments={8} value={cfg.value} mode={cfg.mode} tone={cfg.tone} label={elapsed.toFixed(1)} sub={cfg.sub} />
      </SelectionBrackets>
      <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-1)' }}>Shield Relay</span>
      <span style={{ font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>8s hold · drops opponent core shield</span>
    </div>
  );
}

function Playfield({ phase, selectedSystem, onSelectSystem, route, elapsed }) {
  return (
    <div style={{ position: 'absolute', inset: 0, background: 'var(--void-1)', overflow: 'hidden' }}>
      <div style={{ position: 'absolute', inset: 0, backgroundImage: 'linear-gradient(var(--line-grid) 1px, transparent 1px), linear-gradient(90deg, var(--line-grid) 1px, transparent 1px)', backgroundSize: '80px 80px' }} />
      <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(58% 70% at 52% 44%, rgba(20,30,42,.55) 0%, transparent 70%)' }} />
      {route && <Route {...route} />}
      <Relay phase={phase} elapsed={elapsed} />
      {SYSTEMS.map((s) => <SystemNode key={s.id} s={s} selected={selectedSystem === s.id} onSelect={onSelectSystem} />)}
      <FleetMarker x={33} y={54} id="04" tone="friendly" count="880" stance="aggressive" />
      <FleetMarker x={44} y={80} id="07" tone="friendly" count="510" stance="defensive" />
      <FleetMarker x={18} y={50} id="11" tone="friendly" count="1,240" stance="screen" />
      {phase !== 'positioning' && <FleetMarker x={62} y={38} id="E2" tone="enemy" count="~900" />}
      <FleetMarker x={73} y={50} id="E?" tone="enemy" count="~3 cap" certainty="estimated" />
      <div style={{ position: 'absolute', left: '86%', top: '72%', transform: 'translate(-50%,-50%)' }}>
        <ConfidenceTag level="stale">unscouted · 41s</ConfidenceTag>
      </div>
    </div>
  );
}

Object.assign(window, { Playfield, SYSTEMS });
