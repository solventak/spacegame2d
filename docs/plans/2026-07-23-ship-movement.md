# Single-Ship Inertial Movement Implementation Plan

> **For Hermes:** Implement this plan directly in the repository. Do not use subagents. Follow strict RED → GREEN TDD for simulation and input behavior.

**Goal:** Add a fixed-60-Hz, deterministic single-ship movement prototype controlled by binary W/A/D input, with inertial linear and angular motion, a one-shot latched reset, and a notched procedural ship that visibly translates and rotates in the existing fixed camera.

**Architecture:** Add a pure `simulation` module that owns meters/radians, velocity, heading, constraints, damping, reset behavior, and the monotonic tick. Add a pure `input` module that translates discrete key transitions into current per-tick controls and one-shot commands. Keep `main.rs` as orchestration: map `winit` events into input transitions, execute commands and simulation at fixed tick boundaries, then pass read-only ship state to the renderer. The renderer remains presentation-only and renders the latest completed simulation state directly; interpolation is deferred.

**Tech Stack:** Rust 2024, `glam::Vec2`, `winit 0.30.13`, `wgpu 30.0.0`, `bytemuck`, `pollster`.

**Starting branch:** `ak/feat/2d-triangle-bootstrap` at `751ca00`

**Implementation branch:** `ak/feat/ship-movement`

---

## Settled Design Decisions — Do Not Relitigate

| Area | Decision |
|---|---|
| Simulation rate | Fixed 60 Hz |
| Render timing | Render latest completed simulation state directly; no interpolation yet |
| Units | Position in meters, orientation in radians, velocity in m/s, angular velocity in rad/s |
| Forward convention | Local `+Y` is forward; heading `0` points the triangle upward |
| Thrust | W is binary on/off forward thrust relative to current facing |
| Turning | A is binary counterclockwise angular thrust; D is binary clockwise angular thrust |
| Combined controls | W and A/D may be active simultaneously |
| Opposed controls | A+D produce zero net torque; existing angular velocity damps naturally |
| Linear motion | Acceleration is capped by maximum total velocity magnitude, including lateral drift |
| Linear damping | Applied while forward thrust is off |
| Angular motion | Explicit angular thrust, moment of inertia, angular acceleration, angular velocity cap, and damping |
| Angular damping | Applied while net angular thrust is zero |
| Initial state | Centered, default orientation, zero linear velocity, zero angular velocity |
| Camera/world | Fixed camera; no boundary, wrapping, bounce, or collision |
| Reset | R creates a discrete one-shot reset command consumed at a tick boundary |
| Reset result | Center, heading zero, linear velocity zero, angular velocity zero; simulation tick remains monotonic |
| Reset input latch | Any W/A/D key physically held during reset is suppressed until that key is released and pressed again |
| Facing readability | Procedural ship has a rear notch/cutout; nose remains local `+Y` |
| Future AI | AI will provide the same binary per-tick `ShipInput` as keyboard control |
| Future multiplayer | Server resolves/schedules authoritative input frames for ticks; transport is not part of this slice |

## Explicitly Deferred

- Render interpolation or extrapolation
- Rendering faster than simulation
- Networking, protocol types, command broadcast, prediction, rollback, reconciliation, or snapshots
- Strict cross-platform floating-point determinism guarantees
- Multiple ships or GPU instancing
- Camera tracking, pan, or zoom
- World boundaries, collision, combat, trails, bloom, particles, UI, or telemetry
- Analog thrust levels
- Reverse or lateral thrusters
- Runtime tuning controls or config files

## File Layout After Implementation

```text
spacegame2d/
├── Cargo.toml                  # add glam
├── Cargo.lock
├── docs/plans/2026-07-23-ship-movement.md
└── src/
    ├── input.rs                # keyboard-independent input/latch state machine
    ├── main.rs                 # winit + fixed tick + renderer orchestration
    ├── shader.wgsl             # world translation/rotation to clip space
    └── simulation.rs           # pure movement simulation and tests
```

Do not extract the existing renderer into another module in this milestone. That would be unrelated churn; `main.rs` can retain the GPU structs while consuming `Simulation::ship()` read-only.

---

## Proposed Simulation Contract

