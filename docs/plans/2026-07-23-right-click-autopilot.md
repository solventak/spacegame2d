# Right-Click Flight Autopilot Implementation Plan

> **For Hermes:** Implement this plan directly and sequentially in the repository. Do not use subagents. Follow strict RED → GREEN TDD for controller, actuator, lifecycle, and coordinate behavior. Preserve the existing renderer and manual movement behavior while adding the visible autopilot slice.

**Goal:** Right-click a world-space destination and have the existing single ship autonomously turn, accelerate, brake, correct overshoot, and settle near that point with near-zero linear velocity, using a swappable flight algorithm and tunable human-paced W/A/D actuation.

**Architecture:** Keep `Simulation` authoritative over ship position, velocity, heading, angular velocity, and fixed-tick integration. Add a renderer-independent autopilot layer that receives read-only `ShipState`, asks an injected `FlightController` algorithm for desired binary controls, passes those controls through a deterministic coarse-actuation gate, and returns the resulting `ShipInput` to the existing simulation. `main.rs` remains the orchestrator for window events and tick ordering; the renderer owns only the persistent red destination marker and HUD presentation.

**Efficiency model:** One controller evaluation and a handful of vector/scalar operations per active ship per 60 Hz tick, with no per-tick heap allocation. The first implementation uses one boxed algorithm for runtime swapability; a virtual call per controlled ship per tick is acceptable for hundreds or low thousands of drones and can later be replaced by batched enum dispatch only if profiling proves it necessary.

**Tech Stack:** Rust 2024, `glam::Vec2`, `winit 0.30.13`, `wgpu 30.0.0`, fixed 60 Hz simulation, procedural GPU geometry.

**Starting branch:** `ak/feat/ship-movement` at `25a5ab6`

**Implementation branch:** `ak/feat/right-click-autopilot`

---

## Existing Code Boundary

The current application already provides the correct lower-level seam:

```text
winit keyboard events
    -> InputController
    -> App::about_to_wait (fixed 60 Hz)
    -> Simulation::step(ShipInput)
    -> Renderer::render(&ShipState)
```

Relevant files today:

- `src/simulation.rs` owns deterministic W/A/D physics and the monotonic simulation tick.
- `src/input.rs` owns physical key state, one-shot reset, and reset suppression until release/re-press.
- `src/main.rs` owns the window event loop, tick timing, static ship renderer, and manual input selection.
- `src/shader.wgsl` transforms the ship from local meters to world/clip space.

The autopilot must reuse `ShipInput`. It must not add a second movement integrator, directly mutate `ShipState`, or give the algorithm hidden braking/lateral capabilities.

---

## Settled Design Decisions — Do Not Relitigate

| Area | Decision |
|---|---|
| Command | Right-click sets a world-space destination |
| Replacement | A later right-click immediately replaces the prior destination |
| First decision | The new destination is evaluated on the next simulation tick with no global cooldown delay |
| Controlled entity | One ship only in this milestone |
| Fleet context | Keep algorithm/actuation seams cheap and per-ship reusable; do not implement selection or fleets yet |
| Available controls | Existing binary W/A/D only: forward thrust, left angular thrust, right angular thrust |
| Manual input | W/A/D is ignored while autopilot is active |
| Arrival position | Being within a small configurable radius is sufficient |
| Arrival velocity | Linear speed must also be below a configurable near-zero threshold |
| Final heading | Unconstrained |
| Angular velocity at completion | Unconstrained; only translational completion matters |
| Overshoot | Continue correcting, including turning around and returning to the target |
| Optimization bias | Favor fast practical arrival; some overshoot/correction is acceptable |
| Braking | May turn around and thrust opposite velocity, temporarily moving away from the destination |
| Combined control | May thrust while turning (`W+A` / `W+D`) |
| Algorithm architecture | Flight-control algorithm must be injected/swappable |
| Coarseness architecture | Algorithm emits desired controls; a separate actuation gate decides which changes are currently allowed |
| Decision timing | All evaluations and changes happen at fixed simulation tick boundaries |
| Per-control hold | Every W/A/D state change starts that control’s tunable minimum hold period |
| Global cooldown | Any decision that changes at least one control starts a longer tunable decision cooldown |
| Partial eligibility | After global cooldown, eligible controls may change even if another control remains in its hold period |
| Emergency behavior | No emergency bypass; all algorithm-requested control changes use the coarseness system |
| Completion | Autopilot deactivates automatically after arrival conditions are satisfied and output controls are off |
| Reset | R resets ship, cancels autopilot, clears effective autopilot controls, and clears destination marker |
| Indicator | Visible in-window `AUTOPILOT: ON/OFF` HUD label |
| Destination marker | Simple red dot in world space |
| Marker lifetime | Marker remains after arrival; a new destination moves it; R clears it |

### Explicit implementation default for the one unanswered replacement edge

A replacement destination bypasses the **global decision cooldown** so it is evaluated on the next tick, but it does **not** bypass an individual control’s existing minimum hold deadline. This preserves the requirement that every control change goes through the coarseness system. Eligible controls can still respond immediately to the new target.

### Manual-input handoff safety

When autopilot activates, any W/A/D keys currently held are suppressed until released and pressed again, using the same safety behavior already established for reset. This prevents a key held during a long autopilot command from unexpectedly moving the ship immediately after automatic completion.

---

## Explicitly Deferred

