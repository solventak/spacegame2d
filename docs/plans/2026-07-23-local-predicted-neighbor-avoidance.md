# Local Predicted-Neighbor Avoidance Implementation Plan

> **For Hermes:** Implement this plan directly in the current worktree, task-by-task, using strict RED → GREEN TDD. Do not use subagents; the user prefers direct, narrow repository edits.

**Goal:** Reduce close approaches and destination pile-ups among the 10 drones by adding a local, short-horizon predicted-neighbor avoidance acceleration to the existing velocity-arrival controller.

**Architecture:** Each drone receives a temporary borrowed slice of neighbor position/velocity snapshots. The existing arrival controller computes its normal goal-seeking acceleration, computes a soft avoidance acceleration from predicted closest approaches under constant-velocity neighbor motion, then blends the two before producing ordinary `ShipInput`. `main.rs` builds all observations and all controls from one immutable tick snapshot, then advances every drone through the authoritative `step_ship` transition.

**Tech Stack:** Rust, `glam::Vec2`, existing deterministic 60 Hz simulation, existing `ArrivalController`, existing wgpu renderer.

**Memory model:** Per tick, copy the 10 `ShipState` values and build at most 90 lightweight neighbor observations; no persistent world model, trajectory history, heap-owned data inside controllers, or spatial index.

---

## Settled Design Decisions

| Decision | First-pass choice |
|---|---|
| Planning scope | Each drone sees only the other nine drones; no theater, hierarchy, group object, enemies, obstacles, or player ship in the neighbor model. |
| Goal | All drones retain the same exact right-click destination. No slots or formation targets. |
| Neighbor lifetime | Borrowed, temporary observations rebuilt from an immutable state snapshot each fixed tick. Controllers do not own neighbors. |
| Prediction model | Constant velocity over a short configurable horizon; analytically evaluate time of closest approach and relative closing motion. |
| Avoidance shape | Soft comfort-radius penalty; zero outside the radius and smoothly increasing toward the center. No teleportation or direct physics force. |
| Reciprocal behavior | Each drone applies only half of the pairwise avoidance correction, borrowing ORCA/RVO's equal-responsibility principle without implementing its holonomic velocity half-plane solver. |
| Controller integration | Add the bounded reciprocal avoidance acceleration to the existing arrival acceleration before converting to heading/thrust input. |
| Simulation authority | All output remains `ShipInput`; only `step_ship` mutates drone physics. |
| Update ordering | Compute every drone's control from the same pre-step snapshot, then step all drones. Never observe partially updated peers. |
| Determinism | No runtime randomness. Initial placement remains deterministic and identical inputs produce identical results. |
| Complexity | O(N²) neighbor collection for N=10. No spatial hash until scale measurements justify it. |
| Success criterion | Fewer/shallower close approaches and a visibly looser destination cluster without preventing destination progress. |

## Explicit Non-Goals

- Full MPC, beam search, motion primitives, or branch pruning
- Collision resolution or physical contact impulses
- Guaranteed collision-free paths
- Assigned formation slots or target regions
- Cohesion, alignment, group centers, or flocking rules
- Persistent passing-side state, priorities, or deadlock resolution
- Obstacles, projectiles, enemies, threat fields, or visibility
- Spatial hashing, quadtrees, multithreading, or planner-rate throttling
- Renderer overlays beyond the existing drones and destination marker
- Moving the player ship into the drone neighbor set

## Current Baseline and Required Repair

The current `HEAD` (`ffdaf5b`) contains a partial neighbor-observation edit:

- `NeighborObservation` exists in `src/flight_control/mod.rs`.
- `FlightObservation` has a lifetime parameter, but its `neighbors` field is incorrectly declared `&'static [NeighborObservation]`.
- `FlightObservation::from_ship` now expects neighbors, while existing production and test call sites still pass two arguments.
- `ArrivalControllerConfig` contains `collision_radius_meters` and `collision_strength`, but the controller does not use them.
- `cargo test --locked` currently fails to compile with lifetime and missing-argument errors.