```rust
// src/simulation.rs
use glam::Vec2;

pub const SIMULATION_HZ: u32 = 60;
pub const FIXED_DT_SECONDS: f32 = 1.0 / SIMULATION_HZ as f32;

// Initial tunable values. These are starting points, not balance promises.
pub const SHIP_MASS_KG: f32 = 1.0;
pub const FORWARD_THRUST_NEWTONS: f32 = 8.0;
pub const MAX_SPEED_METERS_PER_SECOND: f32 = 8.0;
pub const LINEAR_DAMPING_PER_SECOND: f32 = 0.8;
pub const MOMENT_OF_INERTIA_KG_M2: f32 = 0.25;
pub const ANGULAR_THRUST_NEWTON_METERS: f32 = 2.0;
pub const MAX_ANGULAR_SPEED_RADIANS_PER_SECOND: f32 = 3.0;
pub const ANGULAR_DAMPING_PER_SECOND: f32 = 2.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipInput {
    pub thrust: bool,
    pub turn_left: bool,
    pub turn_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationCommand {
    ResetShip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading_radians: f32,
    pub angular_velocity_radians_per_second: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Simulation {
    tick: u64,
    ship: ShipState,
}

impl Simulation {
    pub fn tick(&self) -> u64;
    pub fn ship(&self) -> &ShipState;
    pub fn apply_command(&mut self, command: SimulationCommand);
    pub fn step(&mut self, input: ShipInput);
}
```

The simulation owns all authoritative motion. The renderer must never update position, velocity, heading, or angular velocity.

### Tick update order

Each `Simulation::step(input)` performs this exact order:

1. Resolve angular axis: left `+1`, right `-1`, both/neither `0`.
2. If axis is nonzero, apply angular acceleration `torque / inertia` for one fixed step; otherwise apply angular damping.
3. Clamp angular velocity to `±MAX_ANGULAR_SPEED_RADIANS_PER_SECOND`.
4. Integrate heading and wrap it to a bounded interval such as `[-π, π)`.
5. Compute forward from heading using `Vec2::new(-heading.sin(), heading.cos())`, making positive heading counterclockwise from local `+Y`.
6. If thrust is active, apply `force / mass` along forward; otherwise apply linear damping.
7. Clamp the magnitude of the complete velocity vector to `MAX_SPEED_METERS_PER_SECOND`.
8. Integrate position.
9. Increment `tick` exactly once.

Use fixed-step linear drag for damping:

```rust
let retention = (1.0 - damping_per_second * FIXED_DT_SECONDS).max(0.0);
value *= retention;
```

Snap very small velocity/angular-velocity magnitudes to zero so a reset or long coast reaches a truly static state rather than retaining floating-point residue.

`SimulationCommand::ResetShip` restores only ship state. It does **not** rewind `Simulation::tick`, because future authoritative scheduling requires a monotonic timeline.

---

## Proposed Input Contract

```rust
// src/input.rs
use crate::simulation::{ShipInput, SimulationCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Thrust,
    TurnLeft,
    TurnRight,
    Reset,
}

#[derive(Default)]
pub struct InputController {
    // One movement-key state per control: physically held + blocked-until-release.
    // One pending reset bit, consumed through take_command().
}

impl InputController {
    pub fn press(&mut self, key: ControlKey);
    pub fn release(&mut self, key: ControlKey);
    pub fn controls(&self) -> ShipInput;
    pub fn take_command(&mut self) -> Option<SimulationCommand>;
    pub fn clear_for_focus_loss(&mut self);
}
```

Behavior:

- Movement key press sets physical-held state unless blocked by reset.
- Movement key release clears physical-held state **and** clears its reset block.
- Reset press sets one pending command and blocks every movement key currently held.
- Repeated reset presses before a tick coalesce into one command.
- `take_command()` returns reset once, then `None` until a new R press.
- Key-repeat events must not generate repeated reset commands.
- Focus loss clears all held and blocked movement state to prevent stuck controls.
- AI and multiplayer code will bypass `InputController` and construct `ShipInput` directly; keyboard behavior is not part of the simulation.

---

## Proposed Render Contract

