/* @ds-bundle: {"format":4,"namespace":"FleetDesignSystem_d68ed7","components":[{"name":"CommandButton","sourcePath":"components/controls/CommandButton.jsx"},{"name":"IconButton","sourcePath":"components/controls/IconButton.jsx"},{"name":"ObjectiveBar","sourcePath":"components/controls/ObjectiveBar.jsx"},{"name":"DoctrinePill","sourcePath":"components/fleet/DoctrinePill.jsx"},{"name":"FleetChip","sourcePath":"components/fleet/FleetChip.jsx"},{"name":"FrontRow","sourcePath":"components/fleet/FrontRow.jsx"},{"name":"OrderStrip","sourcePath":"components/fleet/OrderStrip.jsx"},{"name":"Icon","sourcePath":"components/icons/Icon.jsx"},{"name":"CaptureRing","sourcePath":"components/signals/CaptureRing.jsx"},{"name":"ConfidenceTag","sourcePath":"components/signals/ConfidenceTag.jsx"},{"name":"Glyph","sourcePath":"components/signals/Glyph.jsx"},{"name":"Meter","sourcePath":"components/signals/Meter.jsx"},{"name":"Readout","sourcePath":"components/signals/Readout.jsx"},{"name":"StateDot","sourcePath":"components/signals/StateDot.jsx"},{"name":"Hairline","sourcePath":"components/surfaces/Hairline.jsx"},{"name":"HudPanel","sourcePath":"components/surfaces/HudPanel.jsx"},{"name":"SelectionBrackets","sourcePath":"components/surfaces/SelectionBrackets.jsx"}],"sourceHashes":{"components/controls/CommandButton.jsx":"377392dd3b0d","components/controls/IconButton.jsx":"0449fa74306f","components/controls/ObjectiveBar.jsx":"e6e2e58eba01","components/fleet/DoctrinePill.jsx":"a19f96f56fa7","components/fleet/FleetChip.jsx":"21d78605719f","components/fleet/FrontRow.jsx":"1f5dac8f4518","components/fleet/OrderStrip.jsx":"c54886c127fd","components/icons/Icon.jsx":"536217966c4b","components/signals/CaptureRing.jsx":"4f76938ebe94","components/signals/ConfidenceTag.jsx":"4fe77e6adf65","components/signals/Glyph.jsx":"5bddcf37e2b2","components/signals/Meter.jsx":"06eac195fef6","components/signals/Readout.jsx":"cc0eee0ef0c4","components/signals/StateDot.jsx":"14e4892560fc","components/surfaces/Hairline.jsx":"c51f051ed6b3","components/surfaces/HudPanel.jsx":"d2d272680e83","components/surfaces/SelectionBrackets.jsx":"21b4b8ee1306","ui_kits/hud/App.jsx":"837ae2d513c5","ui_kits/hud/HudChrome.jsx":"633a532da488","ui_kits/hud/Playfield.jsx":"a79dada3eb0e"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.FleetDesignSystem_d68ed7 = window.FleetDesignSystem_d68ed7 || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/icons/Icon.jsx
try { (() => {
/* Lucide v0.544.0 (ISC), inlined. Mirrors of these files live in assets/icons/.
   Inlined rather than linked so icons colour from currentColor and never depend on a CDN. */
const PATHS = {
  'lock': '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  'unlock': '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>',
  'shield': '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>',
  'shield-off': '<path d="m2 2 20 20"/><path d="M5 5a1 1 0 0 0-1 1v7c0 5 3.5 7.5 7.67 8.94a1 1 0 0 0 .67.01c2.35-.82 4.48-1.97 5.9-3.71"/><path d="M9.309 3.652A12.252 12.252 0 0 0 11.24 2.28a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1v7a9.784 9.784 0 0 1-.08 1.264"/>',
  'radar': '<path d="M19.07 4.93A10 10 0 0 0 6.99 3.34"/><path d="M4 6h.01"/><path d="M2.29 9.62A10 10 0 1 0 21.31 8.35"/><path d="M16.24 7.76A6 6 0 1 0 8.23 16.67"/><path d="M12 18h.01"/><path d="M17.99 11.66A6 6 0 0 1 15.77 16.67"/><circle cx="12" cy="12" r="2"/><path d="m13.41 10.59 5.66-5.66"/>',
  'crosshair': '<circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/>',
  'chevron-right': '<path d="m9 18 6-6-6-6"/>',
  'x': '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  'triangle-alert': '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  'eye-off': '<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>',
  'git-branch': '<line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>',
  'activity': '<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/>',
  'clock': '<path d="M12 6v6l4 2"/><circle cx="12" cy="12" r="10"/>',
  'circle-dot': '<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/>'
};

/** Interface icon (Lucide, inlined). Paints in currentColor. */
function Icon({
  name,
  size = 14,
  strokeColor,
  strokeWidth = 2,
  style,
  title
}) {
  const inner = PATHS[name];
  if (!inner) return null;
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: strokeColor || 'currentColor',
    strokeWidth: strokeWidth,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    role: title ? 'img' : 'presentation',
    "aria-label": title,
    style: {
      display: 'block',
      flex: 'none',
      ...style
    },
    dangerouslySetInnerHTML: {
      __html: inner
    }
  });
}
Icon.names = Object.keys(PATHS);
Object.assign(__ds_scope, { Icon });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/icons/Icon.jsx", error: String((e && e.message) || e) }); }