Implementation must preserve this committed work and repair it incrementally; do not reset or rewrite history.

## Behavioral Model

For one drone and one neighbor:

```text
relative_position = neighbor.position - self.position
relative_velocity = neighbor.velocity - self.velocity

t_closest = clamp(
    -dot(relative_position, relative_velocity)
      / max(length_squared(relative_velocity), epsilon),
    0,
    prediction_horizon
)

predicted_self     = self.position     + self.velocity     * t_closest
predicted_neighbor = neighbor.position + neighbor.velocity * t_closest
away               = predicted_self - predicted_neighbor
predicted_distance = length(away)
```

When `predicted_distance < comfort_radius`:

```text
penetration = 1 - predicted_distance / comfort_radius
weight      = penetration²
pair_avoidance += normalize(away) * avoidance_strength * weight
```

Each drone applies half of the pairwise correction. This borrows ORCA/RVO's equal-responsibility principle without implementing their holonomic velocity half-plane solver, which does not match our heading/thrust dynamics. The result remains a soft preferred-acceleration adjustment rather than a collision-free guarantee.

Then:

```text
goal_acceleration = (desired_velocity - current_velocity) * velocity_gain
combined_acceleration = goal_acceleration + clamped_reciprocal_avoidance
```

The existing heading/angular-velocity/thrust conversion consumes `combined_acceleration` unchanged.

Use a small epsilon for near-zero relative velocity and near-zero separation. In the exact-overlap fallback, derive a deterministic perpendicular direction from relative velocity when available; if both relative position and relative velocity are zero, contribute no direction in this first pass rather than introducing IDs or randomness.

## Initial Tunables

Rename the partial config fields so their intent is explicit:

```rust
pub prediction_horizon_seconds: f32, // start near 0.75
pub comfort_radius_meters: f32,      // start near 1.2 (slightly wider than one ship)
pub avoidance_strength: f32,        // start near 8.0 m/s²
pub max_avoidance_acceleration: f32, // start near 12.0 m/s²
```

These are starting values, not acceptance criteria. Keep the number of knobs small. Do not add separate collision, cohesion, side-preference, or density weights in this pass.

---

### Task 1: Repair the Borrowed Observation Seam

**Objective:** Make temporary neighbor slices a valid, compiling part of `FlightObservation` while proving empty-neighbor behavior remains the baseline.

**Files:**
- Modify: `src/flight_control/mod.rs`
- Modify: `src/autopilot.rs`
- Modify: `src/flight_control/arrival.rs`

**Step 1: Add a failing observation contract test**

In `src/flight_control/mod.rs`, add a test that creates a local `Vec<NeighborObservation>`, calls `FlightObservation::from_ship`, and asserts the resulting observation borrows exactly that slice.

Expected contract:

```rust
let neighbors = vec![NeighborObservation {
    position: Vec2::X,
    velocity: Vec2::Y,
}];
let observation = FlightObservation::from_ship(
    &ShipState::default(),
    Vec2::ZERO,
    &neighbors,
);
assert_eq!(observation.neighbors.len(), 1);
assert_eq!(observation.neighbors[0].position, Vec2::X);
```

**Step 2: Run the focused test and confirm RED**

Run:

```bash
cargo test --locked flight_control::tests::observation_borrows_temporary_neighbors
```

Expected: compilation fails because the field incorrectly requires `'static` and the impl lifetime is malformed.

**Step 3: Repair the lifetime contract**

Use:

```rust
pub struct FlightObservation<'a> {
    // existing state fields
    pub neighbors: &'a [NeighborObservation],
}

impl<'a> FlightObservation<'a> {
    pub fn from_ship(
        ship: &ShipState,
        destination: Vec2,
        neighbors: &'a [NeighborObservation],
    ) -> Self { ... }
}
```

