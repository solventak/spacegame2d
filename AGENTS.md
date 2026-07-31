# AGENTS.md — Repository Conventions

> Read this before making any change. Both human contributors and background automated workers operate under the same rules.

## Project

**Relay Operations** (`spacegame2d`) — a 2D space simulation in Rust using wgpu. Drones move under autopilot and (currently in active development) avoid collisions via local predicted neighbor avoidance. The release branch is `main`; day-to-day development happens on `dev`.

## Toolchain

- Rust compiler pinned by `rustc 1.91` (MSRV `1.91`); Cargo edition `2024` (`Cargo.toml`).
- All dependencies in `Cargo.toml` are pinned with exact versions (`=x.y.z`). Do not relax pinning without coordinating.

## UI design system

For any UI, HUD, rendering, interaction, copy, icon, or visual-style work, inspect
[`ui/Fleet Design System/`](ui/Fleet%20Design%20System/) before making a change. Its
[`readme.md`](ui/Fleet%20Design%20System/readme.md) is the canonical source of truth;
do not recreate its guidance elsewhere.

| Need | Start here |
|---|---|
| Visual rules, copy, colour, type, spacing, motion, and icon usage | `ui/Fleet Design System/readme.md` |
| Reusable CSS variables and global styles | `ui/Fleet Design System/styles.css`, then `tokens/` |
| React component patterns | `ui/Fleet Design System/components/` (`surfaces`, `signals`, `fleet`, `controls`, `icons`) |
| Foundation visual specimens | `ui/Fleet Design System/guidelines/` |
| Working in-match HUD reference and interaction states | `ui/Fleet Design System/ui_kits/hud/README.md`, then `App.jsx`, `HudChrome.jsx`, and `Playfield.jsx` |
| Agent-specific usage instructions | `ui/Fleet Design System/SKILL.md` |

Treat the system as the first proposal for the current HUD, including its documented font,
icon, logo, and copy substitutions. Preserve its semantic rules: cyan is friendly, coral is
enemy, gray is neutral/uncertain; tactical `Glyph` and interface `Icon` are distinct systems.

## Common commands

| Need | Command |
|---|---|
| Build | `cargo build` |
| Run | `cargo run` |
| Format (write) | `cargo fmt` |
| Format (check) | `cargo fmt --check` |
| Lint | `cargo clippy -- -D warnings` |
| Test | `cargo test` |
| Coverage (local) | `cargo tarpaulin --workspace --skip-clean --exclude-files "*/main.rs" --fail-under 85` |
| **Test gate** (full) | `cargo fmt --check && cargo clippy -- -D warnings && cargo test` |

Run the **test gate** locally before opening a PR when the change includes Rust sources, Cargo
configuration, or native/HUD lifecycle code. Infrastructure, workflow, documentation, and other
non-Rust-only changes instead run their relevant local checks; CI remains the full repository
quality gate. If a required local check fails, do not push; instead comment on the Linear ticket
with the failure and move it to "Blocked".

Terraform changes under `infra/` must run `terraform fmt -check -recursive infra`, validate both
Terraform roots with `terraform init -backend=false -input=false` followed by `terraform validate`,
and run `terraform -chdir=infra test` when Terraform test files are present.

Changes under `crates/spacegame2d/hud/`, `crates/spacegame2d/src/hud.rs`, or native HUD lifecycle code must also run `./scripts/qa-hud.sh` after the Rust test gate.

## Coverage gate

A **minimum coverage threshold of 85%** is enforced in CI via `cargo-tarpaulin` (see `.github/workflows/coverage.yml`). The gate runs on every PR and push to `dev`/`main` and will **fail the build** if coverage drops below 85%.

Coverage is measured across the whole workspace but excludes `main.rs` files (GUI rendering and event-loop code that is not unit-testable). All simulation logic, flight control, autopilot, fleet, geometry, and input modules are measured and must stay above the threshold.

When adding new logic, ensure it is accompanied by tests that keep coverage above 85%. Stub or placeholder crates (e.g. `protocol`, `server`) contribute negligibly today but will be measured as they gain code.

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

