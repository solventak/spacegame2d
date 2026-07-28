# SWA-19 — Authoritative Late-Join Snapshot

## Outcome

A client joining an in-progress Relay Operations match installs the server's complete
authoritative state before it begins local simulation. Its first eligible checksum matches the
server, and commands accepted around the join cutover are neither missed nor applied twice.

This implements
[SWA-19](https://linear.app/swarm123/issue/SWA-19/synchronize-late-joining-clients-with-authoritative-world-state).

## Current truth

The repository already has two useful foundations:

- `Simulation::snapshot()` captures deterministic units, structures, objective state, allocator
  state, and the simulation tick for canonical hashing.
- `StateChecksum` detects divergence after a client begins simulating.

The runtime join path does not use that snapshot:

- `ServerHello` contains only configuration, slot, and tick.
- The server marks a client connected immediately after queuing `ServerHello`.
- The client creates a new default `Simulation` and changes only its tick.
- Existing "late join" coverage checks the default layout and ownership, not a moved world.
- `SimulationSnapshot` is capture-only; it cannot validate or restore a simulation.

## Compatibility decision

This changes the shared wire contract and deterministic snapshot definition.
This contract was confirmed for implementation on 2026-07-27.

- Increment `SIMULATION_VERSION` from `17` to `18`.
- Increment `SNAPSHOT_FORMAT_VERSION` from `7` to `8`.
- Add a required `WorldSnapshots` protocol capability.
- Keep hard handshake rejection for clients or servers that do not advertise the capability.
- Do not use Git branches or commit IDs as compatibility gates.

No UI-engine IPC change is required. An invalid snapshot remains a failed connection, with the
specific validation reason recorded in structured client logs.

## Wire contract

Add `InitialWorldState` as a new protocol envelope payload after `ServerHello`.

`InitialWorldState` contains:

- snapshot format and simulation versions;
- the pre-step simulation tick represented by the snapshot;
- world radius and allocator state;
- all static structures and home-objective pairs;
- all live units, including identity, ownership, kinematics, autopilot, hull, and turret state;
- the canonical state hash for verification after restoration; and
- every authoritative command already scheduled for the snapshot tick or later, in server
  scheduling order.

Use typed protobuf enums for controller, structure, objective, hitbox, and combat-target kinds.
Do not transmit Rust debug strings as protocol values.

The existing `ServerHello` remains the first successful response so the client can validate the
simulation frequency, assigned slot, and configured fleet size before accepting the larger state
payload. Its `server_tick` must equal `InitialWorldState.tick`.

The complete encoded frame must remain below `MAX_FRAME_BYTES`. Add a maximum-size configured-world
test rather than introducing chunking in this ticket.

## Snapshot semantics

`SimulationSnapshot.tick` means "the next tick to execute." The server captures it after processing
all connection and command input for the current loop, but before injecting commands for that tick
and calling `Simulation::step()`.

The bootstrap's pending commands therefore include every entry in the server's `scheduled` map
whose `execute_tick >= snapshot.tick`. Flatten the `BTreeMap` in tick order and preserve each
tick's insertion order. Do not re-sort commands with the same execution tick.

Transient presentation state is not authoritative and is not included: shot flashes, destination
markers, rejection messages, and match-result overlays start empty on a late join.

## Implementation

### 1. Complete canonical snapshot coverage

Update `crates/simulation/src/snapshot.rs` so canonical encoding includes every authoritative
snapshot field:

- snapshot and simulation versions;
- allocator state;
- controller kind and autopilot tunables;
- unit active/destination state;
- all combat and objective state already represented by the snapshot; and
- world configuration represented in the snapshot.

Continue omitting fields explicitly derived from other authoritative fields, such as
`core_targetable`.

Add mutation tests proving each encoded authoritative field changes the state hash. This prevents
the bootstrap hash check from silently ignoring restored state.

### 2. Add protocol DTOs and conversions

Update the protobuf schema and `crates/protocol/src/lib.rs` with:

- `Capability::WorldSnapshots`;
- `Message::InitialWorldState`;
- wire DTOs for structures, objectives, units, and their nested state;
- `InitialWorldState::validate_envelope()` for structural checks that belong to the wire type; and
- encode/decode round-trip tests for populated and empty optional fields.

Keep the protocol crate independent of the simulation crate. Conversion from protocol DTOs into
simulation types belongs in the simulation crate.

Reject unknown enum values, absent required nested messages, oversized hashes, and malformed
command payloads as `InvalidData`.

### 3. Make simulation restoration explicit and fallible

Add a `SnapshotRestoreError` using `thiserror` and implement:

```rust
impl TryFrom<spacegame2d_protocol::InitialWorldState> for RestoredSimulation
```

`RestoredSimulation` owns:

- the reconstructed `Simulation`;
- the pending authoritative commands, grouped by `Tick`; and
- the expected snapshot state hash.

Restoration must validate before publishing any state:

- exact snapshot and simulation versions;
- finite, positive radii and finite kinematic/autopilot values;
- valid and unique nonzero unit and structure IDs;
- owner slots within the supported player range;
- allocator state strictly beyond every live unit ID unless exhausted;
- recognized controller, structure, objective, hitbox, and target kinds;
- valid hull/core health ranges and objective counters;
- active autopilots having destinations;
- turret targets referring to a live unit or Command Core;
- objective pairs referring to the correct existing Core and Relay structures;
- nondecreasing pending-command ticks, no commands older than the snapshot, and no duplicate
  `(player_slot, sequence)` identities; and
- snapshot configuration agreeing with `ServerHello`.

Add crate-private restoration constructors where private domain fields require them:

- `Autopilot::restore_arrival(...)`;
- `Unit::restore(...)`;
- `StaticStructure::restore(...)`;
- `HomeObjectivePair::restore(...)`; and
- `World::restore(...)`.

These constructors should accept validated domain values, not raw protobuf messages.

After reconstruction, recompute `Simulation::state_hash()` and compare it to the hash in the
bootstrap. Reject the connection if it differs.

### 4. Introduce an atomic server cutover

Replace the client's handshake boolean with explicit phases:

```text
AwaitingHello -> SyncPending -> Connected
```

Use this server-loop order:

1. Accept sockets and reserve a free slot without treating the peer as connected.
2. Read and validate all `ClientHello` and connected-client command messages.
3. Add newly accepted commands to `scheduled` and broadcast them only to peers already
   `Connected`.
4. For every `SyncPending` peer:
   - register/assign its server-side player slot;
   - capture the simulation snapshot at the current pre-step tick;
   - collect all pending commands at or after that tick;
   - queue `ServerHello`, followed by one `InitialWorldState`;
   - transition the peer to `Connected`.
5. Flush outgoing queues.
6. Apply commands for the current tick and step the authoritative simulation.

Because syncing peers are excluded from step 3, a command accepted in the cutover loop is delivered
to them exactly once inside `InitialWorldState`. Commands accepted in later loops are appended after
the bootstrap as normal live broadcasts.

If snapshot construction or encoding fails, log the failure and close that peer without disturbing
existing clients.

### 5. Bootstrap the client before entering the match

Change `NetworkSession::connect_with_timeout_and_progress` to read both successful handshake
messages while the socket is still blocking and covered by the connection timeout:

1. validate `ServerHello`;
2. require `InitialWorldState`;
3. validate and restore the simulation;
4. install all bootstrap pending commands;
5. switch the socket to nonblocking; and
6. return a bootstrap containing the `NetworkSession`, restored `Simulation`, and scheduled
   command map.

The application must publish the connected UI state and start its local tick timer only after the
bootstrap succeeds.

Replace the current fresh `Simulation::new(...)` plus `set_tick(...)` initialization in
`crates/spacegame2d/src/main.rs` with the restored simulation. Change local player registration so
it updates only the client-side connected-player registry; it must not overwrite authoritative
unit ownership from the snapshot.

Clear transient presentation state when installing the bootstrap.

### 6. Add structured lifecycle logs

Server events:

- `snapshot_captured`: `address`, `slot`, `tick`, unit count, pending-command count, state hash;
- `snapshot_queued`: encoded bytes and the same identity fields; and
- `snapshot_failed`: validation/encoding reason.

Client events:

- `snapshot_received`: `server_tick`, unit count, pending-command count;
- `snapshot_applied`: `local_tick`, state hash; and
- `snapshot_rejected`: structured validation reason.

Do not add per-unit or per-tick logging.

## Tests

### Protocol tests

- Fully populated `InitialWorldState` round-trips through `Message::encode/read`.
- Optional destination and target fields round-trip in both states.
- Unknown enum values, missing nested messages, and invalid hashes are rejected.
- A maximum configured snapshot stays below `MAX_FRAME_BYTES`.
- Old simulation versions and missing `WorldSnapshots` capability are rejected.

### Simulation tests

- `Simulation -> snapshot -> wire -> restore` preserves the tick and exact state hash.
- Round-trip a world after movement, destination changes, combat damage, turret targeting, objective
  progression, unit removal, and allocator advancement.
- Each malformed invariant above returns the expected `SnapshotRestoreError`.
- Restoring a snapshot never partially mutates an existing simulation.

### Network/client tests

- A synthetic server must send `ServerHello` and `InitialWorldState`; either message missing or
  reversed fails the connection.
- The client installs pending commands before its first local step.
- Snapshot hash mismatch fails with an actionable error.
- A successful bootstrap preserves authoritative ownership rather than calling
  `assign_mirror_owners`.

### Real server integration tests

- Connect Player 1, issue a destination command, advance until units move, then connect Player 2.
  Player 2's restored tick and complete state hash match the authoritative snapshot.
- Accept a command in the same server loop as the late join. The new client receives it exactly once
  and both clients match at the next checksum boundary.
- Existing clients continue receiving live commands while the new client synchronizes.
- Disconnect and reuse Player 1's slot while Player 2 remains; the replacement receives the current
  world rather than reset state.
- Invalid/incompatible snapshot handling does not disconnect existing clients.

## Files expected to change

- `crates/protocol/proto/spacegame2d/protocol/v1/protocol.proto`
- `crates/protocol/src/lib.rs`
- `crates/simulation/src/snapshot.rs`
- `crates/simulation/src/autopilot.rs`
- `crates/simulation/src/command.rs`
- `crates/simulation/src/structure.rs`
- `crates/simulation/src/objective.rs`
- `crates/simulation/src/combat.rs`
- `crates/server/src/main.rs`
- `crates/spacegame2d/src/network.rs`
- `crates/spacegame2d/src/main.rs`
- `README.md`
- `docs/QA.md`

Additional focused test modules may be added if the existing `main.rs` test modules become
unwieldy.

No HUD Svelte, UI-engine IPC, renderer, or server-persistence changes are expected.

## Verification

Run:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo tarpaulin --workspace --exclude-files "*/main.rs" --fail-under 85
./scripts/qa-headless.sh
```

Interactive verification:

1. Start a fresh server and Player 1.
2. Move Player 1's fleet and wait until its position visibly changes.
3. Join as Player 2 and confirm both screens show the same fleet positions immediately.
4. Continue issuing commands from both clients and confirm no `state_divergence` events appear.
5. Disconnect Player 1, reconnect a replacement into slot 1, and confirm it receives the same
   in-progress world as Player 2.

## Completion criteria

SWA-19 is complete when a late join and slot reuse both install a validated authoritative bootstrap,
the first shared checksum matches without a reset, commands crossing the join boundary are applied
exactly once, and malformed snapshots fail the joining connection without affecting existing peers.
