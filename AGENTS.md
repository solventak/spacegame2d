# AGENTS.md — Repository Conventions

> Read this before making any change. Both human contributors and background automated workers operate under the same rules.

## Project

`spacegame2d` — a 2D space simulation in Rust using wgpu. Drones move under autopilot and (currently in active development) avoid collisions via local predicted neighbor avoidance. The release branch is `main`; day-to-day development happens on `dev`.

## Toolchain

- Rust compiler pinned by `rustc 1.89` (MSRV `1.89`); Cargo edition `2024` (`Cargo.toml`).
- All dependencies in `Cargo.toml` are pinned with exact versions (`=x.y.z`). Do not relax pinning without coordinating.

## Source layout

```
src/
  main.rs              entry, wgpu setup, frame loop
  simulation.rs        sim tick, drone state, neighbor interactions
  input.rs             keyboard / mouse
  autopilot.rs         velocity targeting
  flight_control/
    mod.rs             facade re-exporting arrival public surface
    arrival.rs         velocity arrival + predicted neighbor avoidance math
  shader.wgsl          GPU shader
docs/
  plans/               milestone planning documents (one file per plan, dated slug)
```

## Common commands

| Need | Command |
|---|---|
| Build | `cargo build` |
| Run | `cargo run` |
| Format (write) | `cargo fmt` |
| Format (check) | `cargo fmt --check` |
| Lint | `cargo clippy -- -D warnings` |
| Test | `cargo test` |
| **Test gate** (full) | `cargo fmt --check && cargo clippy -- -D warnings && cargo test` |

The implementer must run the **test gate** locally before opening a PR. If any step fails, do not push; instead comment on the Linear ticket with the failure and move it to "Blocked".

## Branch and PR conventions

- Base development branch: **`dev`**. Releases go from `dev` to `main` via a release PR.
- Feature branch format: `droid/<TICKET-ID>-<kebab-summary>`, e.g. `droid/SPC-12-velocity-arrival`. Human-led informal work may use other prefixes.
- **Never push to `main`.** Both humans and automated workers refuse this. `main` only accepts PRs from `dev`.
- PR title format: **`<TICKET-ID>: <summary>`**. Example: `SPC-12: implement velocity arrival correction`.
- PR body must include:
  - `Closes <TICKET-ID>` linking the Linear ticket (single ticket per PR).
  - **Acceptance Criteria** as `- [ ]` checkboxes pulled from the ticket.
  - One-paragraph **Summary of files touched**.
  - **Test Output** section with the tail of the `cargo test` run.
  - For stacked PRs, a `Stacked on #<PR-N>` line at the top.
- No force-pushes to `dev` or `main`. Force-push your own feature branch only when you own it end-to-end.

## Ticketing and plans

- Active work is tracked in Linear (the configured workspace). Tickets use this template:

  ```
  ## Requirements
  - ...

  ## Plan
  - ...

  ## Acceptance Criteria
  - [ ] ...

  ## Out of Scope
  - ...

  ## Risks
  - (optional)
  ```

- If a ticket is too large for a single PR or spans multi-day work, mirror it into a dated plan doc at `docs/plans/<YYYY-MM-DD>-<slug>.md` and reference the path in the ticket's "Plan" section. Existing examples:
  - `docs/plans/2026-07-23-ship-movement.md`
  - `docs/plans/2026-07-23-right-click-autopilot.md`
  - `docs/plans/2026-07-23-local-predicted-neighbor-avoidance.md`

## Stacked PRs

If a new ticket depends on an unmerged PR's work, branch off that PR's branch instead of `dev` and include `Stacked on #<PR-N>` in the new PR body. Don't try to silently rebase onto a just-merged PR; the human reviewer can decide whether to merge the stack.

## Logging

Use `log` + `env_logger` for runtime messages. Use `println!` only for genuine startup banners or one-off diagnostics. Don't introduce `tracing` without sign-off from the maintainer; the codebase currently has minimal logging on purpose.

## Pre-commit hook

This repo keeps git hooks versioned under `.githooks/`. After cloning, run:

```
git config core.hooksPath .githooks
```

The pre-commit hook runs the test gate and blocks the commit if any step fails. The hook is advisory for the droid; the implementer must run the gate explicitly before pushing regardless.

## Style overrides

- `rustfmt.toml` sets `max_width = 100` and edition `2024`.
- `clippy.toml` pins MSRV to `1.89`. Warnings are denied via `-D warnings` on the test gate.

Follow existing module conventions: small focused files, `mod.rs` re-exporting only the public surface, tests at the bottom of each file under `#[cfg(test)] mod tests { ... }`.

## Documentation discipline

- Update `docs/plans/` when kicking off a new milestone.
- Keep plan docs alongside the work on `dev`; they do not belong on `main`.