- Active work is tracked in Linear (the configured workspace), including state, assignment,
  priorities, requirements, acceptance criteria, and dependencies. Implementation plans live
  only in the active Codex task: do not create, require, or consult duplicate plan copies in
  Linear or external systems.

- Tickets use this template:

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
  ui/
    Fleet Design System/  canonical UI language, tokens, components, and HUD reference
  ```

  ## UI design system

  For any UI, HUD, rendering, interaction, copy, icon, or visual-style work, inspect
  [`ui/Fleet Design System/`](ui/Fleet%20Design%20System/) before making a change. Its
  [`readme.md`](ui/Fleet%20Design%20System/readme.md) is the canonical source of truth;
  do not recreate its guidance elsewhere.

  | Need | Start here |
  |---|---|
  | Visual rules, copy, colour, type, spacing, motion, and icon usage | `ui/Fleet Design System/readme.md` |
  | Reusable CSS variables and global styles | `ui/Fleet Design System/styles.css`, then `tokens/` |
  | React component patterns | `ui/Fleet Design System/components/` (`surfaces`, `signals`, `fleet`, `controls`, `icons`) |
  | Foundation visual specimens | `ui/Fleet Design System/guidelines/` |
  | Working in-match HUD reference and interaction states | `ui/Fleet Design System/ui_kits/hud/README.md`, then `App.jsx`, `HudChrome.jsx`, and `Playfield.jsx` |
  | Agent-specific usage instructions | `ui/Fleet Design System/SKILL.md` |

  Treat the system as the first proposal for the current HUD, including its documented font,
  icon, logo, and copy substitutions. Preserve its semantic rules: cyan is friendly, coral is
  enemy, gray is neutral/uncertain; tactical `Glyph` and interface `Icon` are distinct systems.

  ## Common commands

  | Need | Command |
  |---|---|
  | Build | `cargo build` |
  | Run | `cargo run` |
  | Format (write) | `cargo fmt` |
  | Format (check) | `cargo fmt --check` |
  | Lint | `cargo clippy -- -D warnings` |
  | Test | `cargo test` |
  | Coverage (local) | `cargo tarpaulin --workspace --skip-clean --exclude-files "*/main.rs" --fail-under 85` |
  | **Test gate** (full) | `cargo fmt --check && cargo clippy -- -D warnings && cargo test` |

  Run the **test gate** locally before opening a PR when the change includes Rust sources, Cargo
  configuration, or native/HUD lifecycle code. Infrastructure, workflow, documentation, and other
  non-Rust-only changes instead run their relevant local checks; CI remains the full repository
  quality gate. If a required local check fails, do not push; instead comment on the Linear ticket
  with the failure and move it to "Blocked".

  Terraform changes under `infra/` must run `terraform fmt -check -recursive infra`, validate both
  Terraform roots with `terraform init -backend=false -input=false` followed by `terraform validate`,
  and run `terraform -chdir=infra test` when Terraform test files are present.

  Changes under `crates/spacegame2d/hud/`, `crates/spacegame2d/src/hud.rs`, or native HUD lifecycle code must also run `./scripts/qa-hud.sh` after the Rust test gate.

  ## Coverage gate

  A **minimum coverage threshold of 85%** is enforced in CI via `cargo-tarpaulin` (see `.github/workflows/coverage.yml`). The gate runs on every PR and push to `dev`/`main` and will **fail the build** if coverage drops below 85%.

  Coverage is measured across the whole workspace but excludes `main.rs` files (GUI rendering and event-loop code that is not unit-testable). All simulation logic, flight control, autopilot, fleet, geometry, and input modules are measured and must stay above the threshold.

  When adding new logic, ensure it is accompanied by tests that keep coverage above 85%. Stub or placeholder crates (e.g. `protocol`, `server`) contribute negligibly today but will be measured as they gain code.

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

  - Active work is tracked in Linear (the configured workspace). Linear is for tracking only — state, assignment, priorities, requirements, acceptance criteria.

  - Tickets use this template:

    ```
    ## Requirements
    - ...

    ## Acceptance Criteria
    - [ ] ...

    ## Out of Scope
    - ...

    ## Risks
    - (optional)
    ```


  - Plan docs in `docs/plans/` are a historical archive. The implementer should not commit the final plan alongside the PR.

  ## Stacked PRs

  If a new ticket depends on an unmerged PR's work, branch off that PR's branch instead of `dev` and include `Stacked on #<PR-N>` in the new PR body. Don't try to silently rebase onto a just-merged PR; the human reviewer can decide whether to merge the stack.

  ## Logging

  Use `tracing` + `tracing-subscriber` for runtime messages, via the `spacegame2d-logging` crate. `tracing` is explicitly permitted for new networking code; the pre-existing `log` + `env_logger` calls in the current 2D client/sim code remain as-is for now. The logging crate provides dual stdout + non-blocking file output, per-run log files under `logs/` named `<timestamp>_<binary>_<pid>.log`, `EnvFilter`, and a JSON mode via `SPACEGAME_LOG_FORMAT=json`. Use `println!` only for genuine startup banners or one-off diagnostics.

  New networking code (server, client, protocol) emits structured `tracing` events using the canonical field vocabulary: `event`, `cmd` (formatted `slot:sequence`), `tick`, `execute_tick`, `receive_tick`, `local_tick`, `server_tick`, `kind`, `address`, `slot`, `recipients`, plus event-specific fields. Keep the codebase's logging minimal and purposeful — log state transitions and wire events, not per-tick noise.

  The pre-existing `log` + `env_logger` calls in the current 2D client/sim code are left as-is for now; only new networking code formalizes the `tracing` convention. `tracing` is permitted (maintainer sign-off recorded in SWA-11); do not introduce other logging frameworks.

  ## Pre-commit hook

  This repo keeps git hooks versioned under `.githooks/`. After cloning, run:

  ```
  git config core.hooksPath .githooks
  ```

  The pre-commit hook runs the Rust test gate only when staged files include Rust, Cargo, or native
  HUD lifecycle code. The hook is advisory for the droid; the implementer must run the checks
  required by the changed file types before pushing regardless.

  ## Style overrides

  - `rustfmt.toml` sets `max_width = 100` and edition `2024`.
  - `clippy.toml` pins MSRV to `1.91`. Warnings are denied via `-D warnings` on the test gate.

  Follow existing module conventions: small focused files, `mod.rs` re-exporting only the public surface, tests at the bottom of each file under `#[cfg(test)] mod tests { ... }`.


  ## API Preferences

  - Prefer `From` and `TryFrom` implementations over free conversion helper functions. Use `From` for infallible conversions and `TryFrom` for conversions that can fail.
  - Keep serialization behavior on the type being serialized. For example, expose `Message::encode()` and `Message::write(...)` rather than `encode_message` or `write_message` free functions.
  - Put behavior on the domain type it belongs to, such as `ClientHello::is_compatible()` and `Client` socket methods, instead of standalone helpers.
  - Preserve structured domain values in tracing fields when possible; prefer recording enums with `?value` over converting them to display strings.
  - Use domain-specific types such as `Tick` for simulation time instead of repeating primitive integer types at protocol boundaries.
  - Use `thiserror` for project error enums; use `Option` only for genuinely absent values and `Result` for conversion or validation failures.
  - Keep wire/protocol DTOs separate from simulation domain types when they serve different purposes: protocol types model serialized data, while simulation newtypes enforce domain invariants. Convert explicitly at the boundary.
  - Put domain arithmetic on domain types. For example, `Tick::increment(...)` owns tick advancement rather than repeating primitive arithmetic at call sites.

  ## Documentation discipline

  - Update `docs/plans/` when kicking off a new milestone.
  - Keep plan docs alongside the work on `dev`; they do not belong on `main`.

  ## Rust design guidance

  See [`docs/rust-guidelines.md`](docs/rust-guidelines.md) for the concise Rust-specific design guidance. In particular, think through module and type interfaces during planning, keep public surfaces intentional, and prefer behavior on the type that owns it.
