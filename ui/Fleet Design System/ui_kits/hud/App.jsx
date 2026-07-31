const { OrderStrip, CommandButton } = window.FleetDesignSystem_d68ed7;
const { useState, useEffect } = React;

const FLEETS = [
  { id: '04', drones: 880, capitals: 2, strength: .62, state: 'idle' },
  { id: '07', drones: 510, capitals: 1, strength: .44, state: 'moving' },
  { id: '11', drones: 1240, capitals: 3, strength: .85, state: 'engaged' },
  { id: '02', drones: 420, capitals: 0, strength: .35, state: 'idle' },
];
const PHASES = [
  ['positioning', 'Calm · positioning'],
  ['contested', 'Contested relay'],
  ['committed', 'Warp committed'],
  ['decaying', 'Relay decaying'],
];

function App() {
  const [phase, setPhase] = useState('positioning');
  const [selected, setSelected] = useState(['04']);
  const [selectedSystem, setSelectedSystem] = useState('kestrel');
  const [activeFront, setActiveFront] = useState('relay');
  const [order, setOrder] = useState(null);
  const [doctrine, setDoctrine] = useState({ stance: 'aggressive', formation: 'wedge', priority: 'capitals' });
  const [overlays, setOverlays] = useState({ scout: true, routes: true, priority: false });
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    const target = { positioning: 0, contested: 4.0, committed: 6.0, decaying: 2.5 }[phase];
    setElapsed(target);
    if (phase !== 'committed') return;
    const t = setInterval(() => setElapsed((e) => (e >= 8 ? 8 : +(e + 0.1).toFixed(1))), 120);
    return () => clearInterval(t);
  }, [phase]);

  useEffect(() => {
    if (phase === 'committed') setOrder({ from: 'meridian', to: 'kestrel', state: 'committed' });
    else if (phase === 'contested') setOrder({ from: 'meridian', to: 'kestrel', state: 'preview' });
    else setOrder(null);
  }, [phase]);

  const toggle = (id) => setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));
  const cycle = (k) => setDoctrine((d) => { const o = DOCTRINE[k]; return { ...d, [k]: o[(o.indexOf(d[k]) + 1) % o.length] }; });

  return (
    <div style={{ position: 'relative', width: 2560, height: 1097, background: 'var(--void-0)', overflow: 'hidden' }}>
      <Playfield phase={phase} selectedSystem={selectedSystem} onSelectSystem={setSelectedSystem} route={overlays.routes ? order : null} elapsed={elapsed} />
      <TopBar phase={phase} elapsed={elapsed} overlays={overlays} onToggleOverlay={(k) => setOverlays((o) => ({ ...o, [k]: !o[k] }))} />
      <SelectionPanel fleets={FLEETS} selected={selected} onToggle={toggle} doctrine={doctrine} onCycle={cycle} phase={phase} />
      <FrontRoster phase={phase} activeFront={activeFront} onSelectFront={setActiveFront} />
      <div style={{ position: 'absolute', left: '50%', transform: 'translateX(-50%)', bottom: 'var(--hud-edge)', zIndex: 3 }}>
        {order ? (
          <OrderStrip fleets={selected} destination="Kestrel Gate" state={order.state} travel="18.0s" arrival="T+04:12"
            onConfirm={order.state === 'preview' ? () => setPhase('committed') : undefined}
            onCancel={order.state === 'preview' ? () => setPhase('positioning') : undefined} />
        ) : (
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '7px 10px', border: '1px dashed var(--sig-neutral-edge)', borderRadius: 'var(--r-1)', background: 'var(--surface-panel)', backdropFilter: 'var(--blur-panel)', font: 'var(--type-label)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-3)' }}>
            no order staged · select fleets, then a destination
            <CommandButton size="sm" onClick={() => setPhase('contested')}>stage warp</CommandButton>
          </div>
        )}
      </div>
      <div style={{ position: 'absolute', left: '50%', transform: 'translateX(-50%)', top: 46, zIndex: 3, display: 'flex', gap: 5 }}>
        {PHASES.map(([p, label]) => (
          <CommandButton key={p} size="sm" variant={phase === p ? 'commit' : 'ghost'} onClick={() => setPhase(p)}>{label}</CommandButton>
        ))}
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