- Multi-selection, formations, flocking, separation, cohesion, obstacle avoidance, or fleet pathfinding
- Multiple simultaneous ships
- Network transport, server command scheduling, prediction, rollback, or reconciliation
- Runtime algorithm picker UI; swapability is established through injection and tests first
- Analog thrust, reverse thrust, lateral thrusters, or special braking
- Perfect time-optimal control, trajectory search, model predictive control, PID autotuning, or machine learning
- Camera pan/zoom and destinations outside the current fixed view
- Queued waypoints, patrol, attack-move, or command append modifiers
- Path previews, velocity vectors, braking-distance overlays, trails, or telemetry graphs
- Strict cross-platform floating-point determinism guarantees
- Imported fonts or a general UI/text framework

---

## Target Data Flow

```text
CursorMoved
    -> latest cursor position in physical pixels

Right mouse press
    -> screen_to_world(cursor, viewport)
    -> pending destination (latest wins before next tick)

Next fixed 60 Hz tick
    -> consume ResetShip first, if any
    -> otherwise consume pending destination
    -> Autopilot::set_destination(world_point, tick)
    -> injected FlightController::desired_input(observation)
    -> CoarseActuator::apply(desired_input, tick)
    -> effective ShipInput
    -> Simulation::step(effective ShipInput)

Render
    -> latest ShipState
    -> Autopilot active flag
    -> persistent destination Option<Vec2>
    -> ship + red destination dot + AUTOPILOT ON/OFF HUD
```

Reset wins if reset and a destination are pending for the same simulation tick: reset clears both the active command and destination marker. The pending destination must be discarded on that tick rather than reactivating autopilot immediately after reset.

---

## Proposed File Layout

```text
src/
├── autopilot.rs
│   ├── Autopilot lifecycle
│   ├── CoarseActuator
│   ├── ActuationConfig
│   └── deterministic unit/integration tests
├── flight_control/
│   ├── mod.rs
│   │   ├── FlightController trait
│   │   ├── FlightObservation
│   │   └── controller factory/name seam
│   └── braking_pursuit.rs
│       ├── BrakingPursuitController
│       ├── BrakingPursuitConfig
│       └── algorithm behavior tests
├── input.rs
│   └── existing keyboard state plus reusable held-key suppression method
├── main.rs
│   ├── cursor/right-click conversion
│   ├── pending high-level destination command
│   └── fixed-tick orchestration
├── renderer.rs
│   ├── existing ship rendering moved intact
│   ├── red destination marker geometry
│   ├── procedural 5×7 HUD glyph geometry
│   └── presentation helper tests
├── shader.wgsl
│   ├── ship vertex entry point
│   ├── world-geometry vertex entry point
│   └── clip-space HUD vertex entry point
└── simulation.rs
    └── existing authoritative motion; expose read-only physics constants only as needed
```

Extracting `Renderer` from the current 447-line `main.rs` is in scope because the marker and HUD add two presentation paths. Do this as a behavior-preserving move before adding new rendering behavior. Do not refactor `Simulation` or `InputController` beyond the minimal APIs required by autopilot integration.

---

## Core Interfaces

### Swappable algorithm contract

```rust
// src/flight_control/mod.rs
use glam::Vec2;

use crate::simulation::ShipInput;

#[derive(Clone, Copy, Debug)]
pub struct FlightObservation {
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading_radians: f32,
    pub angular_velocity_radians_per_second: f32,
    pub destination: Vec2,
}

pub trait FlightController: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn desired_input(&self, observation: FlightObservation) -> ShipInput;
}
```

The method takes `&self`, not `&mut self`: the initial algorithm is stateless. This permits one algorithm instance to be shared across future ships if needed. Per-ship temporal state belongs in `Autopilot`/`CoarseActuator`, not in the algorithm.

Use constructor injection:

```rust
pub struct Autopilot {
    controller: Box<dyn FlightController>,
    // destination, active flag, coarse actuator...
}

impl Autopilot {
    pub fn new(controller: Box<dyn FlightController>, config: AutopilotConfig) -> Self;
}
```

A test controller can return scripted or constant `ShipInput`, proving the coarseness and lifecycle layers do not depend on `BrakingPursuitController`.

### Coarse actuation contract

```rust
#[derive(Clone, Copy, Debug)]
pub struct ActuationConfig {
    pub thrust_min_hold_ticks: u64,
    pub turn_left_min_hold_ticks: u64,
    pub turn_right_min_hold_ticks: u64,
    pub decision_cooldown_ticks: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct ControlLatch {
    value: bool,
    eligible_at_tick: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoarseActuator {
    thrust: ControlLatch,
    turn_left: ControlLatch,
    turn_right: ControlLatch,
    next_decision_tick: u64,
}
```

`CoarseActuator::apply(desired, tick)` follows this exact order:

1. If `tick < next_decision_tick`, return current controls unchanged.
2. For each control independently:
   - If desired equals current, do nothing.
   - If desired differs and `tick >= eligible_at_tick`, apply the change and set that control’s next eligible tick to `tick + its_min_hold_ticks`.
   - If desired differs but the control is ineligible, retain its current state.
3. If one or more controls changed, set `next_decision_tick = tick + decision_cooldown_ticks` once.
4. Return the effective controls.

Suggested starting values, intentionally centralized for tuning:

```rust
pub const DEFAULT_THRUST_MIN_HOLD_TICKS: u64 = 12; // 200 ms
pub const DEFAULT_TURN_MIN_HOLD_TICKS: u64 = 9;    // 150 ms
pub const DEFAULT_DECISION_COOLDOWN_TICKS: u64 = 18; // 300 ms
```

The global cooldown is longer than either per-control hold, matching the requested human-paced feel. These are tuning defaults, not permanent balance values.

`allow_immediate_decision(tick)` sets only `next_decision_tick = tick`; it does not clear per-control hold deadlines.

### Autopilot lifecycle contract

```rust
#[derive(Clone, Copy, Debug)]
pub struct AutopilotConfig {
    pub arrival_radius_meters: f32,
    pub stopped_speed_meters_per_second: f32,
    pub actuation: ActuationConfig,
}

pub struct Autopilot {
    controller: Box<dyn FlightController>,
    config: AutopilotConfig,
    destination: Option<Vec2>,
    active: bool,
    actuator: CoarseActuator,
}
```

Suggested first completion values:

```rust
pub const DEFAULT_ARRIVAL_RADIUS_METERS: f32 = 0.30;
pub const DEFAULT_STOPPED_SPEED_METERS_PER_SECOND: f32 = 0.08;
```

Methods:

```rust
pub fn set_destination(&mut self, destination: Vec2, tick: u64);
pub fn cancel_and_clear_destination(&mut self);
pub fn is_active(&self) -> bool;
pub fn destination(&self) -> Option<Vec2>;
pub fn controls_for_tick(&mut self, tick: u64, ship: &ShipState) -> ShipInput;
```

Completion rule:

```text
distance <= arrival_radius
AND speed <= stopped_speed
AND effective W/A/D output == all false
```

Requiring effective controls to be off prevents automatic deactivation from becoming a hidden bypass of the coarseness layer. The final heading and angular velocity are deliberately ignored.

Cancellation/reset is a discrete player command, not an algorithm decision. It may clear effective autopilot controls immediately because the simulation itself is reset in the same tick.

---

## First Algorithm: Braking-Aware Pursuit

The first algorithm should be understandable, deterministic, cheap, and replaceable—not perfectly optimal.

### Observation values

For each active tick:

```rust
let to_target = destination - position;
let distance = to_target.length();
let speed = velocity.length();
let target_direction = to_target.normalize_or_zero();
let velocity_direction = velocity.normalize_or_zero();
let forward = Vec2::new(-heading.sin(), heading.cos());
```

### Translational intent

Estimate the distance needed to remove current speed using available forward acceleration:

```rust
let thrust_acceleration = FORWARD_THRUST_NEWTONS / SHIP_MASS_KG;
let stopping_distance = speed * speed / (2.0 * thrust_acceleration);
```

Use a tunable safety factor and buffer:

```rust
should_brake = speed > stopped_speed
    && stopping_distance * braking_safety_factor + braking_buffer_meters >= distance;
```

Choose desired direction:

- **Arrival/settle:** if inside arrival radius and below stopped speed, request all controls off.
- **Brake:** if braking is needed, point opposite current velocity.
- **Approach/correct:** otherwise point toward the destination.

Overshoot naturally changes `to_target`, causing the ship to turn around and re-approach. If the ship has substantial lateral velocity, braking opposite total velocity removes that drift instead of considering only closing speed.

### Rotational intent

Compute the signed shortest angle from current forward to desired direction:

```rust
let angle_error = forward.perp_dot(desired_direction).atan2(forward.dot(desired_direction));
```

Positive error means counterclockwise/left; negative means clockwise/right.

Avoid frame-perfect left/right chatter by accounting for existing angular velocity:

```text
desired_angular_velocity = clamp(angle_error * turn_gain, -max_angular_speed, +max_angular_speed)
angular_velocity_error = desired_angular_velocity - current_angular_velocity
```

Then request:

- left when angular velocity error exceeds a positive deadband;
- right when it is below a negative deadband;
- neither inside the deadband.

The coarse actuator, not this deadband, is the primary anti-spam mechanism.

### Thrust intent

- In approach mode, thrust when facing is within a generous configurable angle of the target direction. This allows W+A/W+D and fast curved approaches.
- In brake mode, thrust only when sufficiently aligned opposite velocity, so thrust actually reduces speed.
- Inside the arrival radius, request thrust off while turning as needed to eliminate remaining translational drift.

Suggested centralized initial tuning:

```rust
pub struct BrakingPursuitConfig {
    pub braking_safety_factor: f32,       // start 1.15
    pub braking_buffer_meters: f32,       // start 0.20
    pub approach_thrust_angle_radians: f32, // start 65 degrees
    pub braking_thrust_angle_radians: f32,  // start 30 degrees
    pub turn_gain: f32,                   // start 2.5
    pub angular_velocity_deadband: f32,   // start 0.10 rad/s
}
```

Do not hide magic values in conditionals. Tuning must be possible by editing one default config.

---

# Implementation Tasks

## Task 0: Branch and preserve the plan

**Objective:** Keep the autopilot milestone acceptance-scoped and separate from ship movement.

**Files:**
- Add: `docs/plans/2026-07-23-right-click-autopilot.md`

**Steps:**

1. Confirm clean starting state:

   ```bash
   git status --short --branch
   git log --oneline -3
   ```

   Expected: `ak/feat/ship-movement`, tip `25a5ab6`, with only this plan untracked.