Replace the aspect-only uniform with two packed `vec4` rows to avoid Rust/WGSL alignment ambiguity:

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
    // x/y: inverse world half extents; z/w reserved
    viewport: [f32; 4],
    // x/y: ship world position; z: sin(heading); w: cos(heading)
    ship: [f32; 4],
}
```

Use a fixed vertical camera span:

```rust
const VIEW_HEIGHT_METERS: f32 = 20.0;
let half_height = VIEW_HEIGHT_METERS * 0.5;
let half_width = half_height * surface_aspect;
```

WGSL transforms local ship vertices into world space and then clip space:

```wgsl
let sine = scene.ship.z;
let cosine = scene.ship.w;
let local = input.position;
let rotated = vec2<f32>(
    local.x * cosine - local.y * sine,
    local.x * sine + local.y * cosine,
);
let world = rotated + scene.ship.xy;
output.clip_position = vec4<f32>(world * scene.viewport.xy, 0.0, 1.0);
```

The ship mesh is authored in meters. Use a concave notched silhouette with six boundary points and four triangles:

```text
nose
  /
 /  \
/    \
\  /     <- rear notch points toward the nose
 \/
```

Suggested outer points:

```rust
const NOSE: [f32; 2] = [0.0, 0.60];
const RIGHT_REAR: [f32; 2] = [0.45, -0.40];
const RIGHT_NOTCH: [f32; 2] = [0.14, -0.40];
const NOTCH_APEX: [f32; 2] = [0.0, -0.15];
const LEFT_NOTCH: [f32; 2] = [-0.14, -0.40];
const LEFT_REAR: [f32; 2] = [-0.45, -0.40];
```

Triangulate as:

```text
(nose, right_rear, right_notch)
(nose, right_notch, notch_apex)
(nose, notch_apex, left_notch)
(nose, left_notch, left_rear)
```

Draw those four triangles in cyan, then draw a uniformly scaled copy in black. The notch is real absent geometry rather than a black mask, so later stars/effects can remain visible through it.

---

# Implementation Tasks

## Task 0: Create the movement feature branch and preserve the plan

**Objective:** Keep movement work separate from the triangle bootstrap commit.

**Files:**
- Add: `docs/plans/2026-07-23-ship-movement.md`

**Steps:**

1. Confirm the baseline:

   ```bash
   git status --short --branch
   git log --oneline -3
   ```

   Expected: branch `ak/feat/2d-triangle-bootstrap`, commit `751ca00`, with only this plan untracked.

2. Create the feature branch:

   ```bash
   git switch -c ak/feat/ship-movement
   ```

3. Commit the plan separately:

   ```bash
   git add docs/plans/2026-07-23-ship-movement.md
   git commit -m "docs: plan inertial ship movement"
   ```

---

## Task 1: Add the pure simulation shell and reset contract

**Objective:** Establish meters/radians state, monotonic ticks, and one-shot reset before motion equations.

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `src/simulation.rs`
- Modify: `src/main.rs` only to declare `mod simulation;` so tests compile

**Step 1 — Dependency:**

Add the same math library version used by the original project:

```toml
glam = "=0.33.2"
```

Run:

```bash
cargo update -p glam --precise 0.33.2
```

If `glam` is not yet in the lockfile, use `cargo generate-lockfile` instead.

**Step 2 — RED tests:**

Add tests proving:

- Default simulation starts at tick 0, centered, heading 0, and with zero velocities.
- One empty `step()` advances tick to 1 without moving.
- `ResetShip` returns ship state to defaults but preserves the current tick.

Run each focused test and confirm it fails because the production types/methods are missing:

```bash
cargo test simulation::tests::default_simulation_starts_stationary --locked
cargo test simulation::tests::reset_restores_ship_without_rewinding_tick --locked
```

**Step 3 — GREEN implementation:**

Add the proposed public types and minimal `Default`, getters, `apply_command`, and `step` implementation. At this task, `step` may only increment tick.

**Step 4 — Verify:**

```bash
cargo test simulation::tests --locked
```

Expected: all simulation shell/reset tests pass.

**Step 5 — Commit:**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/simulation.rs
git commit -m "feat: add fixed-tick ship simulation state"
```

---

## Task 2: Implement forward thrust, drift, damping, and total-speed cap

**Objective:** Make W-driven linear movement physically meaningful and constrained.

**Files:**
- Modify/Test: `src/simulation.rs`

**Step 1 — RED tests:**

Write separate behavior tests:

1. Heading zero + thrust increases positive Y velocity and position.
2. Thrust after a 90° counterclockwise heading produces negative X acceleration.
3. Releasing thrust preserves drift but decreases speed through damping.
4. Sustained thrust never exceeds the maximum **total velocity magnitude**.
5. Lateral velocity is included in the same magnitude cap.

Use a local helper rather than a new assertion dependency:

