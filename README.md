# spacegame2d

A 2D space simulation in Rust using wgpu. Drones move under autopilot; collision avoidance via predicted neighbor trajectories is in active development.

## Run

```
cargo run
```

## Test

```
cargo test
```

## Lint and format

```
cargo fmt
cargo clippy -- -D warnings
```

## Contributor / agent conventions

See [`AGENTS.md`](./AGENTS.md) for branch, PR, and ticketing conventions. Both human contributors and background automated workers must follow them.

## Layout

```
src/main.rs              entry, wgpu setup, frame loop
src/simulation.rs        sim tick + drone state
src/input.rs             keyboard / mouse
src/autopilot.rs         velocity targeting
src/flight_control/      arrival and avoidance math
src/shader.wgsl          GPU shader
docs/plans/              milestone planning documents
```

## Milestones

- `docs/plans/2026-07-23-ship-movement.md`
- `docs/plans/2026-07-23-right-click-autopilot.md`
- `docs/plans/2026-07-23-local-predicted-neighbor-avoidance.md`
