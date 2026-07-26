# spacegame2d

A 2D space simulation in Rust using wgpu. Drones move under autopilot; collision avoidance via predicted neighbor trajectories is in active development.

## Quick start

```
cargo run
```

A window opens showing the 64 m arena ring and 30 drone ships by default. The local camera starts at the world origin.

### Server configuration

The server uses 30 drones per fleet and a 64 m arena radius by default. Set `SPACEGAME_FLEET_SIZE` or `SPACEGAME_WORLD_RADIUS_METERS` before starting it to tune validated authoritative configuration; clients receive both values during the version-9 handshake.

### Controls

| Input | Action |
|---|---|
| `W` | Forward thrust |
| `A` | Turn left (counterclockwise) |
| `D` | Turn right (clockwise) |
| `R` | Reset the shared simulation |
| Right-click | Set autopilot destination |
| Middle-click drag | Pan the local camera |
| Close window | Exit |

## QA

**Headless (no GPU/display required):**

```sh
./scripts/qa-headless.sh
```

This runs the full test gate (format + lint + tests) and a server smoke test. The test suite exercises the complete simulation game loop: ship movement, autopilot navigation, reset, world boundary destruction, and drone fleet behavior.

**Test gate:**

```
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

See [`docs/QA.md`](./docs/QA.md) for the full interactive QA guide (headless and GUI paths, controls, and verification scenarios).

## Lint and format

```
cargo fmt
cargo clippy -- -D warnings
```

## Contributor / agent conventions

See [`AGENTS.md`](./AGENTS.md) for branch, PR, and ticketing conventions. Both human contributors and background automated workers must follow them.

For UI, HUD, visual, and interaction work, start with the
[`Fleet Design System`](./ui/Fleet%20Design%20System/readme.md). It indexes the design tokens,
components, visual guidelines, and interactive in-match HUD reference; the task-oriented lookup
table is also in [`AGENTS.md`](./AGENTS.md#ui-design-system).

## Layout

```
crates/
  spacegame2d/           GUI app — wgpu setup, frame loop, input, shaders
  simulation/            game logic — sim tick, drone state, autopilot, fleet
  protocol/              versioned wire protocol
  server/                authoritative simulation server
docs/
  QA.md                  interactive QA guide (headless + GUI paths)
  plans/                 milestone planning documents
scripts/
  qa-headless.sh         headless QA script (test gate + server smoke test)
ui/
  Fleet Design System/   canonical design tokens, components, guidelines, and HUD kit
```

## Milestones

- `docs/plans/2026-07-23-ship-movement.md`
- `docs/plans/2026-07-23-right-click-autopilot.md`
- `docs/plans/2026-07-23-local-predicted-neighbor-avoidance.md`