```rust
fn assert_close(actual: f32, expected: f32, epsilon: f32) {
    assert!((actual - expected).abs() <= epsilon, "{actual} != {expected}");
}
```

Verify RED with focused commands such as:

```bash
cargo test simulation::tests::forward_thrust_accelerates_along_heading --locked
cargo test simulation::tests::total_velocity_is_capped --locked
```

**Step 2 — GREEN implementation:**

Implement force/mass acceleration, no-thrust damping, total-vector clamping, and position integration in the documented tick order. Do not add angular behavior beyond what the current tests need.

**Step 3 — Verify:**

```bash
cargo test simulation::tests --locked
```

**Step 4 — Commit:**

```bash
git add src/simulation.rs
git commit -m "feat: add inertial forward ship movement"
```

---

## Task 3: Implement angular thrust, rotational inertia, damping, and cap

**Objective:** Make A/D produce constrained inertial rotation with the agreed screen direction.

**Files:**
- Modify/Test: `src/simulation.rs`

**Step 1 — RED tests:**

Add separate tests proving:

1. Left input produces positive angular velocity and counterclockwise heading.
2. Right input produces negative angular velocity and clockwise heading.
3. Angular acceleration equals `torque / moment_of_inertia` over one fixed step.
4. Releasing A/D decreases—but does not instantly zero—angular velocity.
5. A+D produces no new torque and damps existing angular velocity.
6. Sustained rotation remains within the angular-velocity cap.
7. W+A in the same tick both rotates and accelerates, producing a curved trajectory over many ticks.

Verify RED with:

```bash
cargo test simulation::tests::left_applies_counterclockwise_angular_thrust --locked
cargo test simulation::tests::opposed_turn_inputs_cancel_torque --locked
```

**Step 2 — GREEN implementation:**

Implement axis resolution, torque/inertia angular acceleration, no-torque damping, angular cap, heading integration, and bounded heading wrapping before linear thrust direction is calculated.

**Step 3 — Verify:**

```bash
cargo test simulation::tests --locked
```

**Step 4 — Commit:**

```bash
git add src/simulation.rs
git commit -m "feat: add inertial angular ship movement"
```

---

## Task 4: Lock the deterministic per-tick input seam

**Objective:** Prove that humans, AI, or authoritative multiplayer input frames can drive the same simulation API reproducibly.

**Files:**
- Modify/Test: `src/simulation.rs`

**Step 1 — RED test:**

Build a fixed sequence of `ShipInput` values containing idle, thrust, left, right, opposed turning, and combined controls. Run two fresh simulations through the exact same sequence and assert exact equality of tick and ship state.

Also prove reset at an intermediate tick produces identical subsequent results in both runs and does not rewind the tick.

```rust
assert_eq!(first, second);
```

Run:

```bash
cargo test simulation::tests::identical_tick_inputs_produce_identical_state --locked
```

Expected RED: the test initially fails until any missing equality/state seams are completed.

**Step 2 — GREEN/refactor:**

Make the smallest API cleanup needed for the deterministic test. Do not add networking, serialization, replay storage, or a scheduler.

**Step 3 — Verify and commit:**

```bash
cargo test simulation::tests --locked
git add src/simulation.rs
git commit -m "test: lock deterministic ship input sequence"
```

---

## Task 5: Implement keyboard-independent held-input and reset-latch behavior

**Objective:** Convert key transitions into binary controls and a one-shot reset without leaking keyboard concerns into simulation.

**Files:**
- Create/Test: `src/input.rs`
- Modify: `src/main.rs` only to declare `mod input;`

**Step 1 — RED tests:**

Add one test per behavior:

1. Press/release W toggles thrust.
2. Press/release A toggles left turn; D toggles right turn.
3. A and D may both be true; cancellation belongs to simulation.
4. R produces one `ResetShip`, and the second `take_command()` returns `None`.
5. Reset while W/A are held immediately clears their exposed controls.
6. Held movement keys remain suppressed after reset.
7. Releasing and pressing each suppressed key reactivates only that key.
8. A movement key not held during reset remains available normally.
9. Focus loss clears all controls and suppression.

Critical reset-latch test shape:

```rust
controller.press(ControlKey::Thrust);
controller.press(ControlKey::TurnLeft);
controller.press(ControlKey::Reset);

assert_eq!(controller.controls(), ShipInput::default());
assert_eq!(controller.take_command(), Some(SimulationCommand::ResetShip));
assert_eq!(controller.take_command(), None);

// Still physically held, still blocked.
assert_eq!(controller.controls(), ShipInput::default());

controller.release(ControlKey::Thrust);
controller.press(ControlKey::Thrust);
assert!(controller.controls().thrust);
assert!(!controller.controls().turn_left);
```

