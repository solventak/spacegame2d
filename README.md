# Relay Operations

A 2D space simulation in Rust using wgpu. Drones move under autopilot; collision avoidance via predicted neighbor trajectories is in active development. The repository and crate name remain `spacegame2d`.

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

### HUD development and runtime

The client embeds its connection screen and compact local-player HUD from the committed production bundle in `crates/spacegame2d/hud/dist`; it never starts a frontend server. On startup it shows an editable connection form and never auto-connects. Development builds prefill `127.0.0.1:4000`; release builds require the public address at compile time:

```sh
SPACEGAME_RELEASE_ADDRESS=play.example.com:4000 cargo build --release -p spacegame2d
```

The form address is not persisted. To change the frontend, use Node 22 and run:

```sh
./scripts/qa-hud.sh
```

Linux builds need `libwebkit2gtk-4.1-dev` and `pkg-config`; Ubuntu testers need `libwebkit2gtk-4.1-0` and XWayland. On Linux, Wry hosts the HUD as an X11 child of the Winit game window, so Wayland sessions run through XWayland. Windows testers need the Evergreen WebView2 Runtime. The executable embeds HUD assets, but relies on the platform WebView runtime.

## Lint and format

```
cargo fmt
cargo clippy -- -D warnings
```

## Contributor / agent conventions

See [`AGENTS.md`](./AGENTS.md) for branch, PR, and ticketing conventions. Both human contributors and background automated workers must follow them.

When `dev` is ready for a release, use the [release-from-dev runbook](./docs/runbooks/release-from-dev.md)
to open the promotion PR to `main`.

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
