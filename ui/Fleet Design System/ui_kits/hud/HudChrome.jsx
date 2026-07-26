const { HudPanel, Hairline, Readout, ConfidenceTag, FleetChip, DoctrinePill, FrontRow, OrderStrip, ObjectiveBar, IconButton, CommandButton, Glyph, StateDot } = window.FleetDesignSystem_d68ed7;

function TopBar({ phase, elapsed, overlays, onToggleOverlay }) {
  const obj = {
    positioning: { value: 0, mode: 'fill', tone: 'friendly' },
    contested: { value: 0.5, mode: 'contested', tone: 'enemy' },
    committed: { value: 0.75, mode: 'fill', tone: 'friendly' },
    decaying: { value: 0.31, mode: 'decay', tone: 'neutral' },
  }[phase];
  return (
    <ObjectiveBar
      style={{ position: 'absolute', top: 0, left: 0, right: 0, zIndex: 3 }}
      left={<>
        <span style={{ font: 'var(--type-label)', letterSpacing: '.22em', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-1)' }}>Fleet</span>
        <Hairline vertical style={{ height: 16 }} />
        <Readout label="drones" value="4,280" size="sm" />
        <Readout label="capitals" value="9" size="sm" />
        <Readout label="fronts" value="3" size="sm" />
        <Readout label="clock" value="04:12" size="sm" />
      </>}
      right={<>
        <ConfidenceTag level="estimated">~3 capitals massing</ConfidenceTag>
        <Readout label="enemy seen" value={phase === 'positioning' ? '41s ago' : '2s ago'} size="sm" stale={phase === 'positioning'} align="right" />
        <Hairline vertical style={{ height: 16 }} />
        <span style={{ display: 'flex', gap: 5 }}>
          <IconButton icon="radar" title="Scouting overlay" active={overlays.scout} onClick={() => onToggleOverlay('scout')} />
          <IconButton icon="git-branch" title="Route overlay" active={overlays.routes} onClick={() => onToggleOverlay('routes')} />
          <IconButton icon="crosshair" title="Target priority" active={overlays.priority} onClick={() => onToggleOverlay('priority')} />
        </span>
      </>}
      objective={{ name: 'Shield Relay', seconds: 8, elapsed, ...obj, cores: [
        { label: 'YOU', tone: 'friendly', shielded: true },
        { label: 'OPP', tone: 'enemy', shielded: phase === 'committed' ? false : true },
      ] }} />
  );
}

const DOCTRINE = {
  stance: ['aggressive', 'defensive', 'screen'],
  formation: ['wedge', 'dispersed', 'screen'],
  priority: ['capitals', 'drones', 'relay'],
};
const GLYPHS = { aggressive: 'aggressive', defensive: 'defensive', screen: 'screen', wedge: 'wedge', dispersed: 'dispersed', capitals: 'priorityCapital', drones: 'drone', relay: 'relay' };

function SelectionPanel({ fleets, selected, onToggle, doctrine, onCycle, phase }) {
  const sel = fleets.filter((f) => selected.includes(f.id));
  const drones = sel.reduce((n, f) => n + f.drones, 0);
  const capitals = sel.reduce((n, f) => n + f.capitals, 0);
  return (
    <HudPanel title="Front 02 · Kestrel Approach" meta={sel.length + ' fleets'} tone="friendly" brackets
      style={{ position: 'absolute', left: 'var(--hud-edge)', bottom: 'var(--hud-edge)', width: 'var(--hud-panel-w)', zIndex: 3 }}>
      <div style={{ display: 'flex', gap: 'var(--sp-6)' }}>
        <Readout label="drones" value={drones.toLocaleString()} tone="friendly" />
        <Readout label="capitals" value={capitals} tone="friendly" />
        <Readout label="strength" value={sel.length ? Math.round(sel.reduce((n, f) => n + f.strength, 0) / sel.length * 100) : 0} unit="%" />
      </div>
      <Hairline inset={-12} style={{ margin: '10px 0' }} />
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
        {Object.keys(DOCTRINE).map((k) => (
          <DoctrinePill key={k} label={k} value={doctrine[k]} glyph={GLYPHS[doctrine[k]]} active mixed={sel.length > 1 && k === 'formation'} onClick={() => onCycle(k)} />
        ))}
      </div>
      <Hairline inset={-12} style={{ margin: '10px 0' }} />
      <div style={{ display: 'grid', gap: 3 }}>
        {fleets.map((f) => (
          <FleetChip key={f.id} id={f.id} strength={f.strength} drones={f.drones.toLocaleString()} capitals={f.capitals}
            state={phase === 'committed' && selected.includes(f.id) ? 'committed' : f.state}
            selected={selected.includes(f.id)} onClick={() => onToggle(f.id)} />
        ))}
      </div>
    </HudPanel>
  );
}

function FrontRoster({ phase, activeFront, onSelectFront }) {
  const fronts = [
    { id: 'relay', name: 'Shield Relay', glyph: 'relay', urgency: phase === 'positioning' ? 'active' : 'critical', tone: phase === 'contested' ? 'enemy' : phase === 'committed' ? 'friendly' : 'neutral', status: phase === 'contested' ? 'contested' : phase === 'committed' ? 'capturing' : phase === 'decaying' ? 'decaying' : 'uncontested', progress: { positioning: 0, contested: .5, committed: .75, decaying: .31 }[phase], mode: { positioning: 'fill', contested: 'contested', committed: 'fill', decaying: 'decay' }[phase] },
    { id: 'kestrel', name: 'Kestrel Gate', glyph: 'gate', urgency: 'active', tone: 'friendly', status: 'inbound 12s', progress: .7, mode: 'fill' },
    { id: 'vantage', name: 'Vantage Outpost', glyph: 'outpost', urgency: 'quiet', tone: 'neutral', status: 'unscouted' },
    { id: 'home', name: 'Home Core', glyph: 'core', urgency: 'quiet', tone: 'friendly', status: 'quiet' },
  ];
  return (
    <HudPanel title="Fronts · by urgency" meta="4"
      style={{ position: 'absolute', right: 'var(--hud-edge)', bottom: 'var(--hud-edge)', width: 'var(--hud-panel-w)', zIndex: 3 }}>
      <div style={{ margin: '0 -4px' }}>
        {fronts.map((f) => (
          <FrontRow key={f.id} name={f.name} status={f.status} urgency={f.urgency} tone={f.tone} glyph={f.glyph}
            progress={f.progress} progressMode={f.mode} selected={activeFront === f.id} onClick={() => onSelectFront(f.id)} />
        ))}
      </div>
      <div style={{ position: 'relative', height: 70, marginTop: 10, border: '1px solid var(--line-hairline)', background: 'rgba(255,255,255,.015)' }}>
        <svg width="100%" height="100%" style={{ display: 'block' }}>
          <line x1="14%" y1="66%" x2="34%" y2="36%" stroke="var(--line-hairline-strong)" strokeWidth="1" />
          <line x1="34%" y1="36%" x2="52%" y2="52%" stroke="var(--sig-friendly)" strokeWidth="1" />
          <line x1="52%" y1="52%" x2="72%" y2="76%" stroke="var(--line-hairline-strong)" strokeWidth="1" strokeDasharray="4 4" />
          <line x1="52%" y1="52%" x2="80%" y2="28%" stroke="var(--sig-enemy)" strokeWidth="1" opacity=".7" />
          <line x1="80%" y1="28%" x2="92%" y2="58%" stroke="var(--sig-enemy)" strokeWidth="1" opacity=".7" />
          {[['14%','66%','var(--sig-friendly)'],['34%','36%','var(--sig-friendly)'],['52%','52%','var(--sig-neutral)'],['72%','76%','var(--sig-neutral)'],['80%','28%','var(--sig-enemy)'],['92%','58%','var(--sig-enemy)']].map(([x,y,c],i)=>(
            <circle key={i} cx={x} cy={y} r="2.5" fill={c} />
          ))}
        </svg>
      </div>
    </HudPanel>
  );
}

Object.assign(window, { TopBar, SelectionPanel, FrontRoster, DOCTRINE, GLYPHS });
