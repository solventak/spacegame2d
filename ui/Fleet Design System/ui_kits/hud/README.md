# UI kit — Fleet in-match HUD (21:9)

A high-fidelity recreation of the only surface Fleet has: the in-match ultrawide HUD.
Design canvas is **2560 × 1097** (21:9); `index.html` scales it to fit the viewport.

## Files
- `index.html` — mounts the kit; loads `styles.css` + `_ds_bundle.js`.
- `Playfield.jsx` — the battlefield: system nodes, fleet markers, routes, the Shield Relay capture ring.
- `HudChrome.jsx` — `TopBar` (slim objective bar), `SelectionPanel` (bottom-left), `FrontRoster` (bottom-right).
- `App.jsx` — state machine + phase switcher.

## Interactions
- **Phase switcher** (top centre): calm positioning → contested relay → warp committed → relay decaying.
- Click **fleet chips** to multi-select; the selection summary and order strip update.
- Click **doctrine pills** to cycle stance / formation / target priority.
- Click **system nodes** to move the selection brackets.
- In the contested phase the order strip is a dashed **preview** — `commit warp` locks it (solid cyan + lock) and starts the relay capture ticking toward 8s.
- Top-right icon toggles switch the scouting / route / priority overlays.

## Layout rules this kit demonstrates
Chrome never crosses `--hud-edge` (14px) into the playfield. The top bar is 34px.
Objective state is drawn **on the objective**, not in a panel. Nothing is coloured except by allegiance.
