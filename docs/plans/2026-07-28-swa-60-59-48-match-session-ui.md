# SWA-60, SWA-59, and SWA-48 — Connected Match Session UI

## Outcome

On top of commit `4ca34ca` on
`droid/SWA-55-58-56-connected-session-contract`, expose the authoritative match-session model
through the typed UI bridge, keep a solo accepted player on the existing main menu, and transition
both accepted players into a compact live in-game HUD without recreating the game window or its
embedded WebView.

This plan covers:

- [SWA-60](https://linear.app/swarm123/issue/SWA-60/expose-match-session-roster-presence-and-clock-state-through-the-ui)
- [SWA-59](https://linear.app/swarm123/issue/SWA-59/keep-a-solo-accepted-player-at-the-main-menu-while-waiting-for-an)
- [SWA-48](https://linear.app/swarm123/issue/SWA-48/show-live-player-status-in-a-compact-in-game-hud)

The dependency order is strict:

1. **Checkpoint 1 — SWA-60: typed match-session UI contract**
2. **Checkpoint 2 — SWA-59: authoritative waiting lifecycle**
3. **Checkpoint 3 — SWA-48: join transition and persistent HUD**

Each checkpoint must compile and pass its focused tests before the next begins. The three tickets
can share one experiment branch because SWA-59 and SWA-48 are direct consumers of SWA-60, but the
commits should remain checkpoint-shaped so a failure can be bisected.

Suggested branch from the current HEAD:

```text
droid/SWA-60-59-48-match-session-ui
```

Do not include or modify the existing untracked
`docs/plans/2026-07-27-relay-operations-connection-screen.md`.

## Current truth

### Authoritative session model

The preceding SWA-55/SWA-58/SWA-56 checkpoint already provides:

- a server-authoritative `SessionSnapshot`;
- canonical participants ordered by player slot;
- recipient-relative `Waiting`, `Present`, and `Disconnected` opponent presence;
- a monotonic `presence_revision`;
- an optional authoritative `started_at_tick`;
- client accessors for local participant, opponent presence, match anchor, elapsed ticks, and
  elapsed whole seconds; and
- validation and stale-revision rejection inside `NetworkSession`.

The server deliberately removes a departed participant from the current roster. The presentation
bridge must therefore retain the last accepted opponent identity for the active match so the
persistent HUD can still identify a disconnected opponent. That cache is presentation state, not
a second authoritative roster.

### UI-engine bridge

- `UI_ENGINE_PROTOCOL_VERSION` is `2`.
- The bridge currently transports connection lifecycle state, protocol errors, and heartbeats.
- `ConnectionStateSnapshot::Connected` contains the local slot/color and the local display name,
  but no opponent, presence revision, or match clock.
- the checked-in TypeScript contract is maintained beside the Rust schema.
- `bridge.ts` is the JSON adapter and must reject structurally incomplete inbound messages.
- `UiReady` currently triggers only a connection-state publication.
- There is no connected-session disconnect command.

### Native WebView and game loop

- One transparent child WebView is created for the lifetime of the game window.
- The WebView is full-window before connection and immediately shrinks to `224 × 88` when the
  transport becomes connected.
- Child WebView bounds, rather than CSS pointer events, determine which part of the game window
  receives gameplay input.
- The client simulation must continue stepping while connected, even if only one player is
  present, so its deterministic mirror stays synchronized with the server.
- Right-click movement and `R` reset are currently gated only by transport connection.
- Session-state events are applied to `NetworkSession` but only logged by `main.rs`.

### Svelte UI

- `App.svelte` renders the current Relay Operations main menu for disconnected/connecting states.
- A connected transport immediately renders the temporary `LOCAL COMMAND` panel.
- There is no solo waiting state, join overlay, persistent two-player HUD, live timer, or connected
  disconnect control.

## Settled cross-ticket behavior

### Transport acceptance is not match acceptance

Treat these as separate transitions:

```text
transport connected + clock inactive
    -> full-window main menu / "Waiting for opponent…"

clock becomes active + opponent present
    -> centered "Match accepted" overlay
    -> hold 700 ms
    -> dock over approximately 600 ms
    -> persistent compact HUD
```

The SWA-48 join overlay starts when the authoritative match state first becomes active, not merely
when the TCP handshake succeeds. This preserves SWA-59 for the first solo player. A second player
who connects to an already waiting player receives an active initial snapshot and enters the join
overlay immediately after its handshake.

### Simulation continues; gameplay availability is gated

Do not stop the simulation tick while waiting. Stopping it would desynchronize the deterministic
client mirror.

Instead:

- continue applying authoritative commands, stepping the simulation, and sending checksums while
  transport-connected;
- define gameplay as available only after `MatchTiming::Active`;
- reject/ignore local movement, reset, camera-drag start, and future gameplay commands while the
  clock is inactive; and
- keep the full-window main menu over the render surface while waiting.

### Departure behavior depends on whether a match ever started

- Before a match starts, the only valid accepted state is `Waiting`; remain on the main menu.
- Once a match starts, its clock anchor remains active across an opponent departure.
- After that departure, keep the compact HUD visible, keep the timer running from the same
  authoritative anchor, retain the last opponent identity, and show the opponent as disconnected.
- If a replacement joins, replace the cached opponent identity with the new authoritative roster
  record and update the HUD in place. Do not replay the join animation for the surviving player.
- A replacement client joining an already active match does receive its own join animation.

This satisfies SWA-59's session lifecycle wording and SWA-48's more specific requirement that a
post-start opponent departure must not return the surviving player to a full-screen view.

### Complete snapshots, not patches

Every match-session bridge event is a complete presentation snapshot. The browser never merges
roster, presence, or clock patches.

Use two independent order values:

- `sequence`: monotonic for every bridge-side match-session publication, including clock refreshes
  and resets; the UI ignores a sequence less than or equal to the last accepted sequence.
- `presenceRevision`: copied unchanged from the server-authoritative session snapshot and used only
  to describe opponent-presence ordering.

The bridge sequence is scoped to one `bridgeId`. A newly loaded UI can start from the presenter's
current sequence because the new bridge has no previously accepted value.

### The UI displays authoritative elapsed time

The Rust client derives elapsed time from:

```text
simulation tick - authoritative started-at tick
```

Publish a complete active snapshot when:

- the initial session snapshot is installed;
- an ordered session-state change is accepted;
- the displayed elapsed whole second changes; or
- the bridge becomes ready and needs its initial complete state.

Do not run an authoritative JavaScript stopwatch. The UI renders the
`elapsedWholeSeconds` supplied by Rust. Publishing once per displayed second is a local UI refresh,
not a new server timer or network protocol event.

### Explicit reset boundaries

Publish a typed reset and clear all presentation caches:

- at process/UI startup;
- before a new connection attempt;
- on intentional disconnect;
- on unexpected session loss; and
- before replacing an established connected resource set.

The reset must clear local identity, opponent identity, presence, clock, and animation phase in the
Svelte model. Retain the last accepted `sequence` for the current `bridgeId` so a delayed pre-reset
message cannot repopulate stale state; only a newly loaded bridge starts with no accepted sequence.
A new complete session snapshot must arrive before the connected presentation is allowed to
render.

## Checkpoint 1 — SWA-60: typed match-session UI contract

### 1. Add the final Rust/JSON contract

Files:

- `crates/ui-protocol/src/lib.rs`
- `crates/ui-protocol/schema/ui-engine-ipc.v1.schema.json`
- `crates/spacegame2d/hud/src/generated/ui-engine-ipc.ts`
- `crates/spacegame2d/hud/src/bridge.ts`
- `crates/spacegame2d/hud/src/model.ts`

Increment `UI_ENGINE_PROTOCOL_VERSION` once, from `2` to `3`. Do not rename the existing
`ui-engine-ipc.v1.schema.json` artifact in this combined change; its filename is the schema bundle
generation, while `protocolVersion` remains the runtime compatibility boundary.

Add presentation DTOs equivalent to:

```rust
pub struct MatchParticipantHudModel {
    pub player_slot: u32,
    pub display_name: String,
    pub color: PlayerColor,
    pub color_hex: String,
}

pub struct MatchClockHudModel {
    pub started_at_tick: u64,
    pub current_tick: u64,
    pub ticks_per_second: u32,
    pub elapsed_whole_seconds: u64,
}

pub enum MatchSessionResetReason {
    Startup,
    NewConnectionAttempt,
    UserDisconnected,
    SessionLost,
}

#[serde(tag = "stage", rename_all = "camelCase")]
pub enum MatchSessionState {
    Reset {
        sequence: u64,
        reason: MatchSessionResetReason,
    },
    Waiting {
        sequence: u64,
        local_player: MatchParticipantHudModel,
        opponent_presence: OpponentPresence,
        presence_revision: u64,
    },
    Active {
        sequence: u64,
        local_player: MatchParticipantHudModel,
        opponent_player: MatchParticipantHudModel,
        opponent_presence: OpponentPresence,
        presence_revision: u64,
        clock: MatchClockHudModel,
    },
}
```

Use the existing cyan/coral semantic colors:

- slot 1 / cyan: `#22CFE8`;
- slot 2 / coral: `#FF6A47`.

Do not let consumers supply or derive a different color. The adapter validates that the
authoritative slot, color enum, and presentation hex agree.

Enforce variant invariants when constructing the presentation state:

- `Waiting` has exactly one authoritative local participant, `Waiting` presence, and no active
  clock;
- `Active` has a local identity, a current or last-known opponent identity, an active clock, and
  presence `Present` or `Disconnected`;
- `Present` always uses the current authoritative opponent record;
- `Disconnected` uses only the last opponent accepted during the same active session;
- reset has no participant, presence, or clock fields;
- ticks and revisions are finite non-negative integers at the JSON boundary; and
- all required fields are present and unknown enum values are rejected.

Add:

```rust
EngineToUiMessage::MatchSessionStateChanged {
    protocol_version,
    bridge_id,
    state: MatchSessionState,
}
```

Add:

```rust
UiToEngineMessage::DisconnectRequested {
    protocol_version,
    bridge_id,
    request_id,
}
```

The connected `requestId` scopes intentional disconnect to the current connection. Reject a stale
or mismatched request rather than disconnecting whichever session happens to be current.

Update all Rust version matching, wrong-direction classification, schema snapshots, the checked-in
TypeScript union, inbound/outbound kind sets, and runtime validators. The TypeScript adapter must
validate the complete nested shape rather than accepting an object based only on its `stage`.

### 2. Add a pure match-session presentation adapter

Files:

- new `crates/spacegame2d/src/match_session.rs`
- `crates/spacegame2d/src/main.rs`

Create a small pure state owner rather than scattering cache rules through the event loop:

```rust
struct MatchSessionPresenter {
    sequence: u64,
    last_opponent: Option<MatchParticipantHudModel>,
    last_elapsed_whole_seconds: Option<u64>,
    state: MatchSessionState,
}
```

Put these behaviors on that type:

- initialize as `Reset { reason: Startup }`;
- map a validated `NetworkSession` snapshot into `Waiting` or `Active`;
- identify local/opponent records by authoritative slot;
- update `last_opponent` only from a current authoritative opponent;
- retain it only for `Active + Disconnected`;
- replace it when a replacement opponent becomes present;
- clear it on every reset;
- build the match clock from `NetworkSession` and `SIMULATION_HZ`;
- increment `sequence` for every emitted complete state;
- suppress duplicate per-tick publication until `elapsed_whole_seconds` changes; and
- expose `current_state()` so `UiReady` can receive a complete initial snapshot.

Use `TryFrom` at the protocol-to-presentation boundary when mapping can fail. A structurally
impossible session snapshot must produce a typed/logged bridge-adapter failure, not a partial UI
state.

### 3. Publish state at every lifecycle boundary

Files:

- `crates/spacegame2d/src/main.rs`
- `crates/spacegame2d/src/session.rs`

Split the current `publish_state` responsibility into explicit helpers:

- publish the current connection state;
- publish the current complete match-session state;
- publish both initial states after `UiReady`; and
- reset then publish match state before publishing a disconnected/new-attempt connection state.

Publish the first `Waiting` or `Active` state only after the handshake's validated session snapshot
and initial simulation have both been installed. This prevents the browser from rendering a
connected session with placeholder identities.

When `ServerEvent::SessionStateChanged` is received, the `NetworkSession` has already accepted and
installed the ordered snapshot. Rebuild and publish the complete presentation state immediately.

After simulation stepping, compare the current elapsed whole second with the presenter's last
published value and publish only when it changes. Include this next second boundary in the Winit
control-flow deadline if necessary so a static scene still refreshes its clock.

### 4. Implement intentional disconnect

Files:

- `crates/spacegame2d/src/session.rs`
- `crates/spacegame2d/src/main.rs`
- `crates/spacegame2d/hud/src/App.svelte`

Add `SessionLifecycle::disconnect(&RequestId)` for the `Connected` phase:

- accept only the current request ID;
- transition to idle with a new `DisconnectedReason::UserDisconnected`;
- drop `NetworkSession` through the existing connected-resource reset;
- reset match presentation with `UserDisconnected`;
- publish the match reset before the idle connection state; and
- leave the process and game window running.

The network-session drop closes the TCP connection and lets existing server departure handling
notify the opponent. Do not add a second wire-level disconnect protocol in this ticket.

Apply the same match reset and publication ordering to unexpected network loss, connection
replacement, and a new connection attempt.

### 5. Checkpoint 1 tests

Rust contract tests:

- all reset/waiting/active states serialize with protocol version `3`;
- disconnect request validates its bridge/request IDs and rejects stale protocol versions,
  missing fields, unknown fields, and wrong-direction kinds;
- waiting rejects an absent local player, active rejects an absent opponent or clock, and invalid
  color/slot combinations cannot be constructed by the adapter;
- schema generation reports protocol version `3`.

Presenter tests:

- initial solo snapshot becomes a complete waiting state;
- initial two-player snapshot becomes a complete active state;
- present-to-disconnected retains the last opponent identity;
- a replacement present snapshot replaces that identity;
- reset clears both identities, presence, clock, and elapsed-second cache;
- bridge sequences increase across presence, clock, and reset updates;
- presence revisions are preserved rather than synthesized;
- repeated calls in one elapsed second do not emit duplicates; and
- elapsed seconds come from simulation ticks and the authoritative anchor.

Lifecycle tests:

- matching connected request can disconnect;
- stale request cannot disconnect;
- user disconnect reports the new idle reason; and
- connect/session-loss paths produce the required reset reasons.

TypeScript adapter tests:

- complete reset, waiting, and active messages are accepted;
- partial nested payloads, bad variants, unknown colors/presence, non-integer sequence/tick values,
  and absent clock/opponent fields are rejected;
- `disconnectRequested` is accepted only with valid current IDs; and
- protocol version `2` is rejected.

## Checkpoint 2 — SWA-59: authoritative waiting lifecycle

### 1. Make waiting a first-class Svelte view

Files:

- `crates/spacegame2d/hud/src/App.svelte`
- `crates/spacegame2d/hud/src/app.css`

Replace the current `state.stage === 'connected'` shortcut with a combined connection/session
view model:

- disconnected/connecting/failed: existing connection UI;
- connected + no complete session snapshot yet: fail closed to the full-window shell;
- connected + `MatchSessionState::Waiting`: waiting main menu;
- connected + `MatchSessionState::Active`: join/persistent HUD flow.

The waiting view reuses the existing Relay Operations full-window shell and does not introduce a
lobby. Replace the connection form area with a compact accepted-session panel that shows:

- the exact required copy `Waiting for opponent…`;
- local display name and assigned color;
- a neutral/active link status;
- a visible `DISCONNECT` action; and
- no in-match timer or gameplay-active wording.

Keep the exact sentence-case copy despite the design system's normal all-caps instrument labels.
Ticket copy wins.

Do not poll. The same mounted Svelte component transitions automatically when the next complete
match-session message arrives.

### 2. Gate gameplay commands on authoritative match start

Files:

- `crates/spacegame2d/src/main.rs`
- optionally `crates/spacegame2d/src/match_session.rs`

Add one testable predicate:

```rust
gameplay_available = network.match_started_at().is_some()
```

Use it for local gameplay interactions, including:

- right-click destination commands;
- `R` simulation reset;
- middle-button camera-drag start; and
- any command entry point added while this work is in flight.

Do not use `opponent_presence == Present` as the gate. The active clock intentionally survives an
opponent departure, and SWA-48 requires the surviving player to remain in the in-match
presentation.

Continue simulation stepping/checksums whenever the transport lifecycle is connected.

### 3. Drive native layout from match presentation, not connection state

Files:

- `crates/spacegame2d/src/hud.rs`
- `crates/spacegame2d/src/main.rs`

Remove `HudLayout::for_state(ConnectionStateSnapshot)`. A connected transport alone must no longer
shrink the WebView.

Use explicit phases:

```rust
enum HudPhase {
    Pregame,
    Waiting,
    JoinAccepted,
    Docking,
    Compact,
}
```

Both `Pregame` and `Waiting` use full-window bounds. `Active` begins `JoinAccepted`. Reset,
intentional disconnect, and session loss return to `Pregame`.

The WebView instance remains unchanged. Only its bounds and the Svelte branch change.

### 4. Checkpoint 2 tests

Svelte tests:

- transport connected plus no session snapshot does not show the compact HUD;
- a waiting snapshot renders the exact `Waiting for opponent…` copy;
- the accepted local name/color are shown;
- the waiting screen exposes Disconnect and emits the request-scoped command;
- a live active snapshot transitions without reload, polling, reconnect, or remount; and
- reset clears waiting data and returns to the connection shell.

Rust tests:

- connected + inactive clock is not gameplay-available;
- active clock is gameplay-available for both present and disconnected opponent states;
- waiting remains full-window at multiple DPI/window sizes; and
- a stale disconnect request leaves the active session untouched.

## Checkpoint 3 — SWA-48: join transition and persistent HUD

### 1. Implement a bounded native join-to-dock transition

Files:

- `crates/spacegame2d/src/hud.rs`
- `crates/spacegame2d/src/main.rs`

A full-window transparent child WebView can intercept the entire game window even when most CSS is
transparent. Therefore, as soon as an active match snapshot arrives:

1. resize the existing WebView from full-window to a centered bounded join card;
2. hold that centered bound for `700 ms`;
3. interpolate the WebView bounds to the top compact HUD over approximately `600 ms`; and
4. retain only compact bounded WebView input after docking.

Suggested logical bounds:

- join card: `520 × 180`, centered in the parent;
- compact bar: up to `760 × 72`, horizontally centered, `14 px` from the top;
- clamp both dimensions and origins for small windows and all scale factors.

Use the Fleet motion curve `cubic-bezier(.2, .6, .3, 1)` for the docking interpolation. Implement
the easing/bounds interpolation as pure functions with deterministic tests. Do not add bounce,
spring, scale, or looping motion.

The native HUD phase owns the transition start `Instant`, next deadline, and bounds. During the
600 ms dock, schedule short Winit deadlines (approximately one frame) so bounds continue changing
without busy-waiting. At completion, settle exactly on compact bounds.

Starting the bounded join card before evaluating the active-state script ensures gameplay input
outside the visible overlay is not captured by an invisible full-window child.

### 2. Render the join overlay

Files:

- `crates/spacegame2d/hud/src/App.svelte`
- `crates/spacegame2d/hud/src/app.css`

When the first active state is accepted for this local connection, render:

- centered heading `Match accepted`;
- local color and display name;
- opponent color and display name;
- current opponent presence;
- authoritative formatted elapsed match time; and
- the Disconnect action.

Hold the join presentation for `700 ms`, then switch its internal Svelte phase to docking for
`600 ms`, synchronized from the same active-state receipt as the native bounds transition.
Afterward settle in the persistent status bar.

Use fake timers in tests. Cancel pending timers on reset/unmount and guard them with the current
match sequence so a stale timer cannot dock a new session.

### 3. Render the persistent top status

The docked HUD always shows:

- both player display names;
- cyan/coral identity signals from the typed model;
- opponent status (`PRESENT` or `DISCONNECTED`);
- authoritative elapsed match time; and
- `DISCONNECT`.

Format elapsed time from `elapsedWholeSeconds`:

- `MM:SS` below one hour;
- `H:MM:SS` at one hour or greater.

Do not increment it locally between Rust snapshots.

On an active update:

- replace all displayed values from the complete snapshot;
- update the timer in place;
- update departure/replacement identity and presence in place;
- never recreate the game window or WebView;
- never return to full-window because the opponent disconnected; and
- never replay the join animation merely because a replacement appeared.

### 4. Apply the Fleet Design System

Canonical references:

- `ui/Fleet Design System/readme.md`
- `ui/Fleet Design System/styles.css`
- `ui/Fleet Design System/ui_kits/hud/README.md`
- `ui/Fleet Design System/ui_kits/hud/HudChrome.jsx`
- `ui/Fleet Design System/ui_kits/hud/motion.css`

Preserve:

- cyan `#22CFE8` for friendly/local identity;
- coral `#FF6A47` for enemy/opponent identity;
- gray for neutral or disconnected signals;
- translucent graphite panels, subtle blur, and `1 px` hairlines;
- mono numeric timer/readouts;
- uppercase instrument labels except ticket-mandated copy;
- near-square radii and no shadows; and
- the `14 px` top edge inset.

Use the local Lucide-style icon system if an icon is added. Do not use emoji or ad-hoc SVG art.
Reduced-motion behavior is explicitly deferred by SWA-48 and is not part of this change.

### 5. Checkpoint 3 tests

Native HUD tests:

- active entry begins at a centered, DPI-aware card bound;
- bounds remain centered through `699 ms`;
- docking begins at `700 ms`;
- interpolation is monotonic and clamped;
- compact bounds are exact at `1300 ms`;
- parent resize/scale-factor changes recompute each endpoint safely;
- reset during either phase returns immediately to full-window pregame;
- opponent departure while compact leaves the phase compact; and
- a replacement update does not restart the animation.

Svelte tests with fake timers:

- active entry shows `Match accepted`, both names/colors, presence, clock, and Disconnect;
- the overlay remains through the 700 ms hold;
- it enters docking at the hold boundary and compact mode after the 600 ms transition;
- timer-only complete snapshots update the displayed value without restarting animation;
- stale/lower sequences are ignored;
- opponent departure retains both names and changes presence to disconnected;
- replacement presence replaces the opponent identity without remount/reload;
- deliberate reset cancels timers, clears old data, and returns to the main menu; and
- the Disconnect control remains available in join and compact phases.

## Expected file map

| File | Responsibility |
| --- | --- |
| `crates/ui-protocol/src/lib.rs` | Protocol v3 DTOs, message variants, validation |
| `crates/ui-protocol/schema/ui-engine-ipc.v1.schema.json` | Regenerated checked-in schema |
| `crates/spacegame2d/src/match_session.rs` | Pure authoritative-to-presentation adapter/cache |
| `crates/spacegame2d/src/session.rs` | Request-scoped intentional disconnect lifecycle |
| `crates/spacegame2d/src/main.rs` | Publication/reset ordering, tick refresh, input gating |
| `crates/spacegame2d/src/hud.rs` | Waiting/join/docking/compact native bounds state machine |
| `crates/spacegame2d/hud/src/generated/ui-engine-ipc.ts` | Checked-in protocol v3 TypeScript union |
| `crates/spacegame2d/hud/src/bridge.ts` | Strict complete-message JSON validation |
| `crates/spacegame2d/hud/src/model.ts` | UI-facing typed helpers/formatters if needed |
| `crates/spacegame2d/hud/src/App.svelte` | Waiting, join, compact, reset, disconnect rendering |
| `crates/spacegame2d/hud/src/app.css` | Fleet-compliant responsive presentation and motion |
| `crates/spacegame2d/hud/tests/App.test.ts` | IPC, waiting, animation, departure, reset tests |
| `crates/spacegame2d/hud/dist/**` | Rebuilt embedded WebView bundle |

Avoid unrelated server, simulation, protobuf, renderer, or design-system source changes.

## Acceptance-criteria traceability

### SWA-60

- Typed local/opponent identity, assigned color, presence revision, and time source:
  `MatchSessionState`.
- Complete current snapshots rather than patches: one full state per bridge event.
- Live updates without window recreation: existing WebView plus message publication.
- Initial state before presentation: connection + match state published on `UiReady`, fail-closed
  Svelte branch until complete.
- Ordered updates: bridge `sequence` plus authoritative `presenceRevision`.
- Explicit stale-state clearing: typed resets at startup/new attempt/disconnect/loss.
- Invalid data handling: pure Rust adapter invariants and strict TypeScript JSON adapter tests.
- Versioned contract: UI-engine protocol `2 -> 3`, regenerated schema and checked-in TS.

### SWA-59

- Solo accepted player stays on existing main menu: connected + inactive timing maps to full-window
  waiting.
- Exact copy: `Waiting for opponent…`.
- No active gameplay before opponent: local inputs gated on active match timing.
- Disconnect while waiting: request-scoped `disconnectRequested`.
- Automatic transition: ordered active snapshot drives the same mounted UI/WebView.
- Departure lifecycle: pre-start remains waiting; post-start remains compact/disconnected.
- No lobby expansion: reuse existing menu shell only.

### SWA-48

- Successful match acceptance transitions to transparent compact HUD: active timing triggers
  bounded join-to-dock phases.
- Centered join overlay with required data: active complete snapshot.
- 700 ms hold and approximately 600 ms dock: native and Svelte phase timers.
- Persistent top status: both identities, colors, presence, time, Disconnect.
- Opponent departure stays visible: last-known identity plus active/disconnected complete state.
- Non-blocking gameplay: child WebView bounds never exceed the visible join/compact surface after
  match acceptance.
- Existing overlay foundation: one Wry child WebView, no game-window or WebView recreation.
- Manual two-client coverage: verification script below.

## Verification sequence

At each checkpoint, run focused tests first:

```sh
cargo test -p spacegame2d-ui-protocol
cargo test -p spacegame2d
cd crates/spacegame2d/hud && npm test
```

Regenerate the Rust schema before the HUD QA gate:

```sh
cargo run -p spacegame2d-ui-protocol --bin export-ui-schema
```

After all three checkpoints, run both required gates:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
./scripts/qa-hud.sh
```

`qa-hud.sh` performs a clean npm install, schema contract check, Svelte/TypeScript check, Vitest,
production build, and verifies that the checked-in `hud/dist` bundle is current.

### Manual two-client matrix

Run one server and two clients, then verify:

1. Connect client A only.
   - It remains on the Relay Operations main menu.
   - It shows exactly `Waiting for opponent…`, its accepted name/color, and Disconnect.
   - Right-click, reset, and camera-drag commands do not affect gameplay.
2. Connect client B.
   - Both clients show `Match accepted`.
   - Both show canonical names and opposite cyan/coral assignments.
   - Both clocks agree from the same match anchor.
3. Observe the transition.
   - The centered card holds about 700 ms and docks over about 600 ms.
   - The game window/WebView is not recreated.
   - Gameplay input outside the bounded card/bar works once the match is active.
4. Intentionally disconnect client B.
   - Client B returns to the connection menu with no stale match data.
   - Client A stays in compact mode, keeps both names and clock, and shows B disconnected.
5. Connect a replacement client.
   - Client A updates the opponent identity/presence in place without replaying its join animation.
   - The replacement receives its own active join animation and the preserved clock anchor.
6. Intentionally disconnect client A from the compact HUD.
   - It returns to the main menu and clears all session data.
7. Repeat with an unexpected transport loss.
   - Stale match state clears locally; automatic recovery remains out of scope for SWA-54.

Capture the manual result in the PR test output because WebView instance continuity, child-window
input bounds, and two-client clock consistency are not fully provable in unit tests.

## Risks and guardrails

### Last-opponent identity is intentionally presentation history

The server roster remains current-only. Retaining a departed opponent in
`MatchSessionPresenter` is valid only while the authoritative match timing remains active and
presence is `Disconnected`. Clear it on reset and replace it immediately from a new authoritative
`Present` snapshot. Never feed it back into networking or simulation.

### Do not confuse bridge sequence with presence revision

Clock publications increase the bridge `sequence` but must not modify `presenceRevision`.
Presence revision remains server-owned.

### Do not use browser time as match truth

Svelte timers may control only the 700/600 ms visual phases. They must not advance the match clock.

### Preserve synchronization while waiting

Gate inputs/presentation, not simulation stepping, event application, or checksums.

### Child WebView transparency is not input pass-through

Keep the full child bounds only for pregame/waiting. During join/docking/compact phases, set the
native child bounds to the visible surface. CSS `pointer-events: none` is not a substitute for
native bounds.

### Keep the experiment scoped

Do not add a lobby, invites, matchmaking, ready state, round lifecycle, unexpected-loss recovery,
accessibility/reduced-motion work, or server protocol changes. Those are outside these tickets.

## Commit shape

Recommended commits:

1. `SWA-60: expose typed match session state to the HUD`
2. `SWA-59: keep solo players in the waiting menu`
3. `SWA-48: add the live match status HUD`

If this experiment is opened as one PR, list all three tickets and reproduce each ticket's
acceptance criteria as checkboxes. The repository convention normally prefers one ticket per PR;
for production review, the cleaner alternative is three stacked PRs at these exact checkpoint
boundaries.