2. Create the branch:

   ```bash
   git switch -c ak/feat/right-click-autopilot
   ```

3. Commit the plan:

   ```bash
   git add docs/plans/2026-07-23-right-click-autopilot.md
   git commit -m "docs: plan right-click flight autopilot"
   ```

---

## Task 1: Define the swappable flight-controller seam

**Objective:** Establish an algorithm contract that consumes read-only ship/target state and emits only existing binary controls.

**Files:**
- Create: `src/flight_control/mod.rs`
- Create: `src/flight_control/braking_pursuit.rs` as an empty declared module only after the seam compiles
- Modify: `src/main.rs` to declare `mod flight_control;`

**RED tests:**

Create a tiny test-only controller implementing `FlightController`. Prove:

1. The trait can be used behind `Box<dyn FlightController>`.
2. It receives position, velocity, heading, angular velocity, and destination.
3. It returns an ordinary `ShipInput` without mutating `ShipState`.
4. `name()` provides a stable debug identity.

Example test shape:

```rust
#[derive(Debug)]
struct AlwaysThrust;

impl FlightController for AlwaysThrust {
    fn name(&self) -> &'static str { "always-thrust" }
    fn desired_input(&self, _: FlightObservation) -> ShipInput {
        ShipInput { thrust: true, ..Default::default() }
    }
}
```

Run and confirm RED because the trait/types do not exist:

```bash
cargo test flight_control::tests::controller_is_swappable_behind_trait_object --locked
```

**GREEN:** Add only the trait and observation contract.

**Verify:**

```bash
cargo test flight_control::tests --locked
cargo test --locked
```

**Commit:**

```bash
git add src/flight_control src/main.rs
git commit -m "feat: add swappable flight controller seam"
```

---

## Task 2: Build the deterministic coarse actuator

**Objective:** Enforce per-control hold times and global decision cooldown independently of any flight algorithm.

**Files:**
- Create/Test: `src/autopilot.rs`
- Modify: `src/main.rs` to declare `mod autopilot;`

Write and run one RED test at a time for:

1. First decision at tick 0 can change all eligible controls immediately.
2. An unchanged desired state does not restart hold timers or global cooldown.
3. A changed thrust state is retained until its own hold deadline.
4. Left/right use independent hold deadlines.
5. A decision changing multiple controls starts one global cooldown.
6. Before global cooldown expires, no control changes.
7. After global cooldown expires, an eligible control changes while an ineligible control remains unchanged.
8. Turning thrust off uses the same hold/cooldown rules as turning it on.
9. `allow_immediate_decision` bypasses only global cooldown, not per-control holds.
10. Identical desired-input/tick sequences produce identical effective output sequences.

Critical partial-eligibility test:

```rust
assert_eq!(
    actuator.apply(
        ShipInput { thrust: false, turn_left: false, turn_right: true },
        tick,
        config,
    ),
    ShipInput {
        thrust: true,      // still held/locked
        turn_left: false,
        turn_right: true, // eligible and changed
    }
);
```

Focused RED/GREEN commands:

```bash
cargo test autopilot::tests::first_decision_changes_controls_immediately --locked
cargo test autopilot::tests::eligible_controls_change_independently --locked
cargo test autopilot::tests::immediate_decision_preserves_control_holds --locked
```

After all focused tests:

```bash
cargo test autopilot::tests --locked
cargo clippy --all-targets --locked -- -D warnings
```

**Commit:**

```bash
git add src/autopilot.rs src/main.rs
git commit -m "feat: add human-paced autopilot actuation"
```

---

## Task 3: Add autopilot command lifecycle and completion

**Objective:** Own destination replacement, active state, marker persistence, arrival completion, and cancellation outside simulation physics.

**Files:**
- Modify/Test: `src/autopilot.rs`

Use an injected fake controller to isolate lifecycle from the real algorithm.

Write and verify RED tests for:

1. Default autopilot is inactive with no destination and no controls.
2. `set_destination` activates autopilot and stores the destination.
3. Setting a new destination replaces the old destination.
4. A new destination permits a decision on that exact tick.
5. Replacement preserves per-control hold deadlines.
6. While active, controller intent flows through `CoarseActuator`.
7. Inside arrival radius but moving too fast remains active.
8. Below stopped speed but outside arrival radius remains active.
9. Position + speed thresholds do not deactivate until effective controls are all off.
10. Complete arrival deactivates and returns zero controls.
11. Completion leaves `destination()` intact for rendering.
12. Cancellation deactivates, clears effective output, and removes destination.
13. Final heading and angular velocity do not block completion.

Run focused tests during RED/GREEN:

```bash
cargo test autopilot::tests::arrival_requires_position_and_speed --locked
cargo test autopilot::tests::arrival_keeps_destination_marker --locked
cargo test autopilot::tests::replacement_is_immediate_but_preserves_holds --locked
```

Then:

```bash
cargo test autopilot::tests --locked
cargo test --locked
```

**Commit:**

```bash
git add src/autopilot.rs
git commit -m "feat: add destination autopilot lifecycle"
```

---

## Task 4: Implement the braking-aware pursuit algorithm

**Objective:** Produce fast, understandable W/A/D intent that approaches, brakes, and corrects overshoot.