Verify RED:

```bash
cargo test input::tests::reset_blocks_held_controls_until_release_and_repress --locked
```

**Step 2 — GREEN implementation:**

Use a small private state per movement key:

```rust
#[derive(Default)]
struct MovementKeyState {
    physically_held: bool,
    blocked_until_release: bool,
}
```

Reset sets `blocked_until_release = true` only for movement keys currently held. Release clears both fields. `controls()` exposes `physically_held && !blocked_until_release`.

**Step 3 — Verify:**

```bash
cargo test input::tests --locked
cargo test --locked
```

**Step 4 — Commit:**

```bash
git add src/input.rs src/main.rs
git commit -m "feat: add latched binary ship controls"
```

---

## Task 6: Add world-space ship transform and the rear notch

**Objective:** Make simulation position/heading visible while preserving the cyan-outline-on-black style.

**Files:**
- Modify: `src/main.rs` (`SceneUniform`, vertices, renderer update)
- Modify: `src/shader.wgsl`

**Step 1 — Add CPU-side contract tests before GPU changes:**

Extract pure helpers in `main.rs` for:

- `scene_uniform(config, ship)`
- notched ship vertex generation

Write tests proving:

1. At 20 m view height and square aspect, world center maps to zero translation and inverse half extents are `0.1`.
2. Scene uniform stores `sin`/`cos` matching heading.
3. Outer and inner notched meshes have the expected fixed vertex count (24 total vertices: 12 outer + 12 inner).
4. No generated triangle fills the notch opening.

Run the focused tests and confirm RED before implementing helpers:

```bash
cargo test tests::scene_uniform_uses_fixed_world_scale --locked
cargo test tests::ship_mesh_preserves_rear_notch --locked
```

**Step 2 — Replace geometry:**

Replace the two solid triangles with the documented concave outer/inner triangulation in local meters. Preserve cyan outer and black inner colors.

**Step 3 — Replace uniform/shader transform:**

- Replace `ViewportUniform` with the 32-byte `SceneUniform` composed of two `[f32; 4]` values.
- Set `min_binding_size` from `size_of::<SceneUniform>()` after confirming it is 32 bytes.
- Update the scene buffer before each render using the latest `ShipState`.
- Apply rotation, world translation, and fixed orthographic scaling in WGSL.
- Change `Renderer::render()` to accept `&ShipState`; do not let it mutate simulation.

**Step 4 — Static verification:**

```bash
cargo fmt --all -- --check
cargo test --locked
cargo check --locked
```

**Step 5 — Runtime initialization verification:**

```bash
cargo run --locked
```

Expected: real window and pipeline initialize without WGSL, uniform-layout, surface, or validation panic; centered cyan notched ship appears on black.

**Step 6 — Commit:**

```bash
git add src/main.rs src/shader.wgsl
git commit -m "feat: render notched ship from simulation state"
```

---

## Task 7: Wire winit input and the fixed 60 Hz application loop

**Objective:** Complete the end-to-end path from keyboard transitions to tick-boundary simulation to rendering.

**Files:**
- Modify: `src/main.rs`

**Step 1 — Extend `App`:**

Add:

```rust
simulation: Simulation,
input: InputController,
next_tick: Instant,
```

Initialize `next_tick` when the app resumes.

**Step 2 — Map physical keys:**

Map `PhysicalKey::Code` values:

```text
KeyW -> ControlKey::Thrust
KeyA -> ControlKey::TurnLeft
KeyD -> ControlKey::TurnRight
KeyR -> ControlKey::Reset
```

- Pressed movement keys call `press` idempotently.
- Released movement keys call `release`.
- Ignore repeated `KeyR` presses using `event.repeat` so OS key repeat cannot emit repeated resets.
- On `WindowEvent::Focused(false)`, call `clear_for_focus_loss()`.

Do not mutate `Simulation` inside keyboard event handling.

**Step 3 — Add the tick boundary:**

In `ApplicationHandler::about_to_wait`:

```rust
while Instant::now() >= self.next_tick {
    if let Some(command) = self.input.take_command() {
        self.simulation.apply_command(command);
    }
    self.simulation.step(self.input.controls());
    self.next_tick += TICK_DURATION;
    stepped = true;
}
```