Update every pre-existing no-neighbor test call to pass `&[]`. Update the player autopilot call to pass `&[]` temporarily; drone neighbor wiring belongs in Task 4.

**Step 4: Run focused and full tests**

Run:

```bash
cargo test --locked flight_control::tests::observation_borrows_temporary_neighbors
cargo test --locked
```

Expected: the observation test passes and the pre-neighbor suite compiles/passes unchanged.

**Step 5: Commit**

```bash
git add src/flight_control/mod.rs src/flight_control/arrival.rs src/autopilot.rs
git commit -m "fix: complete borrowed flight observations"
```

---

### Task 2: Implement Pure Closest-Approach Prediction

**Objective:** Add and verify the small deterministic math primitive used by avoidance, independently of ship controls.

**Files:**
- Modify: `src/flight_control/arrival.rs`

**Step 1: Write failing tests for closest approach**

Add focused tests for a private helper returning predicted separation information:

1. **Converging paths:** two entities moving toward one another have a future closest distance smaller than their current distance.
2. **Diverging paths:** closest time clamps to zero.
3. **Beyond horizon:** closest time clamps to `prediction_horizon_seconds`.
4. **Equal velocities:** no division by zero or NaN; closest time is zero.

Suggested seam:

```rust
struct ClosestApproach {
    time_seconds: f32,
    away: Vec2,
    distance: f32,
}

fn closest_approach(
    self_position: Vec2,
    self_velocity: Vec2,
    neighbor: NeighborObservation,
    horizon_seconds: f32,
) -> ClosestApproach
```

**Step 2: Run tests and confirm RED**

Run:

```bash
cargo test --locked flight_control::arrival::tests::closest_approach
```

Expected: failure because the helper does not exist.

**Step 3: Implement minimal analytical prediction**

Implement the formula in “Behavioral Model.” Clamp the horizon to non-negative values and guard relative speed with an epsilon.

**Step 4: Run focused tests and confirm GREEN**

Run:

```bash
cargo test --locked flight_control::arrival::tests::closest_approach
```

Expected: all closest-approach tests pass without NaN/Inf values.

**Step 5: Commit**

```bash
git add src/flight_control/arrival.rs
git commit -m "feat: predict closest neighbor approach"
```

---

### Task 3: Blend a Soft Avoidance Penalty Into Arrival

**Objective:** Make predicted neighbors alter desired acceleration without replacing the existing arrival behavior.

**Files:**
- Modify: `src/flight_control/arrival.rs`

**Step 1: Write failing pure avoidance tests**

Extract a private method/helper:

```rust
fn avoidance_acceleration(
    observation: &FlightObservation<'_>,
    config: ArrivalControllerConfig,
) -> Vec2
```

Add tests proving:

1. Empty neighbors produce `Vec2::ZERO`.
2. A neighbor outside the comfort radius produces zero avoidance.
3. A predicted crossing inside the horizon produces a nonzero vector away from closest approach.
4. A diverging neighbor does not create future-only avoidance.
5. A closer predicted approach produces a larger magnitude than a marginal one.
6. Many neighbors cannot exceed `max_avoidance_acceleration`.

**Step 2: Run focused tests and confirm RED**

Run:

```bash
cargo test --locked flight_control::arrival::tests::avoidance
```

Expected: failure because avoidance acceleration is not implemented.

**Step 3: Rename and complete the config**

Replace the partial generic fields:

```rust
collision_radius_meters
collision_strength
```

with the four explicit tunables listed under “Initial Tunables.” Do not add more policy knobs.

**Step 4: Implement the soft penalty**

For each neighbor:

- Predict closest approach.
- Ignore it when predicted separation is outside `comfort_radius_meters`.
- Add squared-penetration weighted acceleration away from the predicted neighbor position.
- Sum all contributions.
- Clamp only the total avoidance magnitude.

