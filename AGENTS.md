# AGENTS.md — Repository Conventions

> Read this before making any change. Both human contributors and background automated workers operate under the same rules.

## Project

`spacegame2d` — a 2D space simulation in Rust using wgpu. Drones move under autopilot and (currently in active development) avoid collisions via local predicted neighbor avoidance. The release branch is `main`; day-to-day development happens on `dev`.

## Toolchain

- Rust compiler pinned by `rustc 1.89` (MSRV `1.89`); Cargo edition `2024` (`Cargo.toml`).
- All dependencies in `Cargo.toml` are pinned with exact versions (`=x.y.z`). Do not relax pinning without coordinating.

## Source layout

```
crates/
  simulation/
    Cargo.toml
    src/
      lib.rs            re-exports simulation, flight_control, autopilot, fleet
      simulation.rs      sim tick, drone state, neighbor interactions
      autopilot.rs       velocity targeting
      fleet.rs           drone collection (spawn, step, cull, reset)
      flight_control/
        mod.rs           facade re-exporting arrival public surface
        arrival.rs       velocity arrival + predicted neighbor avoidance math
  protocol/
    Cargo.toml
    src/
      lib.rs             placeholder — wire-format and codec (future tickets)
  server/
    Cargo.toml
    src/
      main.rs            stub — banner + exit (no networking yet)
  spacegame2d/
    Cargo.toml
    src/
      main.rs            entry, wgpu setup, frame loop
      input.rs           keyboard / mouse
      geometry/
        mod.rs            Vertex type + wgpu layout
        overlay.rs        arena ring vertices
        units.rs          ship sprite vertices
      shader.wgsl         GPU shader
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

- Active work is tracked in Linear (the configured workspace). Linear is for tracking only — state, assignment, priorities, requirements, acceptance criteria. Implementation plans live in Notion.

- Tickets use this template:

  ```
  ## Requirements
  - ...

  ## Plan
  See <Notion page URL>

  ## Acceptance Criteria
  - [ ] ...

  ## Out of Scope
  - ...

  ## Risks
  - (optional)
  ```

- The `## Plan` section in a Linear ticket is a one-line pointer to a Notion page in the **Implementation Plans** database. The Notion page is the canonical store for the implementation plan, files touched, API surface, and risk level.

- The Notion page has a **Status** property that drives the workflow:
  - `Needs Review` — the `linear-planner` droid just wrote the plan; awaiting human review.
  - `Approved` — a human has reviewed and signed off; the `linear-implementer` droid may proceed.
  - `Implemented` — the PR has been merged; plan is complete.

- The `linear-planner` droid creates Notion pages with Status = `Needs Review`. The user flips Status to `Approved` when ready. The `linear-implementer` droid checks the Status before proceeding — it only runs if Status = `Approved`.

- The `pr-reviewer` droid does NOT read the plan. It reviews code on its own merits against AGENTS.md conventions and the Linear ticket's acceptance criteria.

- Plan docs in `docs/plans/` are an optional historical archive. The implementer may commit the final plan alongside the PR if a permanent git record is desired, but the canonical plan store is Notion.

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