If at least one tick ran, request a redraw. Set control flow to `ControlFlow::WaitUntil(self.next_tick)` instead of continuously polling. Resize/redraw events may still render the latest completed state without advancing simulation.

Use one fixed simulation delta derived from `SIMULATION_HZ`; never derive movement delta from render-frame time.

**Step 4 — Render read-only state:**

On `RedrawRequested`:

```rust
renderer.render(self.simulation.ship())
```

No interpolation and no render-owned copy of velocity/position are needed in this slice.

**Step 5 — Verify compilation and tests:**

```bash
cargo fmt --all -- --check
cargo test --locked
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
```

**Step 6 — Commit:**

```bash
git add src/main.rs
git commit -m "feat: drive ship movement at fixed 60 hz"
```

---

## Task 8: Real runtime movement verification and tuning pass

**Objective:** Verify the actual player-visible control loop and adjust only named movement constants if necessary.

**Files:**
- Modify only if needed: tuning constants in `src/simulation.rs`

**Step 1 — Launch the real binary:**

```bash
cargo run --locked
```

Confirm no GPU validation panic and exercise this checklist in the actual window:

1. Ship starts centered, still, and facing upward.
2. Rear notch clearly distinguishes front from rear.
3. Hold W: ship accelerates along its nose.
4. Release W: ship keeps drifting but gradually slows.
5. Hold A: ship gains counterclockwise angular velocity.
6. Release A: rotation persists briefly and damps rather than stopping instantly.
7. Hold D: clockwise behavior mirrors A.
8. Hold W+A and W+D: ship follows curved trajectories.
9. Hold A+D: no new torque is applied; existing spin damps.
10. Ship can leave the fixed view; there is no wrapping or collision.
11. Tap R: ship returns to center, heading zero, and has no linear/angular motion.
12. Hold W, tap R while still holding W: ship remains reset and stationary.
13. Release W and press W again: thrust becomes active.
14. Repeat the same reset-latch check independently for A and D.

**Step 2 — Tune only if motion is unusable:**

Adjust only the named constants in `src/simulation.rs`. Do not change equations, add effects, or broaden scope during tuning. Re-run all simulation tests after every tuning edit.

**Step 3 — Full final verification:**

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
cargo tree --depth 1
git status --short --branch
```

Expected direct dependencies:

```text
bytemuck
pollster
glam
wgpu
winit
```

**Step 4 — Final commit if tuning changed:**

```bash
git add src/simulation.rs
git commit -m "tune: adjust single-ship movement feel"
```

Do not create or publish a PR until explicitly requested and a Git remote exists.

---

## Acceptance Criteria

- [ ] Simulation advances only in fixed 1/60-second steps.
- [ ] Simulation tick is explicit and monotonically increasing.
- [ ] Renderer reads but never owns or mutates ship motion state.
- [ ] W applies binary forward thrust relative to current heading.
- [ ] A applies counterclockwise angular thrust; D applies clockwise angular thrust.
- [ ] W and turning work simultaneously.
- [ ] A+D cancel torque and allow angular damping.
- [ ] Linear and angular velocity preserve inertia when input is released.
- [ ] Linear and angular damping are gradual, not immediate stops.
- [ ] Total linear velocity and angular velocity obey their caps.
- [ ] R is consumed once at a tick boundary and does not reset the simulation tick.
- [ ] Movement keys held during reset remain blocked until individually released and pressed again.
- [ ] Fixed camera has no boundary behavior; ship may leave the visible area.
- [ ] Notched procedural silhouette makes facing unambiguous.
- [ ] Identical initial state and tick-input sequence produce identical final state in a headless test.
- [ ] Formatting, locked check/test, clippy-with-warnings-denied, and diff checks pass.
- [ ] Real `wgpu` runtime initializes and the complete W/A/D/R loop is manually verified.

## Known Risks / Open Questions

No unanswered product decisions block this milestone. The initial tuning values are deliberately provisional and should be judged in the running prototype.

Two future technical risks are recorded but explicitly out of scope:

1. Rust `f32` repeatability in one process is sufficient for this prototype, but strict cross-platform lockstep determinism may require stronger numeric/platform constraints later.
2. Direct 60 Hz rendering may visibly stutter on high-refresh displays; previous/current-state interpolation is the agreed later solution.