**Files:**
- Implement/Test: `src/flight_control/braking_pursuit.rs`
- Modify: `src/flight_control/mod.rs` to export the implementation
- Modify: `src/simulation.rs` only if physics constants need `pub(crate)` visibility

Write RED behavior tests for the algorithm itself, without the coarse actuator:

1. Stationary ship facing a far target ahead requests thrust and no turn.
2. Target to the left requests left turn.
3. Target to the right requests right turn.
4. Existing angular velocity toward the target can cause the controller to release turn input before perfect alignment.
5. Far target permits thrust while turning within the approach angle.
6. High speed near target selects braking direction opposite total velocity.
7. Brake thrust stays off until facing sufficiently opposite velocity.
8. Once aligned for braking, thrust turns on.
9. Lateral drift is included in braking direction.
10. Overshot target causes corrective turn toward the new target vector.
11. Inside arrival radius with near-zero speed requests all controls off.
12. No NaN or invalid direction occurs at zero distance or zero speed.

Use vector fixtures with simple headings (`0`, `±π/2`, `π`) so expected left/right signs are obvious.

Focused commands:

```bash
cargo test flight_control::braking_pursuit::tests::far_target_ahead_requests_thrust --locked
cargo test flight_control::braking_pursuit::tests::near_fast_ship_turns_to_brake --locked
cargo test flight_control::braking_pursuit::tests::lateral_drift_is_braked --locked
```

**GREEN implementation rules:**

- No state mutation.
- No allocation.
- No direct simulation step.
- No coarse-timing logic.
- All thresholds come from `BrakingPursuitConfig`.
- Use existing physics constants for thrust acceleration and angular speed cap.

After all tests:

```bash
cargo test flight_control::braking_pursuit::tests --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

**Commit:**

```bash
git add src/flight_control src/simulation.rs
git commit -m "feat: add braking-aware pursuit controller"
```

---

## Task 5: Prove the algorithm and actuator converge through the real simulation

**Objective:** Test the complete headless controller → coarse actuation → `Simulation::step` loop before UI integration.

**Files:**
- Modify/Test: `src/autopilot.rs`

Add a deterministic harness:

```rust
fn run_until_complete(
    simulation: &mut Simulation,
    autopilot: &mut Autopilot,
    max_ticks: u64,
) -> Option<u64> {
    for _ in 0..max_ticks {
        let input = autopilot.controls_for_tick(simulation.tick(), simulation.ship());
        simulation.step(input);
        if !autopilot.is_active() {
            return Some(simulation.tick());
        }
    }
    None
}
```

Write RED end-to-end tests:

1. From rest, destination straight ahead completes within a generous bound.
2. Side destination requires turning and completes.
3. Destination behind requires turning around and completes.
4. Initial lateral velocity is removed and destination completes.
5. A close destination with excessive initial speed overshoots, corrects, and eventually completes.
6. Replacing the target mid-flight converges on the second destination, not the first.
7. Final position is within arrival radius and final speed within stopped threshold.
8. Final heading is not asserted.
9. The exact same command/tick sequence yields exact same final simulation and completion tick.
10. The coarse controller produces materially fewer state changes than one decision per tick; assert a conservative upper bound rather than an exact tuning-dependent count.

Use a generous first timeout such as 3,600 ticks (60 seconds), then tighten only if the real controller reliably supports it. Do not make tests brittle by asserting one exact trajectory or completion tick except for the determinism pair.

Run:

```bash
cargo test autopilot::tests::real_controller_reaches_target_ahead --locked
cargo test autopilot::tests::real_controller_corrects_overshoot --locked
cargo test autopilot::tests::real_controller_replacement_uses_latest_target --locked
```

If these do not converge, tune only `BrakingPursuitConfig`, arrival thresholds, or coarse timing defaults. Do not add hidden forces or mutate simulation state.

Then:

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

**Commit:**

```bash
git add src/autopilot.rs src/flight_control
git commit -m "test: prove autopilot converges through simulation"
```

---

## Task 6: Add screen-to-world right-click commands

**Objective:** Convert a right mouse press into a world-space destination consumed at the next fixed tick.

**Files:**
- Modify/Test: `src/main.rs`
- Modify/Test: `src/input.rs`

### Pure coordinate helper

Add:

```rust
fn screen_to_world(
    cursor: winit::dpi::PhysicalPosition<f64>,
    width: u32,
    height: u32,
) -> glam::Vec2
```

For the existing fixed camera:

```text
ndc_x = 2 * cursor_x / width - 1
ndc_y = 1 - 2 * cursor_y / height
world_x = ndc_x * half_width
world_y = ndc_y * half_height
```

Write RED tests for:

1. Screen center maps to world origin.
2. Top-left has negative X and positive Y.
3. Bottom-right has positive X and negative Y.
4. Wide viewport uses the same aspect-derived half width as rendering.
5. Width/height are clamped to at least one to avoid division by zero.

### Event state

Add to `App`:

```rust
cursor_position: Option<PhysicalPosition<f64>>,
pending_destination: Option<Vec2>,
autopilot: Autopilot,
```

Event behavior:

- `CursorMoved` updates latest cursor position.
- Right `MouseInput::Pressed` converts the latest cursor position and sets `pending_destination = Some(world)`.
- Multiple clicks before a simulation tick coalesce; the latest point wins.
- Mouse release does nothing.
- Missing cursor position does nothing safely.

### Tick ordering

Refactor one tick into a small testable application-domain helper if needed, but preserve this order:

1. Consume reset command.
2. If reset exists:
   - `Simulation::apply_command(ResetShip)`;
   - `Autopilot::cancel_and_clear_destination()`;
   - clear `pending_destination`;
   - step with zero input for that tick.
3. Otherwise consume `pending_destination`, activate/replace target, and suppress currently held manual movement keys.
4. Select controls:
   - autopilot controls if active;
   - manual `InputController::controls()` otherwise.
5. `Simulation::step(controls)`.

### Reuse manual-key suppression

Extract the current reset latch operation in `InputController` into:

```rust
pub fn suppress_held_movement_until_release(&mut self);
```

Call it from reset and when autopilot activates. RED-test that this preserves the existing reset behavior and protects post-autopilot manual handoff.

Focused commands:

```bash
cargo test tests::screen_center_maps_to_world_origin --locked
cargo test input::tests::held_keys_can_be_suppressed_for_mode_handoff --locked
cargo test tests::reset_wins_over_pending_destination --locked
```

Then:

```bash
cargo test --locked
cargo check --locked
```

**Commit:**

```bash
git add src/main.rs src/input.rs
git commit -m "feat: issue autopilot destinations by right click"
```

---

## Task 7: Extract the renderer without changing output

**Objective:** Make room for marker/HUD presentation while preserving the now-correct visible ship path exactly.

**Files:**
- Create: `src/renderer.rs`
- Modify: `src/main.rs`
- Preserve: `src/shader.wgsl`

Move, without redesigning:

- `Vertex`
- `SceneUniform`
- `scene_uniform`
- `notched_ship_vertices`
- `Renderer`
- `VIEW_HEIGHT_METERS` or expose a shared camera helper so mouse conversion and rendering cannot drift

Prefer a shared pure camera contract in `renderer.rs`:

```rust
pub struct FixedCamera {
    pub view_height_meters: f32,
}