**Step 5: Blend with existing arrival acceleration**

Preserve:

```rust
goal_acceleration = velocity_error * velocity_gain;
```

Then use:

```rust
let desired_acceleration = goal_acceleration + avoidance_acceleration;
```

Do not apply movement directly to `ShipState`. Preserve existing heading and thrust conversion.

The early arrival return must not bypass active separation. Return idle only when the ship satisfies arrival conditions **and** avoidance acceleration is negligible. This allows a stopped drone near the destination to move aside for later arrivals.

**Step 6: Add controller-level regression tests**

Prove:

- With `neighbors: &[]`, existing arrival inputs are unchanged.
- A neighbor directly on the desired route changes the requested turn/thrust behavior.
- A nearby stopped neighbor at the destination prevents immediate settled-idle behavior when avoidance is non-negligible.

**Step 7: Run focused and full tests**

```bash
cargo test --locked flight_control::arrival
cargo test --locked
```

Expected: all arrival and existing tests pass.

**Step 8: Commit**

```bash
git add src/flight_control/arrival.rs
git commit -m "feat: add predicted neighbor avoidance to arrival"
```

---

### Task 4: Build Same-Tick Neighbor Snapshots for All Drones

**Objective:** Feed each drone the other nine pre-step states and advance all drones only after all controls have been chosen.

**Files:**
- Modify: `src/main.rs`
- Modify: `src/autopilot.rs`

**Step 1: Write failing neighbor-collection tests**

Extract a pure helper in `main.rs` or a small local simulation helper:

```rust
fn neighbor_observations(
    states: &[ShipState],
    self_index: usize,
) -> Vec<NeighborObservation>
```

Test:

- 10 states produce 9 observations for every valid index.
- The self state is excluded.
- Neighbor order is stable and follows source index order.
- Empty/single-state input returns an empty list.

**Step 2: Run and confirm RED**

```bash
cargo test --locked tests::neighbor_observations
```

Expected: failure because the helper does not exist.

**Step 3: Extend the autopilot call seam**

Change:

```rust
controls_for_tick(&mut self, ship: &ShipState)
```

to:

```rust
controls_for_tick(
    &mut self,
    ship: &ShipState,
    neighbors: &[NeighborObservation],
)
```

Pass neighbors into `FlightObservation::from_ship`. Player-ship call sites pass `&[]` in this milestone.

**Step 4: Compute controls from one immutable snapshot**

At each fixed tick:

```rust
let snapshot = self.drones.clone();
let mut controls = Vec::with_capacity(snapshot.len());

for (index, (ship, autopilot)) in snapshot
    .iter()
    .zip(self.drone_autopilots.iter_mut())
    .enumerate()
{
    let neighbors = neighbor_observations(&snapshot, index);
    controls.push(autopilot.controls_for_tick(ship, &neighbors));
}

for (ship, input) in self.drones.iter_mut().zip(controls) {
    step_ship(ship, input);
}
```

Do not query `self.drones` while stepping it. Do not persist neighbor vectors beyond the planning call.

**Step 5: Run focused and full tests**

```bash
cargo test --locked tests::neighbor_observations
cargo test --locked
```

Expected: all tests pass and previous single-ship behavior remains unchanged.

**Step 6: Commit**

```bash
git add src/main.rs src/autopilot.rs
git commit -m "feat: provide drones same-tick neighbor snapshots"
```

---

### Task 5: Add Deterministic Multi-Drone Behavior Regressions

**Objective:** Prove the local penalty improves separation without destroying goal progress.

**Files:**
- Modify: `src/main.rs` tests, or create a narrow pure test helper in `src/flight_control/arrival.rs` if it avoids application coupling

**Step 1: Add a deterministic swarm runner**

The test helper should:

- Accept initial `Vec<ShipState>`.
- Use one `ArrivalController` per drone.
- Use one shared destination.
- Build all neighbor observations from a pre-step snapshot.
- Compute all inputs before stepping any state.
- Advance with `step_ship` for a fixed number of ticks.
- Record minimum pairwise separation and final goal progress.