// components/controls/CommandButton.jsx
try { (() => {
/** Rectangular command control. commit = the irreversible action, ghost = everything else. */
function CommandButton({
  children,
  variant = 'ghost',
  size = 'md',
  icon,
  disabled = false,
  onClick,
  style
}) {
  const v = {
    commit: {
      bg: 'var(--sig-friendly-wash)',
      border: 'var(--sig-friendly-edge)',
      color: 'var(--cyan-300)'
    },
    danger: {
      bg: 'var(--sig-enemy-wash)',
      border: 'var(--sig-enemy-edge)',
      color: 'var(--coral-300)'
    },
    ghost: {
      bg: 'transparent',
      border: 'var(--line-hairline)',
      color: 'var(--text-2)'
    }
  }[variant];
  const pad = size === 'sm' ? '3px 8px' : '6px 12px';
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    disabled: disabled,
    onClick: onClick,
    style: {
      all: 'unset',
      boxSizing: 'border-box',
      cursor: disabled ? 'not-allowed' : 'pointer',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      padding: pad,
      background: v.bg,
      border: '1px solid ' + v.border,
      borderRadius: 'var(--r-2)',
      font: 'var(--type-label)',
      fontSize: size === 'sm' ? 'var(--fs-micro)' : 'var(--fs-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: v.color,
      opacity: disabled ? .38 : 1,
      transition: 'background var(--dur-instant) var(--ease-out), color var(--dur-instant) var(--ease-out)',
      ...style
    }
  }, icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: size === 'sm' ? 10 : 12
  }), children);
}
Object.assign(__ds_scope, { CommandButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/CommandButton.jsx", error: String((e && e.message) || e) }); }

// components/controls/IconButton.jsx
try { (() => {
/** Square hairline icon control for HUD affordances. */
function IconButton({
  icon,
  active = false,
  size = 22,
  title,
  onClick,
  style
}) {
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    title: title,
    "aria-label": title,
    onClick: onClick,
    style: {
      all: 'unset',
      boxSizing: 'border-box',
      cursor: 'pointer',
      display: 'grid',
      placeItems: 'center',
      width: size,
      height: size,
      border: '1px solid ' + (active ? 'var(--sig-friendly-edge)' : 'var(--line-hairline)'),
      borderRadius: 'var(--r-1)',
      background: active ? 'var(--sig-friendly-wash)' : 'transparent',
      color: active ? 'var(--cyan-300)' : 'var(--text-2)',
      transition: 'all var(--dur-instant) var(--ease-out)',
      ...style
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: Math.round(size * 0.55)
  }));
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/signals/CaptureRing.jsx
try { (() => {
/** Large segmented circular capture ring drawn directly on an objective in the world. */
function CaptureRing({
  value = 0,
  segments = 8,
  tone = 'friendly',
  mode = 'fill',
  size = 132,
  label,
  sub,
  style
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone];
  const r = size / 2 - 8,
    cx = size / 2,
    gap = 3.2,
    step = 360 / segments;
  const filled = Math.max(0, Math.min(1, value)) * segments;
  const arcs = Array.from({
    length: segments
  }, (_, i) => {
    const a0 = -90 + i * step + gap / 2,
      a1 = -90 + (i + 1) * step - gap / 2;
    const p = a => [cx + r * Math.cos(a * Math.PI / 180), cx + r * Math.sin(a * Math.PI / 180)];
    const [x0, y0] = p(a0),
      [x1, y1] = p(a1);
    const on = i < Math.floor(filled),
      partial = i === Math.floor(filled) && filled % 1 > 0.05;
    return {
      d: `M ${x0} ${y0} A ${r} ${r} 0 0 1 ${x1} ${y1}`,
      on,
      partial
    };
  });
  const stroke = mode === 'decay' ? 'var(--sig-neutral)' : c;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: size,
      height: size,
      ...style
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    style: {
      display: 'block',
      overflow: 'visible'
    }
  }, mode === 'contested' && /*#__PURE__*/React.createElement("defs", null, /*#__PURE__*/React.createElement("pattern", {
    id: 'cr-stripe-' + tone,
    width: "6",
    height: "6",
    patternTransform: "rotate(-45)",
    patternUnits: "userSpaceOnUse"
  }, /*#__PURE__*/React.createElement("rect", {
    width: "3",
    height: "6",
    fill: c
  }))), arcs.map((a, i) => /*#__PURE__*/React.createElement("path", {
    key: i,
    d: a.d,
    fill: "none",
    strokeLinecap: "butt",
    strokeWidth: a.on || a.partial ? 6 : 2,
    stroke: a.on ? mode === 'contested' ? `url(#cr-stripe-${tone})` : stroke : a.partial ? stroke : 'rgba(160,178,196,.20)',
    strokeDasharray: a.partial ? '3 3' : undefined,
    style: {
      opacity: mode === 'decay' && a.on ? 0.7 : 1,
      transition: 'stroke-width var(--dur-fast) var(--ease-out)'
    }
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      display: 'grid',
      placeContent: 'center',
      textAlign: 'center',
      gap: 2
    }
  }, label && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--type-hero)',
      color: mode === 'decay' ? 'var(--sig-neutral)' : c,
      letterSpacing: 'var(--tracking-num)',
      fontVariantNumeric: 'tabular-nums'
    }
  }, label), sub && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, sub)));
}
Object.assign(__ds_scope, { CaptureRing });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/CaptureRing.jsx", error: String((e && e.message) || e) }); }

// components/signals/ConfidenceTag.jsx
try { (() => {
/** Marks how trustworthy a piece of information is. Dashed outline = not confirmed. */
function ConfidenceTag({
  level = 'confirmed',
  children,
  tone = 'neutral',
  style
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone];
  const solid = level === 'confirmed';
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      padding: 'var(--pad-chip)',
      border: (solid ? '1px solid ' : '1px dashed ') + (solid ? c : 'var(--sig-neutral-edge)'),
      borderRadius: 'var(--r-2)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: solid ? c : 'var(--text-3)',
      opacity: level === 'stale' ? 0.75 : 1,
      ...style
    }
  }, children || level);
}
Object.assign(__ds_scope, { ConfidenceTag });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/ConfidenceTag.jsx", error: String((e && e.message) || e) }); }