impl FixedCamera {
    pub fn half_extents(&self, width: u32, height: u32) -> Vec2;
    pub fn screen_to_world(&self, cursor: PhysicalPosition<f64>, width: u32, height: u32) -> Vec2;
}
```

This removes duplicate aspect math between click conversion and shader uniforms.

Preserve existing tests, especially:

- world scale at square aspect;
- 24 notched ship vertices;
- black inner ship smaller than cyan outer ship.

Verify before and after extraction:

```bash
cargo test --locked
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
```

Launch the real renderer after the move and visually confirm the cyan outlined notched ship still appears centered:

```bash
cargo run --locked
```

Do not add the marker or HUD until the extracted baseline is visibly confirmed.

**Commit:**

```bash
git add src/main.rs src/renderer.rs
git commit -m "refactor: isolate 2d renderer presentation"
```

---

## Task 8: Render the persistent red destination marker

**Objective:** Show the latest destination as a simple world-space red dot while preserving it after completion and clearing it on reset.

**Files:**
- Modify/Test: `src/renderer.rs`
- Modify: `src/shader.wgsl`
- Modify: `src/main.rs` render call

### Geometry

Generate a small filled circle as a triangle fan in world meters:

```rust
const DESTINATION_MARKER_RADIUS_METERS: f32 = 0.12;
const DESTINATION_MARKER_SEGMENTS: usize = 16;
```

Use a preallocated dynamic vertex buffer large enough for exactly `SEGMENTS * 3` vertices. Write the tiny marker vertex set only when a destination exists; no heap allocation inside algorithm or simulation code is affected.

Alternatively cache the generated `Vec<Vertex>` in `Renderer` and reuse its allocation every frame. Do not create a new GPU buffer per click or frame.

### Shader/pipeline

Split vertex entry points:

- `vs_ship`: existing local rotation + ship world translation.
- `vs_world`: world position multiplied by viewport scale, no ship transform.
- Existing fragment shader remains shared.

Create a second world-geometry pipeline using the same vertex format and scene bind group. Draw marker before or after ship; with no depth buffer and spatial separation either is acceptable, but choose and test one stable ordering.

### Tests

Write RED tests for:

1. Marker geometry has `16 * 3` vertices.
2. Every boundary vertex is approximately marker radius from destination.
3. Marker vertices are red.
4. `None` destination yields a zero draw count.
5. Completed inactive autopilot still provides `Some(destination)` to render.
6. Reset provides `None` and removes the draw.

Run:

```bash
cargo test renderer::tests::destination_marker_is_red_world_circle --locked
cargo test --locked
cargo check --locked
```

Launch and right-click several points. Confirm the red dot matches the clicked world point and remains after arrival/click replacement.

**Commit:**

```bash
git add src/renderer.rs src/shader.wgsl src/main.rs
git commit -m "feat: render persistent destination marker"
```

---

## Task 9: Render the in-window autopilot HUD label

**Objective:** Display `AUTOPILOT: ON` or `AUTOPILOT: OFF` without introducing a UI framework or font dependency.

**Files:**
- Modify/Test: `src/renderer.rs`
- Modify: `src/shader.wgsl`
- Modify: `src/main.rs` render call

### Minimal procedural glyphs

Implement a tiny 5×7 bitmap font for only the required characters:

```text
A U T O P I L : N F space
```

Convert each lit cell into two triangles in clip space. Build two immutable vertex buffers at renderer initialization:

- `AUTOPILOT: ON`
- `AUTOPILOT: OFF`

Select the appropriate buffer/draw count at render time. No per-frame text allocation or tessellation is needed.

Use a stable upper-left clip-space anchor with a small margin and legible scale. Suggested colors:

- ON: cyan/green high-visibility value.
- OFF: dim gray but still clearly visible.

The label must be drawn inside the render surface; changing only the OS window title does not satisfy this requirement.

### Shader/pipeline

Add `vs_hud`, which treats vertex positions as clip coordinates and does not apply world scale or ship transform. Create a small HUD pipeline sharing the fragment shader.

### Tests

Write RED tests for:

1. All required glyphs resolve; unsupported glyphs fail explicitly in tests rather than silently disappearing.
2. ON and OFF labels produce nonzero geometry.
3. ON and OFF geometry differs.
4. All label vertices remain within clip-space bounds.
5. HUD color differs visibly between ON and OFF.
6. Renderer selects ON geometry only while `autopilot.is_active()`.

Run:

```bash
cargo test renderer::tests::autopilot_hud_builds_on_and_off_labels --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Launch and confirm the label visibly switches ON on right-click and OFF only after arrival or R.