Do not use winit, wgpu, wall time, or randomness.

**Step 2: Write the RED comparison test**

Run the same controlled fixture twice:

- Baseline: `avoidance_strength = 0.0`.
- Avoidance: default avoidance config.

Use a compact but non-overlapping starting arrangement converging on one destination. Assert:

1. Avoidance produces a larger minimum pairwise separation than baseline.
2. Every avoidance-enabled drone makes material progress toward the destination.
3. Running the avoidance scenario twice yields identical final states and metrics.
4. No state contains NaN or infinity.

Avoid asserting a perfect formation or absolute zero collisions; this pass is a soft penalty experiment.

**Step 3: Run and confirm RED**

```bash
cargo test --locked swarm_avoidance_improves_minimum_separation
```

Expected: the comparison fails before neighbor observations are connected to controller behavior, or if the first tunables are ineffective.

**Step 4: Tune only the four planned constants**

Adjust only:

- prediction horizon
- comfort radius
- avoidance strength
- maximum avoidance acceleration

Do not add new rules merely to satisfy the test. Keep the broad acceptance ordering:

```text
separation improvement
without losing destination progress
```

**Step 5: Run full deterministic verification**

```bash
cargo fmt --all -- --check
cargo test --locked
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
```

Expected: all commands exit successfully with no warnings.

**Step 6: Commit**

```bash
git add src/main.rs src/flight_control/arrival.rs
git commit -m "test: verify local swarm separation"
```

---

### Task 6: Verify the Actual 10-Drone Scene

**Objective:** Confirm the tested behavior is visibly present in the real graphical application.

**Files:**
- No production changes unless runtime verification reveals a reproducible defect; any defect must receive a RED regression test before fixing.

**Step 1: Launch the real application**

```bash
cargo run --locked
```

Expected: wgpu shader, bind groups, pipeline, surface, and event loop initialize without panic.

**Step 2: Exercise representative commands**

In the actual window:

1. Right-click near the center so drones approach from multiple directions.
2. Observe whether they avoid exact stacking and settle into a loose cluster.
3. Right-click across the screen while they are moving.
4. Observe crossing and same-direction congestion.
5. Right-click into an already dense cluster.
6. Press `R`, confirm reset/cancellation remains functional.

**Step 3: Compare against an isolated A/B baseline**

For tuning verification only, temporarily run with `avoidance_strength = 0.0`, then restore the committed default and rerun the same click sequence. Do not commit the disabled baseline.

Visible acceptance requires:

- The enabled run has visibly fewer/deeper overlaps than OFF.
- Drones continue making progress rather than scattering indefinitely.
- No persistent high-amplitude oscillation appears around the target.
- The shared destination and original player ship behavior still work.

Capture screenshots or video for ON/OFF comparison when desktop tooling is available. Visual evidence overrides a passing numerical test if behavior still looks wrong.

**Step 4: Re-run final gates after any tuning**

```bash
cargo fmt --all -- --check
cargo test --locked
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
git status --short --branch
```

**Step 5: Commit final verified tuning only if values changed**

```bash
git add src/flight_control/arrival.rs
git commit -m "tune: balance drone goal and separation costs"
```

---

## Research Influence

The quick review found four relevant bodies of work:

- **Reynolds steering behaviors (1999):** supports our separation of steering from locomotion and combining goal-seeking with local avoidance. This validates layering avoidance into the controller rather than changing `Simulation`.
- **Reynolds flocking / modern flocking reviews:** identify separation as the minimal local rule, while alignment and cohesion are separate behaviors. Therefore this pass adds no alignment, cohesion, or formation logic.
- **RVO/ORCA (van den Berg et al.):** uses relative position/velocity, a short collision horizon, local observations, and reciprocal half-responsibility. We borrow those ingredients, but not the full linear-program velocity solver because our drones cannot instantly choose arbitrary 2D velocities.
- **ORCA-MPC and conflict-based MPC:** show that dynamics-aware predictive planning and explicit conflict constraints are valuable in dense/tight scenes, but they are substantially beyond the current 10-drone experiment. They remain follow-up options if the soft penalty produces deadlocks or insufficient safety.

