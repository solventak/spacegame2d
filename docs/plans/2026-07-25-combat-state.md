# Deterministic Turret and Hull Combat State

## Ticket

SWA-28 — Add deterministic turret and hull combat state.

## Outcome

Every authoritative simulation unit has deterministic persistent hull and one independently oriented turret. The change prepares the simulation for autonomous engagement without adding target selection, tracking, firing, damage, or rendering behavior.

## Design

- Add `combat.rs` with `HullState`, `TurretState`, and `CombatState`.
- Use world-space turret heading, initialized from the owning ship heading.
- Initialize hull to 100/100, target to absent, and cooldown to zero.
- Keep initial weapon parameters as compile-time simulation constants.
- Add `CombatState` to `command::Unit`; both normal spawning and reset construction use the existing unit constructors.
- Include every combat field in the canonical unit snapshot and state hash.
- Bump `SNAPSHOT_FORMAT_VERSION` and `SIMULATION_VERSION`; the existing handshake therefore rejects older peers.

## Files

- `crates/simulation/src/combat.rs`
- `crates/simulation/src/lib.rs`
- `crates/simulation/src/command.rs`
- `crates/simulation/src/simulation.rs`
- `crates/simulation/src/snapshot.rs`
- `crates/protocol/src/lib.rs`

## Verification

- Constructor tests cover full hull, preserved initial turret angle, no target, and ready cooldown.
- Reset tests mutate every combat field and verify canonical defaults are restored.
- Snapshot tests verify hull, heading, target, and cooldown each change the deterministic hash.
- Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Out of scope

Target acquisition, turret movement, hitscan ray evaluation, hull damage, destruction, rendering, manual targeting, shields, inventory, and weapon variety remain in later tickets.