**Commit:**

```bash
git add src/renderer.rs src/shader.wgsl src/main.rs
git commit -m "feat: render autopilot status hud"
```

---

## Task 10: End-to-end application lifecycle tests

**Objective:** Lock the interactions among manual input, destination replacement, reset, autopilot completion, and persistent presentation state.

**Files:**
- Modify/Test: `src/main.rs`
- Modify/Test only if a seam is missing: `src/autopilot.rs`, `src/input.rs`

If `App` is too GPU-coupled to test, extract only the fixed-tick domain fields into a lightweight `GameState`:

```rust
struct GameState {
    simulation: Simulation,
    manual_input: InputController,
    autopilot: Autopilot,
    pending_destination: Option<Vec2>,
}

impl GameState {
    fn step_fixed_tick(&mut self);
}
```

`App` should own `Renderer`, `Window`, cursor state, and `GameState`; `GameState` must have no wgpu/window dependency.

Write RED tests for:

1. Manual W drives the ship when autopilot is inactive.
2. Manual W/A/D is ignored while autopilot is active.
3. A held manual key at activation remains suppressed after completion until release/re-press.
4. A second destination replaces the first before the next tick; latest wins.
5. A second destination replaces an active target during flight.
6. First decision occurs on the next tick.
7. Arrival deactivates autopilot but retains destination.
8. Reset clears destination, cancels autopilot, resets simulation, and outputs zero input that tick.
9. Reset and destination pending on one tick results in reset only.
10. Focus loss clears manual keys but does not cancel an active autopilot command.
11. Identical player command/tick sequences produce identical game state.

Run:

```bash
cargo test tests::manual_controls_are_ignored_during_autopilot --locked
cargo test tests::reset_cancels_autopilot_and_clears_marker --locked
cargo test tests::latest_destination_wins --locked
cargo test --locked
```

**Commit:**

```bash
git add src/main.rs src/autopilot.rs src/input.rs
git commit -m "test: lock autopilot application lifecycle"
```

---

## Task 11: Real runtime flight verification and tuning

**Objective:** Exercise the complete player-visible loop and tune only centralized parameters until the first controller is fast, chunky, and reliable.

**Files:**
- Modify only if needed: default configs in `src/autopilot.rs` and `src/flight_control/braking_pursuit.rs`

### Launch

```bash
cargo run --locked
```

### Runtime checklist

1. Initial ship and cyan outline remain visible.
2. HUD starts at `AUTOPILOT: OFF`.
3. Right-click ahead:
   - red marker appears exactly at click;
   - HUD becomes ON;
   - ship reacts on the next simulation tick.
4. Right-click left/right/behind:
   - ship chooses the expected turn direction;
   - W may combine with A/D.
5. Right-click a new point during flight:
   - marker moves immediately;
   - controller begins responding on the next tick, subject to current per-control holds.
6. Hold W/A/D during autopilot:
   - manual input does not alter autopilot behavior.
7. Observe controls indirectly through motion:
   - no rapid frame-perfect thrust or turn chatter;
   - turns/thrust occur in readable chunks;
   - response is still practical rather than sluggish.
8. Command a close target while moving quickly:
   - ship may overshoot;
   - turns around;
   - converges rather than giving up.
9. Command targets in all quadrants from different drift states.
10. On arrival:
    - ship is within the visible red-dot neighborhood;
    - translational motion is effectively stopped;
    - HUD switches OFF;
    - marker remains;
    - final heading/spin is not treated as failure.
11. Press R during travel:
    - ship resets;
    - autopilot goes OFF;
    - marker disappears;
    - held manual controls remain latched until release/re-press.
12. Press R after arrival:
    - persistent marker clears.

### Tuning discipline

Tune one category at a time:

1. **Algorithm convergence:** braking safety factor/buffer, approach angle, braking angle, turn gain/deadband.
2. **Arrival definition:** arrival radius and stopped-speed threshold.
3. **Human pacing:** thrust hold, left/right hold, decision cooldown.

After each tuning change:

```bash
cargo test autopilot::tests --locked
cargo test flight_control::braking_pursuit::tests --locked
```

Do not change physics constants merely to make the controller pass unless the user explicitly decides the underlying ship feel should change. The controller must adapt to the existing ship.