// components/signals/Glyph.jsx
try { (() => {
/** Small geometric tactical glyph: system type, fleet composition, stance, formation, target priority. */
const PATHS = {
  core: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("rect", {
    x: s * .18,
    y: s * .18,
    width: s * .64,
    height: s * .64
  }), /*#__PURE__*/React.createElement("rect", {
    x: s * .38,
    y: s * .38,
    width: s * .24,
    height: s * .24,
    fill: "currentColor",
    stroke: "none"
  })),
  relay: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: s / 2,
    cy: s / 2,
    r: s * .32
  }), /*#__PURE__*/React.createElement("path", {
    d: `M${s / 2} ${s * .06} V${s * .22} M${s / 2} ${s * .78} V${s * .94} M${s * .06} ${s / 2} H${s * .22} M${s * .78} ${s / 2} H${s * .94}`
  })),
  gate: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .5} ${s * .1} L${s * .9} ${s * .5} L${s * .5} ${s * .9} L${s * .1} ${s * .5} Z`
  })),
  outpost: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .5} ${s * .12} L${s * .88} ${s * .86} L${s * .12} ${s * .86} Z`
  })),
  drone: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: s * .28,
    cy: s * .5,
    r: s * .1,
    fill: "currentColor",
    stroke: "none"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: s * .56,
    cy: s * .32,
    r: s * .1,
    fill: "currentColor",
    stroke: "none"
  }), /*#__PURE__*/React.createElement("circle", {
    cx: s * .56,
    cy: s * .68,
    r: s * .1,
    fill: "currentColor",
    stroke: "none"
  })),
  capital: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .12} ${s * .5} L${s * .5} ${s * .2} L${s * .88} ${s * .5} L${s * .5} ${s * .8} Z`,
    fill: "currentColor",
    stroke: "none"
  })),
  aggressive: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .15} ${s * .75} L${s * .85} ${s * .25} M${s * .85} ${s * .25} H${s * .55} M${s * .85} ${s * .25} V${s * .55}`
  })),
  defensive: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .5} ${s * .12} L${s * .85} ${s * .28} V${s * .55} C${s * .85} ${s * .75} ${s * .68} ${s * .85} ${s * .5} ${s * .9} C${s * .32} ${s * .85} ${s * .15} ${s * .75} ${s * .15} ${s * .55} V${s * .28} Z`
  })),
  screen: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .12} ${s * .3} H${s * .88} M${s * .12} ${s * .7} H${s * .88}`
  })),
  wedge: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .5} ${s * .18} L${s * .86} ${s * .8} M${s * .5} ${s * .18} L${s * .14} ${s * .8}`
  })),
  dispersed: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: s * .25,
    cy: s * .3,
    r: s * .07
  }), /*#__PURE__*/React.createElement("circle", {
    cx: s * .72,
    cy: s * .36,
    r: s * .07
  }), /*#__PURE__*/React.createElement("circle", {
    cx: s * .4,
    cy: s * .72,
    r: s * .07
  }), /*#__PURE__*/React.createElement("circle", {
    cx: s * .78,
    cy: s * .74,
    r: s * .07
  })),
  priorityCapital: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: s / 2,
    cy: s / 2,
    r: s * .34
  }), /*#__PURE__*/React.createElement("path", {
    d: `M${s * .5} ${s * .16} V${s * .84} M${s * .16} ${s * .5} H${s * .84}`
  })),
  warp: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("path", {
    d: `M${s * .14} ${s * .5} H${s * .8} M${s * .62} ${s * .3} L${s * .86} ${s * .5} L${s * .62} ${s * .7}`
  })),
  unknown: s => /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("circle", {
    cx: s / 2,
    cy: s / 2,
    r: s * .32,
    strokeDasharray: "2 3"
  }))
};
function Glyph({
  name = 'unknown',
  size = 14,
  tone = 'inherit',
  strokeWidth = 1.25,
  style,
  title
}) {
  const col = {
    inherit: 'currentColor',
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone] || 'currentColor';
  const draw = PATHS[name] || PATHS.unknown;
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: `0 0 ${size} ${size}`,
    style: {
      display: 'block',
      color: col,
      flex: 'none',
      ...style
    },
    fill: "none",
    stroke: "currentColor",
    strokeWidth: strokeWidth,
    strokeLinecap: "square",
    strokeLinejoin: "miter",
    "aria-label": title,
    role: title ? 'img' : 'presentation'
  }, draw(size));
}
Glyph.names = Object.keys(PATHS);
Object.assign(__ds_scope, { Glyph });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/Glyph.jsx", error: String((e && e.message) || e) }); }

// components/fleet/DoctrinePill.jsx
try { (() => {
/** Editable doctrine setting: stance, formation, or target priority. */
function DoctrinePill({
  label,
  value,
  glyph,
  active = false,
  mixed = false,
  onClick,
  style
}) {
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClick,
    title: label,
    style: {
      all: 'unset',
      cursor: 'pointer',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      padding: 'var(--pad-chip)',
      background: active ? 'var(--sig-friendly-wash)' : 'transparent',
      border: '1px ' + (mixed ? 'dashed' : 'solid') + ' ' + (active ? 'var(--sig-friendly-edge)' : 'var(--line-hairline)'),
      borderRadius: 'var(--r-2)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: mixed ? 'var(--text-3)' : active ? 'var(--cyan-300)' : 'var(--text-2)',
      transition: 'all var(--dur-instant) var(--ease-out)',
      ...style
    }
  }, glyph && /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: glyph,
    size: 11
  }), mixed ? 'mixed' : value);
}
Object.assign(__ds_scope, { DoctrinePill });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/fleet/DoctrinePill.jsx", error: String((e && e.message) || e) }); }

// components/fleet/OrderStrip.jsx
try { (() => {
/** Bottom-centre order confirmation strip — the player's proof an irreversible order landed. */
function OrderStrip({
  fleets = [],
  destination,
  state = 'preview',
  travel,
  arrival,
  onConfirm,
  onCancel,
  style
}) {
  const committed = state === 'committed';
  const c = committed ? 'var(--sig-friendly)' : 'var(--sig-neutral)';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 'var(--sp-3)',
      padding: '7px 10px',
      background: 'var(--surface-panel)',
      border: '1px ' + (committed ? 'solid' : 'dashed') + ' ' + (committed ? 'var(--sig-friendly-edge)' : 'var(--sig-neutral-edge)'),
      borderRadius: 'var(--r-1)',
      backdropFilter: 'var(--blur-panel)',
      boxShadow: 'var(--shadow-panel)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: c
    }
  }, committed ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "lock",
    size: 11
  }) : /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "unknown",
    size: 11
  }), committed ? 'warp committed' : 'preview order'), /*#__PURE__*/React.createElement("i", {
    style: {
      width: 1,
      height: 18,
      background: 'var(--line-hairline)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6
    }
  }, fleets.map(f => /*#__PURE__*/React.createElement("em", {
    key: f,
    style: {
      font: 'var(--type-readout)',
      fontStyle: 'normal',
      fontSize: 'var(--fs-body)',
      color: 'var(--text-1)'
    }
  }, f)), /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "warp",
    size: 14,
    tone: committed ? 'friendly' : 'neutral'
  }), /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-readout)',
      fontStyle: 'normal',
      fontSize: 'var(--fs-body)',
      color: committed ? 'var(--cyan-300)' : 'var(--text-2)'
    }
  }, destination)), /*#__PURE__*/React.createElement("i", {
    style: {
      width: 1,
      height: 18,
      background: 'var(--line-hairline)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'baseline',
      gap: 10,
      font: 'var(--type-readout)',
      fontSize: 'var(--fs-body)',
      color: 'var(--text-2)',
      fontVariantNumeric: 'tabular-nums'
    }
  }, /*#__PURE__*/React.createElement("span", null, travel, /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-label)',
      fontStyle: 'normal',
      color: 'var(--text-3)',
      marginLeft: 3
    }
  }, "travel")), /*#__PURE__*/React.createElement("span", {
    style: {
      color: committed ? 'var(--text-1)' : 'var(--text-3)'
    }
  }, arrival, /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-label)',
      fontStyle: 'normal',
      color: 'var(--text-3)',
      marginLeft: 3
    }
  }, "arrival"))), !committed && (onConfirm || onCancel) && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      gap: 6,
      marginLeft: 2
    }
  }, onCancel && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onCancel,
    style: {
      all: 'unset',
      cursor: 'pointer',
      padding: 'var(--pad-chip)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, "cancel"), onConfirm && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onConfirm,
    style: {
      all: 'unset',
      cursor: 'pointer',
      padding: 'var(--pad-chip)',
      border: '1px solid var(--sig-friendly-edge)',
      borderRadius: 'var(--r-2)',
      background: 'var(--sig-friendly-wash)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--cyan-300)'
    }
  }, "commit warp")));
}
Object.assign(__ds_scope, { OrderStrip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/fleet/OrderStrip.jsx", error: String((e && e.message) || e) }); }

// components/signals/Meter.jsx
try { (() => {
/** Linear meter. mode carries the meaning: fill / contested (stripes) / decay (reverse-draining gray). */
function Meter({
  value = 0,
  tone = 'friendly',
  mode = 'fill',
  height = 3,
  width,
  showTrack = true,
  style
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone];
  const stripe = tone === 'enemy' ? 'var(--stripe-contested-enemy)' : 'var(--stripe-contested-friendly)';
  const fill = mode === 'contested' ? stripe : mode === 'decay' ? 'var(--stripe-decay)' : c;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: width || '100%',
      height,
      background: showTrack ? 'rgba(255,255,255,.07)' : 'transparent',
      borderRadius: 'var(--r-full)',
      overflow: 'hidden',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      right: 'auto',
      width: Math.max(0, Math.min(1, value)) * 100 + '%',
      background: fill,
      transition: 'width var(--dur-slow) var(--ease-linear)'
    }
  }));
}
Object.assign(__ds_scope, { Meter });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/Meter.jsx", error: String((e && e.message) || e) }); }

// components/controls/ObjectiveBar.jsx
try { (() => {
/** Extremely slim top bar: friendly state left, decisive objective centre, scouting confidence right. */
function ObjectiveBar({
  left,
  right,
  objective = {},
  style
}) {
  const {
    name = 'Shield Relay',
    value = 0,
    seconds = 8,
    elapsed = 0,
    mode = 'fill',
    tone = 'friendly',
    cores = []
  } = objective;
  const c = mode === 'decay' ? 'var(--sig-neutral)' : tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const stateWord = mode === 'contested' ? 'contested' : mode === 'decay' ? 'decaying' : tone === 'enemy' ? 'enemy capturing' : 'capturing';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr auto 1fr',
      alignItems: 'center',
      gap: 'var(--sp-4)',
      height: 'var(--hud-topbar-h)',
      padding: '0 var(--sp-3)',
      background: 'var(--surface-panel)',
      borderBottom: '1px solid var(--line-hairline)',
      backdropFilter: 'var(--blur-panel)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 'var(--sp-4)',
      minWidth: 0
    }
  }, left), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 'var(--sp-3)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "relay",
    size: 13,
    tone: mode === 'decay' ? 'neutral' : tone
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-2)',
      whiteSpace: 'nowrap'
    }
  }, name), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'grid',
      gap: 3,
      width: 190
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Meter, {
    value: value,
    tone: tone,
    mode: mode,
    height: 3
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: c
    }
  }, /*#__PURE__*/React.createElement("em", {
    style: {
      fontStyle: 'normal'
    }
  }, stateWord), /*#__PURE__*/React.createElement("em", {
    style: {
      fontStyle: 'normal',
      fontFamily: 'var(--font-mono)',
      fontVariantNumeric: 'tabular-nums',
      color: 'var(--text-2)'
    }
  }, elapsed.toFixed(1), " / ", seconds, "s"))), cores.map(core => /*#__PURE__*/React.createElement("span", {
    key: core.label,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      padding: 'var(--pad-chip)',
      border: '1px solid ' + (core.tone === 'enemy' ? 'var(--sig-enemy-edge)' : 'var(--sig-friendly-edge)'),
      borderRadius: 'var(--r-2)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: core.tone === 'enemy' ? 'var(--coral-300)' : 'var(--cyan-300)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: core.shielded ? 'shield' : 'shield-off',
    size: 10
  }), core.label, /*#__PURE__*/React.createElement("em", {
    style: {
      fontStyle: 'normal',
      color: core.shielded ? 'var(--text-3)' : core.tone === 'enemy' ? 'var(--coral-300)' : 'var(--cyan-300)'
    }
  }, core.shielded ? 'shielded' : 'exposed')))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'flex-end',
      gap: 'var(--sp-4)',
      minWidth: 0
    }
  }, right));
}
Object.assign(__ds_scope, { ObjectiveBar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/ObjectiveBar.jsx", error: String((e && e.message) || e) }); }

// components/fleet/FleetChip.jsx
try { (() => {
/** Compact fleet row: number, composition glyphs, strength bar, commitment state. */
function FleetChip({
  id,
  strength = 1,
  state = 'idle',
  drones,
  capitals,
  tone = 'friendly',
  selected = false,
  onClick,
  style
}) {
  const c = tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const stateLabel = {
    idle: 'holding',
    moving: 'en route',
    committed: 'committed',
    engaged: 'engaged',
    planned: 'planned'
  }[state] || state;
  const planned = state === 'planned';
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClick,
    style: {
      all: 'unset',
      cursor: 'pointer',
      display: 'grid',
      gridTemplateColumns: '34px 1fr auto',
      alignItems: 'center',
      gap: 'var(--sp-2)',
      padding: 'var(--pad-row)',
      background: selected ? 'var(--surface-row-active)' : 'var(--surface-row)',
      border: '1px solid ' + (selected ? 'var(--sig-friendly-edge)' : 'transparent'),
      borderRadius: 'var(--r-1)',
      transition: 'background var(--dur-instant) var(--ease-out)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-readout)',
      color: c,
      letterSpacing: 'var(--tracking-num)'
    }
  }, id), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'grid',
      gap: 4
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      color: 'var(--text-3)'
    }
  }, capitals ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "capital",
    size: 10
  }), /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-label)',
      fontStyle: 'normal'
    }
  }, capitals)) : null, drones ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "drone",
    size: 10
  }), /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-label)',
      fontStyle: 'normal'
    }
  }, drones)) : null), /*#__PURE__*/React.createElement(__ds_scope.Meter, {
    value: strength,
    tone: tone,
    mode: planned ? 'contested' : 'fill',
    height: 2
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: state === 'committed' ? c : 'var(--text-3)'
    }
  }, state === 'committed' && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "lock",
    size: 9
  }), planned && /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: "unknown",
    size: 9
  }), stateLabel));
}
Object.assign(__ds_scope, { FleetChip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/fleet/FleetChip.jsx", error: String((e && e.message) || e) }); }

// components/signals/Readout.jsx
try { (() => {
/** Label + tabular value + unit. The atom of every Fleet panel. */
function Readout({
  label,
  value,
  unit,
  tone = 'default',
  size = 'md',
  align = 'left',
  stale = false,
  style
}) {
  const col = {
    default: 'var(--text-1)',
    friendly: 'var(--text-friendly)',
    enemy: 'var(--text-enemy)',
    neutral: 'var(--text-2)'
  }[tone];
  const fs = {
    sm: 'var(--fs-body)',
    md: 'var(--fs-readout)',
    lg: 'var(--fs-hero)'
  }[size];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gap: 2,
      justifyItems: align === 'right' ? 'end' : 'start',
      ...style
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-readout)',
      fontSize: fs,
      color: stale ? 'var(--text-stale)' : col,
      letterSpacing: 'var(--tracking-num)',
      fontVariantNumeric: 'tabular-nums',
      display: 'inline-flex',
      alignItems: 'baseline',
      gap: 3
    }
  }, value, unit && /*#__PURE__*/React.createElement("em", {
    style: {
      font: 'var(--type-label)',
      fontStyle: 'normal',
      color: 'var(--text-3)'
    }
  }, unit)));
}
Object.assign(__ds_scope, { Readout });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/Readout.jsx", error: String((e && e.message) || e) }); }

// components/signals/StateDot.jsx
try { (() => {
/** 6px allegiance dot. Hollow = unconfirmed, dashed = estimated. */
function StateDot({
  tone = 'neutral',
  certainty = 'confirmed',
  size = 6,
  style
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone];
  const border = certainty === 'estimated' ? '1px dashed ' + c : '1px solid ' + c;
  return /*#__PURE__*/React.createElement("i", {
    style: {
      display: 'inline-block',
      width: size,
      height: size,
      borderRadius: 'var(--r-full)',
      border,
      background: certainty === 'confirmed' ? c : 'transparent',
      opacity: certainty === 'stale' ? 0.5 : 1,
      ...style
    }
  });
}
Object.assign(__ds_scope, { StateDot });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/signals/StateDot.jsx", error: String((e && e.message) || e) }); }

// components/fleet/FrontRow.jsx
try { (() => {
/** One entry in the front roster, ordered by urgency. */
function FrontRow({
  name,
  status,
  urgency = 'quiet',
  tone = 'neutral',
  glyph = 'gate',
  progress,
  progressMode = 'fill',
  selected = false,
  onClick,
  style
}) {
  const urg = {
    critical: 'var(--sig-enemy)',
    active: 'var(--sig-friendly)',
    quiet: 'var(--sig-neutral)'
  }[urgency];
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClick,
    style: {
      all: 'unset',
      cursor: 'pointer',
      display: 'grid',
      gridTemplateColumns: '3px 14px 1fr auto',
      alignItems: 'center',
      gap: 'var(--sp-2)',
      padding: 'var(--pad-row)',
      background: selected ? 'var(--surface-row-active)' : 'transparent',
      borderBottom: '1px solid var(--line-hairline)',
      transition: 'background var(--dur-instant) var(--ease-out)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("i", {
    style: {
      width: 2,
      height: 18,
      background: urg,
      opacity: urgency === 'quiet' ? .45 : 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.Glyph, {
    name: glyph,
    size: 12,
    tone: tone === 'neutral' ? 'neutral' : tone
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'grid',
      gap: 3,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-body)',
      fontFamily: 'var(--font-condensed)',
      fontWeight: 'var(--fw-semi)',
      letterSpacing: 'var(--tracking-wide)',
      color: 'var(--text-1)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, name), progress != null && /*#__PURE__*/React.createElement(__ds_scope.Meter, {
    value: progress,
    tone: tone === 'neutral' ? 'neutral' : tone,
    mode: progressMode,
    height: 2
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: urgency === 'quiet' ? 'var(--text-3)' : urg,
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.StateDot, {
    tone: tone,
    certainty: urgency === 'quiet' ? 'stale' : 'confirmed',
    size: 5
  }), status));
}
Object.assign(__ds_scope, { FrontRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/fleet/FrontRow.jsx", error: String((e && e.message) || e) }); }

// components/surfaces/Hairline.jsx
try { (() => {
/** 1px hairline rule used to split panel sections. */
function Hairline({
  vertical = false,
  inset = 0,
  strong = false,
  style
}) {
  const c = strong ? 'var(--line-hairline-strong)' : 'var(--line-hairline)';
  return /*#__PURE__*/React.createElement("hr", {
    style: {
      border: 0,
      margin: 0,
      alignSelf: 'stretch',
      ...(vertical ? {
        width: 1,
        background: c,
        marginBlock: inset
      } : {
        height: 1,
        background: c,
        marginInline: inset
      }),
      ...style
    }
  });
}
Object.assign(__ds_scope, { Hairline });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/surfaces/Hairline.jsx", error: String((e && e.message) || e) }); }

// components/surfaces/HudPanel.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/** Translucent graphite panel with hairline outline — the only chrome container in Fleet. */
function HudPanel({
  title,
  meta,
  tone = 'neutral',
  brackets = false,
  dense = false,
  style,
  children,
  ...rest
}) {
  const edge = {
    neutral: 'var(--line-hairline)',
    friendly: 'var(--sig-friendly-edge)',
    enemy: 'var(--sig-enemy-edge)'
  }[tone];
  return /*#__PURE__*/React.createElement("section", _extends({}, rest, {
    style: {
      position: 'relative',
      background: 'var(--surface-panel)',
      border: '1px solid ' + edge,
      borderRadius: 'var(--r-1)',
      backdropFilter: 'var(--blur-panel)',
      boxShadow: 'var(--shadow-panel)',
      padding: dense ? 'var(--pad-panel-tight)' : 'var(--pad-panel)',
      ...style
    }
  }), (title || meta) && /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      justifyContent: 'space-between',
      gap: 'var(--sp-2)',
      marginBottom: dense ? 'var(--sp-2)' : 'var(--sp-3)'
    }
  }, title && /*#__PURE__*/React.createElement("h2", {
    style: {
      margin: 0,
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-2)'
    }
  }, title), meta && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, meta)), children, brackets && /*#__PURE__*/React.createElement(Brackets, {
    color: edge
  }));
}
function Brackets({
  color
}) {
  const base = {
    position: 'absolute',
    width: 'var(--bracket-len)',
    height: 'var(--bracket-len)',
    pointerEvents: 'none'
  };
  const c = 'var(--line-bracket)';
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      top: -1,
      left: -1,
      borderTop: '1px solid ' + c,
      borderLeft: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      top: -1,
      right: -1,
      borderTop: '1px solid ' + c,
      borderRight: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      bottom: -1,
      left: -1,
      borderBottom: '1px solid ' + c,
      borderLeft: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      bottom: -1,
      right: -1,
      borderBottom: '1px solid ' + c,
      borderRight: '1px solid ' + c
    }
  }));
}
Object.assign(__ds_scope, { HudPanel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/surfaces/HudPanel.jsx", error: String((e && e.message) || e) }); }

// components/surfaces/SelectionBrackets.jsx
try { (() => {
/** Thin corner brackets: the universal Fleet mark for "this is selected". */
function SelectionBrackets({
  tone = 'friendly',
  size = 10,
  inset = 0,
  children,
  style
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[tone];
  const base = {
    position: 'absolute',
    width: size,
    height: size,
    pointerEvents: 'none'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      display: 'inline-block',
      ...style
    }
  }, children, /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      top: inset,
      left: inset,
      borderTop: '1px solid ' + c,
      borderLeft: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      top: inset,
      right: inset,
      borderTop: '1px solid ' + c,
      borderRight: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      bottom: inset,
      left: inset,
      borderBottom: '1px solid ' + c,
      borderLeft: '1px solid ' + c
    }
  }), /*#__PURE__*/React.createElement("i", {
    style: {
      ...base,
      bottom: inset,
      right: inset,
      borderBottom: '1px solid ' + c,
      borderRight: '1px solid ' + c
    }
  }));
}
Object.assign(__ds_scope, { SelectionBrackets });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/surfaces/SelectionBrackets.jsx", error: String((e && e.message) || e) }); }

// ui_kits/hud/App.jsx
try { (() => {
const {
  OrderStrip,
  CommandButton
} = window.FleetDesignSystem_d68ed7;
const {
  useState,
  useEffect
} = React;
const FLEETS = [{
  id: '04',
  drones: 880,
  capitals: 2,
  strength: .62,
  state: 'idle'
}, {
  id: '07',
  drones: 510,
  capitals: 1,
  strength: .44,
  state: 'moving'
}, {
  id: '11',
  drones: 1240,
  capitals: 3,
  strength: .85,
  state: 'engaged'
}, {
  id: '02',
  drones: 420,
  capitals: 0,
  strength: .35,
  state: 'idle'
}];
const PHASES = [['positioning', 'Calm · positioning'], ['contested', 'Contested relay'], ['committed', 'Warp committed'], ['decaying', 'Relay decaying']];
function App() {
  const [phase, setPhase] = useState('positioning');
  const [selected, setSelected] = useState(['04']);
  const [selectedSystem, setSelectedSystem] = useState('kestrel');
  const [activeFront, setActiveFront] = useState('relay');
  const [order, setOrder] = useState(null);
  const [doctrine, setDoctrine] = useState({
    stance: 'aggressive',
    formation: 'wedge',
    priority: 'capitals'
  });
  const [overlays, setOverlays] = useState({
    scout: true,
    routes: true,
    priority: false
  });
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    const target = {
      positioning: 0,
      contested: 4.0,
      committed: 6.0,
      decaying: 2.5
    }[phase];
    setElapsed(target);
    if (phase !== 'committed') return;
    const t = setInterval(() => setElapsed(e => e >= 8 ? 8 : +(e + 0.1).toFixed(1)), 120);
    return () => clearInterval(t);
  }, [phase]);
  useEffect(() => {
    if (phase === 'committed') setOrder({
      from: 'meridian',
      to: 'kestrel',
      state: 'committed'
    });else if (phase === 'contested') setOrder({
      from: 'meridian',
      to: 'kestrel',
      state: 'preview'
    });else setOrder(null);
  }, [phase]);
  const toggle = id => setSelected(s => s.includes(id) ? s.filter(x => x !== id) : [...s, id]);
  const cycle = k => setDoctrine(d => {
    const o = DOCTRINE[k];
    return {
      ...d,
      [k]: o[(o.indexOf(d[k]) + 1) % o.length]
    };
  });
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: 2560,
      height: 1097,
      background: 'var(--void-0)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement(Playfield, {
    phase: phase,
    selectedSystem: selectedSystem,
    onSelectSystem: setSelectedSystem,
    route: overlays.routes ? order : null,
    elapsed: elapsed
  }), /*#__PURE__*/React.createElement(TopBar, {
    phase: phase,
    elapsed: elapsed,
    overlays: overlays,
    onToggleOverlay: k => setOverlays(o => ({
      ...o,
      [k]: !o[k]
    }))
  }), /*#__PURE__*/React.createElement(SelectionPanel, {
    fleets: FLEETS,
    selected: selected,
    onToggle: toggle,
    doctrine: doctrine,
    onCycle: cycle,
    phase: phase
  }), /*#__PURE__*/React.createElement(FrontRoster, {
    phase: phase,
    activeFront: activeFront,
    onSelectFront: setActiveFront
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: '50%',
      transform: 'translateX(-50%)',
      bottom: 'var(--hud-edge)',
      zIndex: 3
    }
  }, order ? /*#__PURE__*/React.createElement(OrderStrip, {
    fleets: selected,
    destination: "Kestrel Gate",
    state: order.state,
    travel: "18.0s",
    arrival: "T+04:12",
    onConfirm: order.state === 'preview' ? () => setPhase('committed') : undefined,
    onCancel: order.state === 'preview' ? () => setPhase('positioning') : undefined
  }) : /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '7px 10px',
      border: '1px dashed var(--sig-neutral-edge)',
      borderRadius: 'var(--r-1)',
      background: 'var(--surface-panel)',
      backdropFilter: 'var(--blur-panel)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, "no order staged \xB7 select fleets, then a destination", /*#__PURE__*/React.createElement(CommandButton, {
    size: "sm",
    onClick: () => setPhase('contested')
  }, "stage warp"))), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: '50%',
      transform: 'translateX(-50%)',
      top: 46,
      zIndex: 3,
      display: 'flex',
      gap: 5
    }
  }, PHASES.map(([p, label]) => /*#__PURE__*/React.createElement(CommandButton, {
    key: p,
    size: "sm",
    variant: phase === p ? 'commit' : 'ghost',
    onClick: () => setPhase(p)
  }, label))));
}
ReactDOM.createRoot(document.getElementById('root')).render(/*#__PURE__*/React.createElement(App, null));
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/hud/App.jsx", error: String((e && e.message) || e) }); }

// ui_kits/hud/HudChrome.jsx
try { (() => {
const {
  HudPanel,
  Hairline,
  Readout,
  ConfidenceTag,
  FleetChip,
  DoctrinePill,
  FrontRow,
  OrderStrip,
  ObjectiveBar,
  IconButton,
  CommandButton,
  Glyph,
  StateDot
} = window.FleetDesignSystem_d68ed7;
function TopBar({
  phase,
  elapsed,
  overlays,
  onToggleOverlay
}) {
  const obj = {
    positioning: {
      value: 0,
      mode: 'fill',
      tone: 'friendly'
    },
    contested: {
      value: 0.5,
      mode: 'contested',
      tone: 'enemy'
    },
    committed: {
      value: 0.75,
      mode: 'fill',
      tone: 'friendly'
    },
    decaying: {
      value: 0.31,
      mode: 'decay',
      tone: 'neutral'
    }
  }[phase];
  return /*#__PURE__*/React.createElement(ObjectiveBar, {
    style: {
      position: 'absolute',
      top: 0,
      left: 0,
      right: 0,
      zIndex: 3
    },
    left: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--type-label)',
        letterSpacing: '.22em',
        textTransform: 'uppercase',
        fontWeight: 700,
        color: 'var(--text-1)'
      }
    }, "Fleet"), /*#__PURE__*/React.createElement(Hairline, {
      vertical: true,
      style: {
        height: 16
      }
    }), /*#__PURE__*/React.createElement(Readout, {
      label: "drones",
      value: "4,280",
      size: "sm"
    }), /*#__PURE__*/React.createElement(Readout, {
      label: "capitals",
      value: "9",
      size: "sm"
    }), /*#__PURE__*/React.createElement(Readout, {
      label: "fronts",
      value: "3",
      size: "sm"
    }), /*#__PURE__*/React.createElement(Readout, {
      label: "clock",
      value: "04:12",
      size: "sm"
    })),
    right: /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(ConfidenceTag, {
      level: "estimated"
    }, "~3 capitals massing"), /*#__PURE__*/React.createElement(Readout, {
      label: "enemy seen",
      value: phase === 'positioning' ? '41s ago' : '2s ago',
      size: "sm",
      stale: phase === 'positioning',
      align: "right"
    }), /*#__PURE__*/React.createElement(Hairline, {
      vertical: true,
      style: {
        height: 16
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        gap: 5
      }
    }, /*#__PURE__*/React.createElement(IconButton, {
      icon: "radar",
      title: "Scouting overlay",
      active: overlays.scout,
      onClick: () => onToggleOverlay('scout')
    }), /*#__PURE__*/React.createElement(IconButton, {
      icon: "git-branch",
      title: "Route overlay",
      active: overlays.routes,
      onClick: () => onToggleOverlay('routes')
    }), /*#__PURE__*/React.createElement(IconButton, {
      icon: "crosshair",
      title: "Target priority",
      active: overlays.priority,
      onClick: () => onToggleOverlay('priority')
    }))),
    objective: {
      name: 'Shield Relay',
      seconds: 8,
      elapsed,
      ...obj,
      cores: [{
        label: 'YOU',
        tone: 'friendly',
        shielded: true
      }, {
        label: 'OPP',
        tone: 'enemy',
        shielded: phase === 'committed' ? false : true
      }]
    }
  });
}
const DOCTRINE = {
  stance: ['aggressive', 'defensive', 'screen'],
  formation: ['wedge', 'dispersed', 'screen'],
  priority: ['capitals', 'drones', 'relay']
};
const GLYPHS = {
  aggressive: 'aggressive',
  defensive: 'defensive',
  screen: 'screen',
  wedge: 'wedge',
  dispersed: 'dispersed',
  capitals: 'priorityCapital',
  drones: 'drone',
  relay: 'relay'
};
function SelectionPanel({
  fleets,
  selected,
  onToggle,
  doctrine,
  onCycle,
  phase
}) {
  const sel = fleets.filter(f => selected.includes(f.id));
  const drones = sel.reduce((n, f) => n + f.drones, 0);
  const capitals = sel.reduce((n, f) => n + f.capitals, 0);
  return /*#__PURE__*/React.createElement(HudPanel, {
    title: "Front 02 \xB7 Kestrel Approach",
    meta: sel.length + ' fleets',
    tone: "friendly",
    brackets: true,
    style: {
      position: 'absolute',
      left: 'var(--hud-edge)',
      bottom: 'var(--hud-edge)',
      width: 'var(--hud-panel-w)',
      zIndex: 3
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 'var(--sp-6)'
    }
  }, /*#__PURE__*/React.createElement(Readout, {
    label: "drones",
    value: drones.toLocaleString(),
    tone: "friendly"
  }), /*#__PURE__*/React.createElement(Readout, {
    label: "capitals",
    value: capitals,
    tone: "friendly"
  }), /*#__PURE__*/React.createElement(Readout, {
    label: "strength",
    value: sel.length ? Math.round(sel.reduce((n, f) => n + f.strength, 0) / sel.length * 100) : 0,
    unit: "%"
  })), /*#__PURE__*/React.createElement(Hairline, {
    inset: -12,
    style: {
      margin: '10px 0'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexWrap: 'wrap',
      gap: 5
    }
  }, Object.keys(DOCTRINE).map(k => /*#__PURE__*/React.createElement(DoctrinePill, {
    key: k,
    label: k,
    value: doctrine[k],
    glyph: GLYPHS[doctrine[k]],
    active: true,
    mixed: sel.length > 1 && k === 'formation',
    onClick: () => onCycle(k)
  }))), /*#__PURE__*/React.createElement(Hairline, {
    inset: -12,
    style: {
      margin: '10px 0'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gap: 3
    }
  }, fleets.map(f => /*#__PURE__*/React.createElement(FleetChip, {
    key: f.id,
    id: f.id,
    strength: f.strength,
    drones: f.drones.toLocaleString(),
    capitals: f.capitals,
    state: phase === 'committed' && selected.includes(f.id) ? 'committed' : f.state,
    selected: selected.includes(f.id),
    onClick: () => onToggle(f.id)
  }))));
}
function FrontRoster({
  phase,
  activeFront,
  onSelectFront
}) {
  const fronts = [{
    id: 'relay',
    name: 'Shield Relay',
    glyph: 'relay',
    urgency: phase === 'positioning' ? 'active' : 'critical',
    tone: phase === 'contested' ? 'enemy' : phase === 'committed' ? 'friendly' : 'neutral',
    status: phase === 'contested' ? 'contested' : phase === 'committed' ? 'capturing' : phase === 'decaying' ? 'decaying' : 'uncontested',
    progress: {
      positioning: 0,
      contested: .5,
      committed: .75,
      decaying: .31
    }[phase],
    mode: {
      positioning: 'fill',
      contested: 'contested',
      committed: 'fill',
      decaying: 'decay'
    }[phase]
  }, {
    id: 'kestrel',
    name: 'Kestrel Gate',
    glyph: 'gate',
    urgency: 'active',
    tone: 'friendly',
    status: 'inbound 12s',
    progress: .7,
    mode: 'fill'
  }, {
    id: 'vantage',
    name: 'Vantage Outpost',
    glyph: 'outpost',
    urgency: 'quiet',
    tone: 'neutral',
    status: 'unscouted'
  }, {
    id: 'home',
    name: 'Home Core',
    glyph: 'core',
    urgency: 'quiet',
    tone: 'friendly',
    status: 'quiet'
  }];
  return /*#__PURE__*/React.createElement(HudPanel, {
    title: "Fronts \xB7 by urgency",
    meta: "4",
    style: {
      position: 'absolute',
      right: 'var(--hud-edge)',
      bottom: 'var(--hud-edge)',
      width: 'var(--hud-panel-w)',
      zIndex: 3
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      margin: '0 -4px'
    }
  }, fronts.map(f => /*#__PURE__*/React.createElement(FrontRow, {
    key: f.id,
    name: f.name,
    status: f.status,
    urgency: f.urgency,
    tone: f.tone,
    glyph: f.glyph,
    progress: f.progress,
    progressMode: f.mode,
    selected: activeFront === f.id,
    onClick: () => onSelectFront(f.id)
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      height: 70,
      marginTop: 10,
      border: '1px solid var(--line-hairline)',
      background: 'rgba(255,255,255,.015)'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "100%",
    height: "100%",
    style: {
      display: 'block'
    }
  }, /*#__PURE__*/React.createElement("line", {
    x1: "14%",
    y1: "66%",
    x2: "34%",
    y2: "36%",
    stroke: "var(--line-hairline-strong)",
    strokeWidth: "1"
  }), /*#__PURE__*/React.createElement("line", {
    x1: "34%",
    y1: "36%",
    x2: "52%",
    y2: "52%",
    stroke: "var(--sig-friendly)",
    strokeWidth: "1"
  }), /*#__PURE__*/React.createElement("line", {
    x1: "52%",
    y1: "52%",
    x2: "72%",
    y2: "76%",
    stroke: "var(--line-hairline-strong)",
    strokeWidth: "1",
    strokeDasharray: "4 4"
  }), /*#__PURE__*/React.createElement("line", {
    x1: "52%",
    y1: "52%",
    x2: "80%",
    y2: "28%",
    stroke: "var(--sig-enemy)",
    strokeWidth: "1",
    opacity: ".7"
  }), /*#__PURE__*/React.createElement("line", {
    x1: "80%",
    y1: "28%",
    x2: "92%",
    y2: "58%",
    stroke: "var(--sig-enemy)",
    strokeWidth: "1",
    opacity: ".7"
  }), [['14%', '66%', 'var(--sig-friendly)'], ['34%', '36%', 'var(--sig-friendly)'], ['52%', '52%', 'var(--sig-neutral)'], ['72%', '76%', 'var(--sig-neutral)'], ['80%', '28%', 'var(--sig-enemy)'], ['92%', '58%', 'var(--sig-enemy)']].map(([x, y, c], i) => /*#__PURE__*/React.createElement("circle", {
    key: i,
    cx: x,
    cy: y,
    r: "2.5",
    fill: c
  })))));
}
Object.assign(window, {
  TopBar,
  SelectionPanel,
  FrontRoster,
  DOCTRINE,
  GLYPHS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/hud/HudChrome.jsx", error: String((e && e.message) || e) }); }

// ui_kits/hud/Playfield.jsx
try { (() => {
const {
  CaptureRing,
  Glyph,
  SelectionBrackets,
  Meter,
  ConfidenceTag,
  Icon,
  StateDot
} = window.FleetDesignSystem_d68ed7;
const SYSTEMS = [{
  id: 'home',
  name: 'Home Core',
  glyph: 'core',
  x: 9,
  y: 64,
  tone: 'friendly',
  sub: 'shielded'
}, {
  id: 'meridian',
  name: 'Meridian Gate',
  glyph: 'gate',
  x: 24,
  y: 34,
  tone: 'friendly',
  sub: 'held'
}, {
  id: 'kestrel',
  name: 'Kestrel Gate',
  glyph: 'gate',
  x: 39,
  y: 68,
  tone: 'friendly',
  sub: 'held'
}, {
  id: 'vantage',
  name: 'Vantage Outpost',
  glyph: 'outpost',
  x: 65,
  y: 76,
  tone: 'neutral',
  sub: 'unclaimed'
}, {
  id: 'thren',
  name: 'Thren Gate',
  glyph: 'gate',
  x: 79,
  y: 30,
  tone: 'enemy',
  sub: 'enemy held'
}, {
  id: 'opp',
  name: 'Command Core',
  glyph: 'core',
  x: 91,
  y: 58,
  tone: 'enemy',
  sub: 'exposed'
}];
function SystemNode({
  s,
  selected,
  onSelect
}) {
  const c = {
    friendly: 'var(--sig-friendly)',
    enemy: 'var(--sig-enemy)',
    neutral: 'var(--sig-neutral)'
  }[s.tone];
  const node = /*#__PURE__*/React.createElement("div", {
    onClick: () => onSelect(s.id),
    style: {
      display: 'grid',
      justifyItems: 'center',
      gap: 6,
      padding: 8,
      cursor: 'pointer'
    }
  }, /*#__PURE__*/React.createElement(Glyph, {
    name: s.glyph,
    size: 26,
    tone: s.tone,
    strokeWidth: 1.25
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      justifyItems: 'center',
      gap: 2
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-1)',
      whiteSpace: 'nowrap'
    }
  }, s.name), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: s.tone === 'neutral' ? 'var(--text-3)' : c,
      whiteSpace: 'nowrap'
    }
  }, s.sub)));
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: s.x + '%',
      top: s.y + '%',
      transform: 'translate(-50%,-50%)'
    }
  }, selected ? /*#__PURE__*/React.createElement(SelectionBrackets, {
    inset: -2,
    size: 12
  }, node) : node);
}
function FleetMarker({
  x,
  y,
  id,
  tone,
  count,
  certainty = 'confirmed',
  stance = 'aggressive'
}) {
  const c = tone === 'enemy' ? 'var(--sig-enemy)' : 'var(--sig-friendly)';
  const est = certainty !== 'confirmed';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: x + '%',
      top: y + '%',
      transform: 'translate(-50%,-50%)',
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      padding: '3px 7px',
      border: '1px ' + (est ? 'dashed' : 'solid') + ' ' + (est ? 'var(--sig-neutral-edge)' : c),
      background: 'rgba(7,10,15,.72)',
      borderRadius: 'var(--r-2)',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement(Glyph, {
    name: est ? 'unknown' : stance,
    size: 12,
    tone: est ? 'neutral' : tone
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-readout)',
      fontSize: 'var(--fs-body)',
      color: est ? 'var(--text-stale)' : c
    }
  }, id), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, count));
}
function Route({
  from,
  to,
  state
}) {
  const a = SYSTEMS.find(s => s.id === from),
    b = SYSTEMS.find(s => s.id === to);
  if (!a || !b) return null;
  const committed = state === 'committed';
  const c = committed ? 'var(--sig-friendly)' : 'var(--sig-neutral)';
  const mid = {
    x: (a.x + b.x) / 2,
    y: (a.y + b.y) / 2
  };
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("svg", {
    style: {
      position: 'absolute',
      inset: 0,
      width: '100%',
      height: '100%',
      pointerEvents: 'none',
      overflow: 'visible'
    }
  }, /*#__PURE__*/React.createElement("defs", null, /*#__PURE__*/React.createElement("marker", {
    id: "rt-arrow",
    viewBox: "0 0 8 8",
    refX: "7",
    refY: "4",
    markerWidth: "7",
    markerHeight: "7",
    orient: "auto"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M0 0 L8 4 L0 8",
    fill: "none",
    stroke: committed ? '#22CFE8' : '#98A4AE',
    strokeWidth: "1.4"
  }))), /*#__PURE__*/React.createElement("line", {
    x1: a.x + '%',
    y1: a.y + '%',
    x2: b.x + '%',
    y2: b.y + '%',
    stroke: c,
    strokeWidth: "1.25",
    strokeDasharray: committed ? undefined : '5 5',
    markerEnd: "url(#rt-arrow)",
    opacity: committed ? 1 : 0.8
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: mid.x + '%',
      top: mid.y + '%',
      transform: 'translate(-50%,-50%)',
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      padding: '2px 6px',
      background: 'rgba(7,10,15,.85)',
      border: '1px ' + (committed ? 'solid' : 'dashed') + ' ' + (committed ? 'var(--sig-friendly-edge)' : 'var(--sig-neutral-edge)'),
      borderRadius: 'var(--r-2)',
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: committed ? 'var(--cyan-300)' : 'var(--text-3)',
      whiteSpace: 'nowrap'
    }
  }, committed ? /*#__PURE__*/React.createElement(Icon, {
    name: "lock",
    size: 10
  }) : /*#__PURE__*/React.createElement(Glyph, {
    name: "unknown",
    size: 10
  }), committed ? 'warp locked' : 'planned'));
}
function Relay({
  phase,
  elapsed
}) {
  const cfg = {
    positioning: {
      value: 0,
      mode: 'fill',
      tone: 'neutral',
      sub: 'uncontested'
    },
    contested: {
      value: 0.5,
      mode: 'contested',
      tone: 'enemy',
      sub: 'contested'
    },
    committed: {
      value: 0.75,
      mode: 'fill',
      tone: 'friendly',
      sub: 'capturing'
    },
    decaying: {
      value: 0.31,
      mode: 'decay',
      tone: 'neutral',
      sub: 'decaying'
    }
  }[phase];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: '52%',
      top: '44%',
      transform: 'translate(-50%,-50%)',
      display: 'grid',
      justifyItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(SelectionBrackets, {
    inset: -14,
    size: 16,
    tone: cfg.tone === 'neutral' ? 'neutral' : cfg.tone
  }, /*#__PURE__*/React.createElement(CaptureRing, {
    size: 190,
    segments: 8,
    value: cfg.value,
    mode: cfg.mode,
    tone: cfg.tone,
    label: elapsed.toFixed(1),
    sub: cfg.sub
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-1)'
    }
  }, "Shield Relay"), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--type-label)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-3)'
    }
  }, "8s hold \xB7 drops opponent core shield"));
}
function Playfield({
  phase,
  selectedSystem,
  onSelectSystem,
  route,
  elapsed
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      background: 'var(--void-1)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      backgroundImage: 'linear-gradient(var(--line-grid) 1px, transparent 1px), linear-gradient(90deg, var(--line-grid) 1px, transparent 1px)',
      backgroundSize: '80px 80px'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      background: 'radial-gradient(58% 70% at 52% 44%, rgba(20,30,42,.55) 0%, transparent 70%)'
    }
  }), route && /*#__PURE__*/React.createElement(Route, route), /*#__PURE__*/React.createElement(Relay, {
    phase: phase,
    elapsed: elapsed
  }), SYSTEMS.map(s => /*#__PURE__*/React.createElement(SystemNode, {
    key: s.id,
    s: s,
    selected: selectedSystem === s.id,
    onSelect: onSelectSystem
  })), /*#__PURE__*/React.createElement(FleetMarker, {
    x: 33,
    y: 54,
    id: "04",
    tone: "friendly",
    count: "880",
    stance: "aggressive"
  }), /*#__PURE__*/React.createElement(FleetMarker, {
    x: 44,
    y: 80,
    id: "07",
    tone: "friendly",
    count: "510",
    stance: "defensive"
  }), /*#__PURE__*/React.createElement(FleetMarker, {
    x: 18,
    y: 50,
    id: "11",
    tone: "friendly",
    count: "1,240",
    stance: "screen"
  }), phase !== 'positioning' && /*#__PURE__*/React.createElement(FleetMarker, {
    x: 62,
    y: 38,
    id: "E2",
    tone: "enemy",
    count: "~900"
  }), /*#__PURE__*/React.createElement(FleetMarker, {
    x: 73,
    y: 50,
    id: "E?",
    tone: "enemy",
    count: "~3 cap",
    certainty: "estimated"
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: '86%',
      top: '72%',
      transform: 'translate(-50%,-50%)'
    }
  }, /*#__PURE__*/React.createElement(ConfidenceTag, {
    level: "stale"
  }, "unscouted \xB7 41s")));
}
Object.assign(window, {
  Playfield,
  SYSTEMS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/hud/Playfield.jsx", error: String((e && e.message) || e) }); }

__ds_ns.CommandButton = __ds_scope.CommandButton;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.ObjectiveBar = __ds_scope.ObjectiveBar;

__ds_ns.DoctrinePill = __ds_scope.DoctrinePill;

__ds_ns.FleetChip = __ds_scope.FleetChip;

__ds_ns.FrontRow = __ds_scope.FrontRow;

__ds_ns.OrderStrip = __ds_scope.OrderStrip;

__ds_ns.Icon = __ds_scope.Icon;

__ds_ns.CaptureRing = __ds_scope.CaptureRing;

__ds_ns.ConfidenceTag = __ds_scope.ConfidenceTag;

__ds_ns.Glyph = __ds_scope.Glyph;

__ds_ns.Meter = __ds_scope.Meter;

__ds_ns.Readout = __ds_scope.Readout;

__ds_ns.StateDot = __ds_scope.StateDot;

__ds_ns.Hairline = __ds_scope.Hairline;

__ds_ns.HudPanel = __ds_scope.HudPanel;

__ds_ns.SelectionBrackets = __ds_scope.SelectionBrackets;

})();
