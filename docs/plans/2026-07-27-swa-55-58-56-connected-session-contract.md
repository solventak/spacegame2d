# SWA-55, SWA-58, and SWA-56 — Connected Session Contract

## Outcome

On top of `droid/SWA-57-display-name`, implement the authoritative Rust/networking
foundation for a connected match session in two checkpoints:

1. every accepted client receives and retains a validated server-authoritative roster; and
2. every accepted client receives ordered opponent-presence and match-timing state derived from
   the authoritative server simulation tick.

This plan covers:

- [SWA-55](https://linear.app/swarm123/issue/SWA-55/include-the-accepted-player-roster-in-the-session-handshake)
- [SWA-58](https://linear.app/swarm123/issue/SWA-58/publish-authoritative-opponent-presence-changes-to-connected-clients)
- [SWA-56](https://linear.app/swarm123/issue/SWA-56/provide-an-authoritative-elapsed-match-time-source-to-clients)

It deliberately stops at the Rust client model. SWA-60 will expose this model through the
UI-engine bridge, and SWA-59/SWA-48 will render it.

## Execution shape

Use one experiment branch from the current `droid/SWA-57-display-name` HEAD. Do not include or
modify the existing untracked
`docs/plans/2026-07-27-relay-operations-connection-screen.md`.

Suggested branch:

```text
droid/SWA-55-58-56-connected-session-contract
```

Implement and verify the work in exactly two checkpoints:

1. **Checkpoint 1 — authoritative roster (SWA-55)**
2. **Checkpoint 2 — presence and authoritative match timing (SWA-58 + SWA-56)**

Each checkpoint must compile and pass its focused tests before moving to the next. Run the full
repository gate only after Checkpoint 2.

## Current truth

### Protocol

- `SIMULATION_VERSION` is `18`.
- `ClientHello` already carries the canonical display name introduced by SWA-57.
- `ServerHello` carries the assigned `player_slot`, server tick, simulation configuration, and
  capabilities. It does not carry a roster.
- A successful blocking handshake currently reads `ServerHello`, then `InitialWorldState`.
- Live protocol events are authoritative commands and command rejections only.
- Protocol model types and protobuf-generated wire types are intentionally separate. Keep this
  layering.

### Server

- The server supports two slots and assigns the lowest free slot.
- A socket reserves a slot before its `ClientHello` is validated.
- A client becomes `connected` after a valid display name and compatible capabilities are
  accepted.
- Canonical display names are already retained on `Client`.
- Slot 1 owns the blue/cyan fleet and slot 2 owns the orange/coral fleet.
- Disconnect handling removes the client and calls `simulation.world.disconnect_player`.
- The simulation clock runs continuously at `SIMULATION_HZ == 60`, including while zero or one
  clients are connected.
- There is no independent match-session state or match-start anchor.

### Client

- `NetworkSession` retains `player_slot`, `server_tick`, simulation configuration, and bootstrap
  state.
- `poll_events()` handles authoritative commands and rejections.
- The client already advances a deterministic mirror tick. That tick, not wall-clock time, must be
  used to derive elapsed match time.
- UI-engine state does not yet include roster, presence, or clock data. Do not add those fields in
  this work.

## Settled design decisions

### One complete session snapshot

Add one protocol message, `SessionSnapshot`, that represents the complete session view for one
receiving client:

```rust
pub struct SessionSnapshot {
    pub local_player_slot: u32,
    pub participants: Vec<SessionParticipant>,
    pub opponent_presence: OpponentPresence,
    pub presence_revision: u64,
    pub match_timing: MatchTiming,
}

pub struct SessionParticipant {
    pub player_slot: u32,
    pub display_name: String,
    pub color: PlayerColor,
}

pub enum PlayerColor {
    Cyan,
    Coral,
}

pub enum OpponentPresence {
    Waiting,
    Present,
    Disconnected,
}

pub enum MatchTiming {
    Inactive,
    Active { started_at_tick: Tick },
}
```

`SessionSnapshot` is recipient-specific because `local_player_slot` and the presence revision
belong to that connection. Its participant values remain globally consistent.

Use full snapshots for both handshake state and live updates. Do not add independent roster
patches, presence patches, or per-second clock messages. This avoids merge order bugs and gives
SWA-60 a complete typed state to bridge later.

### Roster semantics

- `participants` contains only currently accepted participants, sorted by `player_slot`.
- A newly accepted solo client receives a one-record roster containing itself.
- Once the second client is accepted, both clients receive a two-record roster.
- After an opponent departs, the remaining client's next snapshot returns to a one-record roster.
  The departed identity is not retained as a current participant.
- Slot 1 is `Cyan`; slot 2 is `Coral`.
- The server assigns slot, display name, and color. Clients validate but never derive or normalize
  a replacement value.
- `local_player_slot` must identify exactly one roster record.
- A first client learns the second client's identity through the complete live snapshot sent when
  the second client is accepted.

This resolves the apparent tension between SWA-55's two-player roster and SWA-58's valid solo
accepted state: a snapshot has one current participant while waiting and two while both are
present.

### Presence semantics and revisions

Presence is the receiving client's view of its opponent:

- `Waiting`: this connection has never yet had an accepted opponent.
- `Present`: another accepted participant currently occupies the other slot.
- `Disconnected`: this connection previously had an accepted opponent, and that opponent has
  departed or was lost.

Each accepted connection has its own `presence_revision`:

- its blocking handshake snapshot starts at revision `0`;
- every subsequent opponent transition increments it exactly once;
- live snapshots with a revision less than or equal to the last applied revision are ignored;
- a replacement connection starts a new sequence at revision `0`;
- the surviving connection keeps its existing monotonic sequence across opponent slot reuse.

Validate every decoded snapshot before the stale-revision comparison. A structurally invalid
snapshot is a protocol failure even if its revision would otherwise be stale.

Intentional departure and unexpected transport loss produce the same `Disconnected` opponent
state. Their diagnostic distinction remains in the lower-level connection/session lifecycle and
structured logs; it is not part of `OpponentPresence`.

### Match timing semantics

The server owns:

```rust
match_started_at: Option<Tick>
```

- It is `None` for a solo client before the first two-player match.
- When accepted participant count changes from one to two and no match is active, set it to the
  current pre-step `simulation.tick()`.
- Send that exact anchor to both clients in `MatchTiming::Active`.
- If one participant leaves, preserve the anchor.
- A reconnecting or replacement participant receives the preserved anchor while the surviving
  participant keeps the match active.
- When accepted participant count reaches zero, clear the anchor.
- The next transition from one accepted participant to two creates a new anchor from the then
  current simulation tick.

The client derives:

```rust
elapsed_ticks = local_tick - started_at_tick
elapsed_whole_seconds = elapsed_ticks / SIMULATION_HZ
```

Use saturating `Tick` subtraction. Do not use `Instant`, wall-clock timestamps, a client-side
stopwatch, or per-second network messages.

### Compatibility

This combined experiment is one unreleased protocol feature. Increment `SIMULATION_VERSION` once,
from `18` to `19`, during Checkpoint 1 and keep `19` through Checkpoint 2. Do not increment again
between local checkpoints.

The existing hard version mismatch is sufficient; do not add a capability bit for session
snapshots.

The successful handshake order becomes:

```text
ClientHello
ServerHello
InitialWorldState
SessionSnapshot
```

The client must validate and install all three server messages before returning a successful
`NetworkSession`.

## Wire contract

Add a new `session_snapshot` payload to `Envelope` using a new, never-reused field number. Add
protobuf messages/enums equivalent to:

```proto
message SessionSnapshot {
  uint32 local_player_slot = 1;
  repeated SessionParticipant participants = 2;
  OpponentPresence opponent_presence = 3;
  uint64 presence_revision = 4;
  MatchTiming match_timing = 5;
}

message SessionParticipant {
  uint32 player_slot = 1;
  string display_name = 2;
  PlayerColor color = 3;
}

enum PlayerColor {
  PLAYER_COLOR_UNSPECIFIED = 0;
  PLAYER_COLOR_CYAN = 1;
  PLAYER_COLOR_CORAL = 2;
}

enum OpponentPresence {
  OPPONENT_PRESENCE_UNSPECIFIED = 0;
  OPPONENT_PRESENCE_WAITING = 1;
  OPPONENT_PRESENCE_PRESENT = 2;
  OPPONENT_PRESENCE_DISCONNECTED = 3;
}

message MatchTiming {
  oneof state {
    MatchTimingInactive inactive = 1;
    ActiveMatchTiming active = 2;
  }
}

message MatchTimingInactive {}

message ActiveMatchTiming {
  uint64 started_at_tick = 1;
}
```

Keep `ServerHello.player_slot` for compatibility within version 19 and require it to equal
`SessionSnapshot.local_player_slot`. Removing it provides no value in this change and would make
bootstrap validation less clear.

### Protocol validation

Add `SessionSnapshot::validate()` and call it at every client boundary. Reject with
`io::ErrorKind::InvalidData` when any invariant fails:

- local slot is not `1` or `2`;
- roster length is zero or greater than two;
- participant slots are not `1` or `2`, are duplicated, or are not sorted;
- the local slot is absent or appears more than once;
- a display name fails `DisplayName` validation;
- canonicalizing a wire display name would change it; server-canonical values must arrive exactly;
- slot 1 is not cyan or slot 2 is not coral;
- protobuf enum values are unspecified or unknown;
- `MatchTiming` is absent;
- `Present` does not have exactly two participants;
- `Waiting` or `Disconnected` does not have exactly one local participant;
- `Waiting` is paired with active timing, `Present` is paired with inactive timing, or
  `Disconnected` is paired with inactive timing;
- the initial `SessionSnapshot.local_player_slot` disagrees with `ServerHello.player_slot`.

The protocol crate must remain independent of the simulation crate. It may know that this contract
supports slots 1 and 2, but it must not import `MAX_PLAYERS` or `SIMULATION_HZ`.

## Checkpoint 1 — authoritative roster (SWA-55)

### 1. Add the final session-snapshot protocol types test-first

Files:

- `crates/protocol/proto/spacegame2d/protocol/v1/protocol.proto`
- `crates/protocol/src/lib.rs`

Start with protocol tests for:

1. one-participant and two-participant snapshots round-trip through `Message::encode()` and
   `FrameDecoder`;
2. Unicode canonical names round-trip byte-for-byte;
3. unknown/unspecified color values fail decoding;
4. empty, duplicate, unsorted, out-of-range, or missing-local rosters fail validation;
5. a color/slot mismatch fails validation;
6. a noncanonical wire name such as decomposed Unicode fails validation rather than being silently
   transformed;
7. all three presence states and both timing states round-trip; and
8. absent/unspecified presence or timing state fails validation.

Then:

- add the complete final `PlayerColor`, `SessionParticipant`, `OpponentPresence`, `MatchTiming`,
  and `SessionSnapshot` types shown in the wire contract;
- add `Message::SessionSnapshot`;
- implement explicit wire/domain conversions beside the existing conversions;
- increment `SIMULATION_VERSION` to `19`;
- update every existing protocol/server/client test fixture that constructs a `ServerHello` or
  completes a handshake.

Define the complete schema now so Checkpoint 2 changes behavior, not wire shape. Checkpoint 1 only
asserts the roster acceptance boundary, but its snapshots must still contain valid presence,
revision, and timing values. A solo acceptance is `Waiting`, revision `0`, and `Inactive`; the
second acceptance is `Present` and uses the current tick as the active match anchor. The detailed
transition tests remain in Checkpoint 2.

### 2. Add a testable server match-session model

Files:

- new `crates/server/src/session.rs`
- `crates/server/src/main.rs`

Create a small pure state machine, separate from sockets:

```rust
struct MatchSession {
    participants: BTreeMap<u32, ParticipantState>,
    match_started_at: Option<Tick>,
}

struct ParticipantState {
    display_name: String,
    presence_revision: u64,
    has_seen_opponent: bool,
}

struct SessionDelivery {
    recipient_slot: u32,
    snapshot: SessionSnapshot,
}
```

The exact visibility may remain crate-private. Put color assignment and recipient-specific snapshot
construction on this domain type, not in the TCP loop.

For Checkpoint 1:

- `accept(slot, canonical_name, tick)` records the participant;
- it returns an initial snapshot for the new participant;
- when acceptance changes the roster from one participant to two, it also returns a complete
  updated snapshot for the existing participant;
- the first solo snapshot is waiting/inactive at revision 0;
- the second acceptance starts the match at `tick`, marks both views present, increments the
  existing participant once, and leaves the new participant at revision 0;
- duplicate slot acceptance is an error;
- ordering is always slot 1 then slot 2.

Unit-test the state machine without sockets:

- first acceptance returns one roster record and identifies itself;
- second acceptance returns mutually consistent two-record rosters;
- the two snapshots differ only in recipient-specific local identity/revision fields;
- colors remain stable regardless of connection order or recipient.

### 3. Queue the snapshot atomically during acceptance

File:

- `crates/server/src/main.rs`

After a `ClientHello` has passed version, capability, and display-name validation:

1. retain the exact canonical name;
2. mark the client accepted;
3. register it with `MatchSession`;
4. queue `ServerHello`;
5. queue `InitialWorldState`;
6. queue the new client's initial `SessionSnapshot`;
7. after the mutable client iteration ends, queue any returned update snapshots to already accepted
   recipients.

Do not notify peers about a socket that has merely connected or reserved a slot. Rejected or
malformed handshakes never enter `MatchSession`.

Keep the snapshot/bootstrap cutover consistent: the initial `SessionSnapshot` uses the same
pre-step tick as `ServerHello` and `InitialWorldState`.

Add real TCP tests proving:

- a solo valid handshake returns the three successful server messages in the required order;
- a second accepted client receives both canonical participants;
- the already connected first client receives the same canonical two-participant roster;
- each client identifies itself by `local_player_slot`;
- Player 1 is cyan and Player 2 is coral;
- an invalid name or incompatible client receives no session snapshot;
- both clients agree exactly on `(slot, display_name, color)` records.

Use different names for the two real TCP clients. Existing integration tests currently reuse one
`ClientHello`, which would conceal roster mistakes.

### 4. Install and retain the authoritative roster in the client

File:

- `crates/spacegame2d/src/network.rs`

During blocking connect:

1. read and validate `ServerHello`;
2. read and restore `InitialWorldState`;
3. require and validate `SessionSnapshot`;
4. require local-slot agreement between the hello and session snapshot;
5. only then switch the socket to nonblocking and return success.

Add to `NetworkSession`:

- the latest validated `SessionSnapshot`;
- `session_snapshot(&self) -> &SessionSnapshot`;
- `local_participant(&self) -> &SessionParticipant`.

`local_participant()` may use an invariant-preserving lookup and `expect` because installation
already validates the local record. Do not infer the local participant from name or color.

Extend `ServerEvent` with a session-state variant or update internal state while returning a
notification variant. The application may intentionally do nothing with that notification until
SWA-60, but it must not treat the valid message as an unexpected protocol event.

Add synthetic client tests for:

- successful three-message bootstrap;
- missing, reversed, or wrong-type `SessionSnapshot`;
- hello/session local-slot mismatch;
- malformed roster rejection;
- authoritative roster and local participant retained after connect.

### Checkpoint 1 exit criteria

- SWA-55 acceptance criteria are covered at protocol, server TCP, and client bootstrap layers.
- Focused tests pass:

```bash
cargo test -p spacegame2d-protocol
cargo test -p spacegame2d-server
cargo test -p spacegame2d
```

- The first client learns the second participant without reconnecting.
- No UI protocol, HUD JavaScript/CSS, or simulation behavior has changed.

## Checkpoint 2 — presence and authoritative timing (SWA-58 + SWA-56)

### 1. Harden the final session snapshot contract

Files:

- `crates/protocol/proto/spacegame2d/protocol/v1/protocol.proto`
- `crates/protocol/src/lib.rs`

The final fields and domain types already exist from Checkpoint 1. Complete or tighten
`SessionSnapshot::validate()` with every transition-sensitive cross-field rule in this plan.

Add protocol tests for:

- every presence state round-trip;
- inactive and active timing round-trip with exact `u64` tick precision;
- absent timing state rejection;
- unspecified/unknown presence rejection;
- presence/roster cardinality mismatch rejection;
- maximum `u64` revision and tick preservation.

Do not change `SIMULATION_VERSION` again; version 19 represents the final combined contract.

### 2. Finish `MatchSession` transitions

File:

- `crates/server/src/session.rs`

Implement:

```rust
accept(slot, canonical_name, current_tick) -> Result<Vec<SessionDelivery>, SessionError>
depart(slot) -> Vec<SessionDelivery>
snapshot_for(slot) -> Result<SessionSnapshot, SessionError>
```

Acceptance transition:

1. add the new participant at revision `0`;
2. if this creates two participants:
   - set both views to `Present`;
   - mark both as having seen an opponent;
   - if no match is active, set `match_started_at = Some(current_tick)`;
   - increment existing recipients once;
   - keep the new recipient's initial revision at `0`;
3. emit complete snapshots for all affected recipients.

Departure transition:

1. remove only an accepted participant;
2. if one participant remains:
   - preserve `match_started_at`;
   - set the survivor's view to `Disconnected`;
   - increment the survivor's revision exactly once;
   - emit one complete snapshot with only the survivor in the roster;
3. if zero participants remain:
   - clear `match_started_at`;
   - emit nothing.

Replacement transition:

- accepting a participant into the free slot while one survivor remains restores both views to
  `Present`;
- preserve the old match anchor;
- increment the survivor once;
- start the replacement at revision `0`;
- include only the replacement's current name, never the departed participant's name.

Pure state-machine tests must cover:

- solo `Waiting`, revision 0, inactive timing;
- second join, `Present` for both, same exact start tick;
- one departure, survivor `Disconnected`, revision increment, active anchor preserved;
- replacement join, both `Present`, survivor increments, replacement starts at 0, anchor preserved;
- replacement name does not leak the departed name;
- final departure clears the anchor;
- a new one-then-two sequence gets a later new anchor;
- duplicate or unknown slot transitions cannot corrupt state.

### 3. Route join and loss transitions through one server path

File:

- `crates/server/src/main.rs`

Refactor removal so read EOF, read error, write/flush failure, and any future intentional close all
call the same accepted-participant departure path exactly once.

Important details:

- collect removals by stable client identity/slot, or sort and deduplicate indices before removal;
- do not call `depart` for a rejected pre-handshake socket;
- do not leave a flush-failed client occupying a slot for another loop;
- queue returned snapshots only to clients still accepted after all removals are applied;
- preserve lower-level diagnostic logging for why a transport ended;
- do not expose that diagnostic as a new presence enum.

On the second accepted participant, pass the current pre-step `simulation.tick()` to
`MatchSession::accept`. Queue all resulting snapshots before stepping the simulation.

Add structured state-transition logs, without per-tick noise:

- `session_participant_accepted`: slot, canonical display name, revision;
- `opponent_presence_changed`: recipient slot, presence, revision;
- `match_started`: `started_at_tick`;
- `session_participant_departed`: slot, transport/lifecycle reason;
- `match_ended`: tick at which the final participant left.

Real TCP integration tests must cover:

- solo handshake reports `Waiting`, revision 0, inactive timing;
- second join updates the first client live and gives both the same start anchor;
- clean EOF/drop updates the survivor to `Disconnected`;
- a write/read failure uses the same public presence state;
- stale participant data is absent after departure;
- replacement/slot reuse reports the replacement's name and preserves the active anchor;
- after all clients leave, a new solo client is inactive and waiting;
- the next second client receives a new start anchor;
- existing command/snapshot broadcasts still work after session-state messages are introduced.

### 4. Apply ordered snapshots and expose elapsed time

File:

- `crates/spacegame2d/src/network.rs`

When `poll_events()` decodes a live `SessionSnapshot`:

1. validate the complete snapshot;
2. require its `local_player_slot` to match the established connection;
3. compare `presence_revision` with the installed snapshot;
4. ignore revisions less than or equal to the installed revision;
5. atomically replace the installed snapshot for a newer revision;
6. return a typed `ServerEvent::SessionStateChanged` notification.

Malformed newer or stale snapshots are protocol errors and fail the session. A valid stale snapshot
is ignored and does not emit an event.

Add accessors:

```rust
pub fn opponent_presence(&self) -> OpponentPresence;
pub fn match_started_at(&self) -> Option<Tick>;
pub fn elapsed_match_ticks(&self) -> Option<Tick>;
pub fn elapsed_match_seconds(&self) -> Option<u64>;
```

`elapsed_match_ticks()` uses the existing `local_tick` maintained by the app. Seconds are integer
division by `u64::from(SIMULATION_HZ)`, which rounds down exactly as required.

Client tests must cover:

- a newer revision replaces the snapshot and emits one event;
- duplicate and lower revisions are ignored;
- an invalid stale snapshot still fails validation;
- a local-slot change is rejected;
- elapsed time is `None` while inactive;
- elapsed ticks and whole seconds derive from `local_tick - started_at_tick`;
- 59 ticks reports 0 seconds, 60 reports 1, and 119 reports 1;
- local tick behind the anchor saturates at zero;
- disconnect and replacement snapshots preserve the original active anchor.

### 5. Keep the application compatible without implementing SWA-60

File:

- `crates/spacegame2d/src/main.rs`

Handle `ServerEvent::SessionStateChanged` exhaustively. Retain the model inside
`NetworkSession`; a trace at state-transition level is sufficient for this checkpoint.

Do not:

- add roster/presence/clock fields to `spacegame2d-ui-protocol`;
- change `UI_ENGINE_PROTOCOL_VERSION`;
- add HUD components, waiting-screen behavior, or clock rendering;
- start a wall-clock timer in `App`;
- delay the simulation/render loop while waiting for an opponent.

SWA-60 will read `NetworkSession::session_snapshot()` and publish a complete UI-engine snapshot.

### Checkpoint 2 exit criteria

- SWA-58 and SWA-56 server/client-source acceptance criteria are covered.
- Two clients receive mutually consistent current roster, presence, and exact match anchor.
- Stale revisions cannot roll client state backward.
- Slot reuse cannot expose an old display name.
- Elapsed seconds are derived from authoritative simulation ticks.
- No per-second network traffic exists.

## Acceptance matrix

| Scenario | Expected authoritative result | Primary test layer |
|---|---|---|
| First valid client joins | One participant, local slot matches, waiting, revision 0, inactive | Server TCP + client bootstrap |
| Second valid client joins | Both rosters identical, both present, same start tick | Session unit + server TCP |
| Canonical Unicode names | Exact server values on both clients | Protocol + server TCP |
| Invalid roster/color/name | Client fails with `InvalidData` | Protocol + client synthetic server |
| Opponent departs | Survivor disconnected, one-record roster, revision +1 | Session unit + server TCP |
| Stale update arrives | Valid stale snapshot ignored | Client network unit |
| Malformed stale update arrives | Session fails as protocol error | Client network unit |
| Slot is reused | Current replacement identity only; survivor revision increases | Session unit + server TCP |
| One player remains | Match anchor continues unchanged | Session unit + client network |
| Both players leave | Server clears active match | Session unit + server TCP |
| Next match starts | New start anchor at current server tick | Session unit + server TCP |
| Client displays source time | Whole seconds floor from local simulation tick | Client network unit |
| Existing command flow | Commands and checksums still route normally | Existing real TCP regression |

## Files expected to change

- `crates/protocol/proto/spacegame2d/protocol/v1/protocol.proto`
- `crates/protocol/src/lib.rs`
- new `crates/server/src/session.rs`
- `crates/server/src/main.rs`
- `crates/spacegame2d/src/network.rs`
- `crates/spacegame2d/src/main.rs`
- `README.md` if its handshake description is touched; avoid hard-coded version prose
- this plan document

## Files expected not to change

- `crates/simulation/**` — use the existing `Tick` and fixed 60 Hz simulation
- `crates/ui-protocol/**` — SWA-60 owns the UI bridge contract
- `crates/spacegame2d/hud/**` — SWA-59/SWA-48 own presentation
- `ui/Fleet Design System/**`
- the existing untracked connection-screen plan

If implementation reveals that one of these exclusions must change, stop and explain the newly
discovered dependency before expanding scope.

## Risks and controls

### Borrowing and transition ordering in the server loop

Do not mutate peer clients while iterating one mutable client. Have `MatchSession` return
`SessionDelivery` values, then route those deliveries after the input/removal pass.

### Duplicate removal

The current loop can discover removal through read and flush paths. Deduplicate removal targets and
call `MatchSession::depart` once per accepted slot.

### First-client roster freshness

SWA-55 is incomplete if only the joining second client receives the two-player roster. The existing
first client must receive a live complete snapshot in the same acceptance transition.

### Handshake/live-message boundary

The new client's initial `SessionSnapshot` is part of blocking bootstrap. Updates after that are
live messages. Do not allow the initial snapshot to race into the nonblocking event path.

### Tick off-by-one

Capture the match start at the current pre-step simulation tick, before `Simulation::step()`.
Tests should assert the exact transmitted tick, not only approximate elapsed seconds.

### TCP ordering does not remove revision requirements

TCP is ordered, but revision filtering is still required by SWA-58 and protects future transport or
bridge changes. Implement and test it explicitly.

### Ticket boundary

The Rust client source can calculate whole elapsed seconds, but visible UI rendering belongs to
SWA-60/SWA-48. Do not pull those tickets into this experiment.

## Verification

Run focused tests throughout:

```bash
cargo test -p spacegame2d-protocol
cargo test -p spacegame2d-server
cargo test -p spacegame2d
```

Before handing off or opening a PR, run the repository gate:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Because this plan does not change native HUD lifecycle code or HUD assets, `qa-hud.sh` is not
required. If implementation expands into those files, run `./scripts/qa-hud.sh`.

## Definition of done

- `SIMULATION_VERSION` is 19 and all fixtures use it.
- The successful handshake requires `ServerHello`, `InitialWorldState`, and `SessionSnapshot`.
- Both accepted clients retain the same canonical current participant records.
- Client local identity comes from `local_player_slot`, never name or color inference.
- Presence transitions are complete, ordered, revisioned, and stale-safe.
- Slot reuse starts a new recipient revision sequence and never leaks the previous name.
- Match timing starts on the first two-player transition, survives one departure/replacement, and
  resets only after all participants leave.
- Client elapsed time uses simulation ticks and integer floor division at 60 Hz.
- Existing command, snapshot, checksum, and disconnect behavior remains green.
- No UI bridge or HUD scope is included.
- The full Rust test gate passes.