The plan is therefore intentionally a **dynamics-compatible reciprocal steering penalty**, not a claim of guaranteed ORCA-style collision freedom.

## Sources Consulted

- Craig Reynolds, “Steering Behaviors For Autonomous Characters” (GDC 1999): https://www.red3d.com/cwr/steer/gdc99
- van den Berg et al., “Optimal Reciprocal Collision Avoidance for Multi-Agent Navigation”: https://gamma.cs.unc.edu/ORCA
- Cheng et al., “Decentralized Navigation of Multiple Agents Based on ORCA and Model Predictive Control” (IROS 2017): http://www.linliang.net/wp-content/uploads/2017/07/IROS17_MultiAgentNavi.pdf
- Ali et al., “State-of-the-Art Flocking Strategies for the Collective Motion of Multi-Robots” (Machines, 2024): https://www.mdpi.com/2075-1702/12/10/739
- Tajbakhsh et al., “Conflict-Based Model Predictive Control for Scalable Multi-Robot Motion Planning”: https://arxiv.org/html/2303.01619v3

---

## Acceptance Criteria

### Functional

- A right-click still sends all 10 drones toward one shared destination.
- Each drone receives exactly the other nine drones as temporary observations.
- Neighbor state is sampled before any drone advances for that tick.
- Constant-velocity closest-approach prediction is bounded by a short horizon.
- Avoidance is zero outside the comfort radius and increases smoothly inside it.
- Avoidance influences ordinary `ShipInput`; it never mutates physics directly.
- Drones at the destination can still react to close late arrivals rather than permanently ignoring neighbors.
- Player ship controls/autopilot remain unchanged and do not participate in drone avoidance.

### Determinism

- Same initial states, destination, and tick count produce byte-for-byte/equality-identical ship states.
- Neighbor iteration order is stable.
- No wall-clock timing, random tie-breaks, or unordered collections affect controls.
- No NaN/Inf states are produced by zero relative speed or near-zero separation.

### Behavioral

- In a controlled deterministic scenario, avoidance improves minimum pairwise separation over a strength-zero baseline.
- Drones still make material progress toward the shared destination.
- In the real scene, enabled avoidance visibly reduces pile-ups and deep overlaps.
- The result remains a loose emergent cluster, not a rigid ring or formation.

### Quality Gates

```bash
cargo fmt --all -- --check
cargo test --locked
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
git diff --check
cargo run --locked
```

## Known Risks and Follow-Up Signals

1. **Shared-point pressure:** Goal attraction and separation may create oscillation near the destination. First tune the four existing constants; do not immediately add slots or cohesion.
2. **Reciprocal symmetry:** Two identical agents can choose mirrored responses. If visible deadlocks or side-switching remain after this pass, stable ID-based passing preference is a separate follow-up feature.
3. **Constant-velocity neighbor prediction:** It assumes peers maintain current velocity even though they are also steering. This is acceptable for the short first-pass horizon; full trajectory exchange or MPC is deferred.
4. **Soft safety only:** This reduces collision likelihood but does not guarantee collision-free motion. Physical collision resolution remains a separate simulation concern.
5. **Autopilot completion:** If a drone deactivates at a temporary equilibrium and later stops responding to changing neighbors, add a focused failing lifecycle test before deciding whether drone controllers should remain continuously active near a retained destination.
6. **O(N²) scaling:** Correct and trivial for 10 drones. Introduce a spatial index only after a larger swarm benchmark identifies neighborhood collection as a real cost.