### Optional debug evidence during tuning

If convergence is hard to understand, temporarily log at most one line per control change—not every tick—with:

```text
tick, distance, speed, angle_error, mode, desired_input, effective_input
```

Remove temporary logs before final commit unless a user-visible debug mode is explicitly requested.

**Commit tuning separately:**

```bash
git add src/autopilot.rs src/flight_control/braking_pursuit.rs
git commit -m "tune: refine autopilot arrival behavior"
```

---

## Task 12: Final verification and branch review

**Objective:** Prove static correctness, deterministic behavior, live visibility, and clean scope before reporting completion.

Run:

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
cargo tree --depth 1
git status --short --branch
git log --oneline --decorate -12
```

Expected dependencies remain:

```text
bytemuck
glam
pollster
wgpu
winit
```

No text/UI dependency should be added for the procedural HUD.

Inspect scope:

```bash
git diff --stat 25a5ab6..HEAD
git diff --name-status 25a5ab6..HEAD
```

Expected production changes are limited to:

```text
src/autopilot.rs
src/flight_control/mod.rs
src/flight_control/braking_pursuit.rs
src/input.rs
src/main.rs
src/renderer.rs
src/shader.wgsl
```

plus this plan.

Do not create or publish a PR until explicitly requested and a Git remote exists.

---

## Acceptance Criteria

### Command and lifecycle

- [ ] Right-click maps to the correct fixed-camera world point.
- [ ] Latest destination replaces any prior destination.
- [ ] First decision occurs on the next fixed simulation tick.
- [ ] Autopilot deactivates only after position, speed, and zero-output completion conditions pass.
- [ ] Destination marker persists after completion.
- [ ] R cancels autopilot and clears the marker.

### Flight behavior

- [ ] Ship reaches representative ahead, side, and behind destinations.
- [ ] Ship corrects overshoot and initial lateral drift.
- [ ] Ship stops within the configured radius and speed thresholds.
- [ ] Final heading and angular velocity are not required.
- [ ] Controller uses only W/A/D through `ShipInput`.
- [ ] W may combine with A or D.
- [ ] Braking uses turn-around plus forward thrust, not hidden forces.

### Human-paced controls

- [ ] Every algorithm-requested control transition respects its per-control hold deadline.
- [ ] Every changed decision starts the global cooldown.
- [ ] Eligible controls may change while another remains locked.
- [ ] New destinations bypass global cooldown but preserve per-control holds.
- [ ] There is no emergency bypass.
- [ ] Runtime motion visibly avoids frame-perfect control chatter.

### Architecture and efficiency

- [ ] `Simulation` remains sole owner of authoritative motion.
- [ ] Flight algorithm is injected behind `FlightController`.
- [ ] Algorithm intent and effective coarse controls are distinct.
- [ ] Algorithm has no per-tick allocation or renderer/window dependency.
- [ ] Autopilot lifecycle has no wgpu/winit dependency.
- [ ] Marker/HUD are presentation-only.
- [ ] Manual input is ignored while active and safely suppressed across handoff.
- [ ] Headless deterministic command-sequence tests pass.

### Presentation

- [ ] Existing cyan outlined notched ship remains visible.
- [ ] Active/last destination is a visible red world-space dot.
- [ ] In-window HUD clearly shows `AUTOPILOT: ON/OFF`.
- [ ] HUD switches at the correct lifecycle boundaries.

### Verification

- [ ] Focused RED → GREEN tests were observed for each behavior slice.
- [ ] Full test suite passes.
- [ ] Formatting passes.
- [ ] Clippy passes with warnings denied.
- [ ] Diff integrity passes.
- [ ] Real wgpu runtime is launched and visually checked for ship, marker, HUD, replacement, overshoot correction, arrival, and reset.

---

## Known Risks and Mitigations

1. **Braking estimate ignores turn time.** The scalar stopping-distance equation assumes immediate braking alignment. Mitigate with a safety factor/buffer and runtime tuning; do not build trajectory search in this milestone.
2. **Coarse control timing can induce limit cycles.** Arrival radius, stopped-speed threshold, turn deadband, and hold/cooldown durations must be tuned together. Headless convergence tests prevent regressions.
3. **Linear drag complicates exact braking distance.** Treat thrust acceleration as a conservative approximation and tune empirically against the real simulation.
4. **Final angular motion may look odd.** This is accepted: translational state defines completion. Do not silently add heading requirements.
5. **Dynamic dispatch at fleet scale.** One virtual call per active ship per tick is expected to be negligible. Profile before replacing the clear trait seam.
6. **Procedural HUD scope.** Implement only the characters needed for ON/OFF. Do not grow it into a general text engine.
7. **Manual held-key handoff.** Suppress held controls when autopilot starts so automatic completion cannot be immediately undone by a physically held key.
8. **Visual verification cannot be inferred from compilation.** A runtime launch without screenshots proves pipeline initialization only; manually inspect the real window before claiming marker/HUD correctness.

---

## Definition of Done

The milestone is done when a player can launch the real application, right-click any visible point, watch the red marker and ON indicator appear, observe the ship navigate using visibly chunky W/A/D-like control bursts, replace the target mid-flight, see the ship correct overshoot and settle near the latest point with near-zero linear speed, retain the marker after OFF, and clear everything with R—while the complete headless and static verification suite passes.
