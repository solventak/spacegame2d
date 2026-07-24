# SWA-5: World arena — 100 m death boundary with subtle visible ring

> Linear ticket: https://linear.app/swarm123/issue/SWA-5/world-arena-100-m-death-boundary-with-subtle-visible-ring
> Risk Level: RISKY

## Plan

1. **Add `WORLD_RADIUS_M` constant and boundary helper in `simulation.rs`**: Add `pub const WORLD_RADIUS_M: f32 = 100.0;` with a one-line comment next to the existing metric constants. Add `pub fn is_out_of_bounds(position: Vec2) -> bool { position.length() > WORLD_RADIUS_M }`. Add unit tests for the helper (exact-boundary survives, epsilon-beyond dies).

2. **Change `Simulation::ship` to `Option<ShipState>`**: Update the field type from `ShipState` to `Option<ShipState>`. Change `ship()` accessor to return `Option<&ShipState>`. In `step()`, skip ship physics when `None` but still increment tick. In `apply_command(ResetShip)`, set `ship = Some(ShipState::default())`. Update `Default` impl to `Some(ShipState::default())`. Update existing simulation tests to unwrap/expect the Option.

3. **Add `ResetSimulation` command variant**: Add `ResetSimulation` to the `SimulationCommand` enum. In `apply_command(ResetSimulation)`, set `ship = Some(ShipState::default())` and reset `tick = 0`. Keep existing `ResetShip` variant for backward compatibility (it resets ship only, preserves tick). Update `input.rs` `take_command()` to emit `ResetSimulation` instead of `ResetShip` on R key press. Update `input.rs` tests to expect `ResetSimulation`.

4. **Add boundary kill in `Simulation::step()`**: After `step_ship` in `step()`, if ship is `Some(ref s)` and `is_out_of_bounds(s.position)`, set `self.ship = None` and emit `log::info!("ship destroyed: out of bounds at ({:.1}, {:.1})", s.position.x, s.position.y)`. Add unit test: ship positioned at exactly `WORLD_RADIUS_M` survives one tick; ship at `WORLD_RADIUS_M + 0.01` is removed (ship becomes `None`).

5. **Add boundary kill for drones in `main.rs`**: After the drone `step_ship` loop in `about_to_wait`, iterate drones and log `info!` for any out-of-bounds drone, then call `self.drones.retain(|d| !simulation::is_out_of_bounds(d.position))`. This removes dead drones from the vec so they are not rendered or simulated on subsequent ticks.

6. **Update R-key reset in `main.rs` to full simulation reset**: In the `about_to_wait` command-handling block, when the command is `ResetSimulation`: call `self.simulation.apply_command(ResetSimulation)` (resets ship + tick), set `self.drones = initial_drone_positions()` (respawns from deterministic seeded positions), cancel all drone autopilots, cancel player autopilot, and clear `pending_destination`. Change the drone reset from the `for drone in &mut self.drones { *drone = ShipState::default(); }` loop to `self.drones = initial_drone_positions()`.

7. **Handle player ship absence in rendering**: Change `Renderer::render()` signature from `ship: &ShipState` to `ship: Option<&ShipState>`. When `None`, skip writing `scene_buffers[0]` and skip the player draw call. Update the `RedrawRequested` handler in `App::window_event` to pass `self.simulation.ship()` (now `Option<&ShipState>`) directly.

8. **Increase `VIEW_HEIGHT_METERS` to make the ring visible**: Change `VIEW_HEIGHT_METERS` from `20.0` to `220.0` so the full 200 m diameter ring is on-screen with 10 m margin. Update the `scene_uniform_uses_fixed_world_scale` test in `main.rs` to match the new viewport scale (`1.0 / 110.0`).

9. **Add ring rendering**: Generate thin-annulus vertices at `WORLD_RADIUS_M` (128 segments, inner radius 99.5 m, outer radius 100.5 m, 768 vertices as non-indexed triangles) with a dim blue color `[0.0, 0.0, 0.15, 1.0]`. Store in a separate `wgpu::Buffer` created in `Renderer::new`. In `Renderer::render()`, after drawing all ships, bind the ring vertex buffer and issue a draw call for the ring vertices. In `shader.wgsl`, add `let is_ring = input.color.b > 0.1 && input.color.r < 0.01 && input.color.g < 0.01;` and use `select(world, input.position, is_ring)` to render ring vertices at literal world coordinates (no ship transform).

10. **Add unit tests for reset-after-removal**: In `simulation.rs`, add a test that: creates a `Simulation`, kills the ship by stepping it out of bounds, verifies `ship()` returns `None` and tick advanced, then calls `apply_command(ResetSimulation)` and verifies `ship()` returns `Some(default)` and tick is 0.

11. **Run the test gate**: Execute `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. Fix any formatting, lint, or test failures before pushing.

## Files Touched

- `src/simulation.rs` — Add `WORLD_RADIUS_M` constant, `is_out_of_bounds()` helper, change `ship` field to `Option<ShipState>`, add `ResetSimulation` command variant, add boundary kill logic in `step()`, update existing tests, add new boundary/reset tests.
- `src/main.rs` — Add drone boundary kill/removal via `retain()`, update reset to `ResetSimulation` with `initial_drone_positions()` respawn, change `VIEW_HEIGHT_METERS` to 220.0, handle `Option<&ShipState>` in render path, add ring vertex buffer and draw call, update `scene_uniform_uses_fixed_world_scale` test.
- `src/input.rs` — Change `take_command()` to emit `ResetSimulation` on R key; update `reset_is_one_shot` test.
- `src/shader.wgsl` — Add `is_ring` branch to render ring vertices at literal world coordinates without ship transform.

## API Surface

- `Simulation::ship()` return type changes from `&ShipState` to `Option<&ShipState>` — **breaking public API change**.
- `SimulationCommand` gains new `ResetSimulation` variant — public enum expansion (non-breaking for existing match arms if they use `_` or are updated).
- `simulation::WORLD_RADIUS_M` — new public constant.
- `simulation::is_out_of_bounds(Vec2) -> bool` — new public function.

## Risk Level

RISKY — Cross-module change touching `simulation.rs`, `main.rs`, `input.rs`, and `shader.wgsl`. Public API change (`Simulation::ship()` return type). GPU rendering changes (ring geometry, new vertex buffer, shader branch). View height change alters the entire visual scale. Data model change (`Option<ShipState>`) ripples through simulation, rendering, and tests.
