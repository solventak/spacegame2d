# spacegame2d

A 2D space simulation in Rust using wgpu. Drones move under autopilot; collision avoidance via predicted neighbor trajectories is in active development.

## Quick start

```
cargo run
```

A window opens showing the 16 m arena ring and 30 drone ships. The player ship starts at the origin.

### Controls

| Input | Action |
|---|---|
| `W` | Forward thrust |
| `A` | Turn left (counterclockwise) |
| `D` | Turn right (clockwise) |
| `R` | Reset simulation |
| Right-click | Set autopilot destination |
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

## Layout

```
crates/
  spacegame2d/           GUI app — wgpu setup, frame loop, input, shaders
  simulation/            game logic — sim tick, drone state, autopilot, fleet
  protocol/              wire-format stub (future tickets)
  server/                server stub (banner + exit)
docs/
  QA.md                  interactive QA guide (headless + GUI paths)
  plans/                 milestone planning documents
scripts/
  qa-headless.sh         headless QA script (test gate + server smoke test)
```

## Milestones

- `docs/plans/2026-07-23-ship-movement.md`
- `docs/plans/2026-07-23-right-click-autopilot.md`
- `docs/plans/2026-07-23-local-predicted-neighbor-avoidance.md`
