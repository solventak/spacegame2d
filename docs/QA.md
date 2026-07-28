# QA Guide — Relay Operations

This document describes how to bring the application to an interactive state and exercise it end-to-end. It covers both a **headless path** (no GPU or display required, suitable for CI and autonomous agents) and a **GUI path** (requires a desktop display).

When QA touches the HUD, controls, visual hierarchy, copy, icons, or interaction states, use the
[`Fleet Design System`](../ui/Fleet%20Design%20System/readme.md) as the visual acceptance reference.
Its [HUD kit guide](../ui/Fleet%20Design%20System/ui_kits/hud/README.md) indexes the reference
states and interactions; [`AGENTS.md`](../AGENTS.md#ui-design-system) provides the full agent
lookup table.

## Prerequisites

| Requirement | Headless path | GUI path |
|---|---|---|
| Rust toolchain (MSRV 1.91) | Yes | Yes |
| GPU adapter (wgpu-compatible) | No | Yes |
| Display (X11/Wayland/Win32/macOS) | No | Yes |
| WebView runtime (HUD) | No | Yes — WebView2 on Windows; WebKitGTK in an X11 child surface on Ubuntu |

Install Rust via [rustup](https://rustup.rs) if not already installed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

There is no auth/login gate, database, or external service dependency. The application is fully self-contained.

## Applications in this repository

| Crate | Binary | Type | Headless QA |
|---|---|---|---|
| `spacegame2d` | `spacegame2d` | GUI desktop app (wgpu/winit) | Simulation logic via `cargo test` |
| `simulation` | (library) | Game simulation library | `cargo test -p spacegame2d-simulation` |
| `protocol` | (library) | Versioned wire protocol | `cargo build -p spacegame2d-protocol` |
| `server` | `spacegame2d-server` | Authoritative simulation server | `cargo run -p spacegame2d-server -- 127.0.0.1:4000` |

## Headless QA path (agent-friendly)

This path exercises the full simulation game loop, flight control, autopilot, and fleet behavior without requiring a GPU or display. It is the primary QA path for CI and autonomous agents.

### Quick start

```sh
./scripts/qa-headless.sh
```

This script runs the complete test gate and a server smoke test, then reports a pass/fail summary. See [`scripts/qa-headless.sh`](../scripts/qa-headless.sh) for details.

### Manual steps

1. **Build all crates:**

   ```sh
   cargo build
   ```

2. **Run the test gate (format + lint + tests):**

   ```sh
   cargo fmt --check && cargo clippy -- -D warnings && cargo test
   ```

   This exercises the following interactive scenarios headlessly:

   - **Ship movement**: forward thrust accelerates along heading, turn keys apply angular torque, velocity/angular speed are capped, damping decays drift, combined inputs curve trajectory.
   - **Autopilot navigation**: right-click-style destination setting drives the ship to a target via the `ArrivalController`, converges from rest without orbiting, brakes near destination, settles with near-zero velocity.
   - **Reset**: `R`-equivalent `ResetSimulation` command preserves the monotonic tick and respawns the ship at the origin.
   - **World boundary**: ship at the exact 16 m radius survives; ship beyond the boundary is destroyed and removed; destruction emits an info log.
   - **Networked fleets**: each connected player receives an owned fleet, sees the other player's fleet, and right-click movement commands are broadcast and applied at the scheduled tick. Reset restores the deterministic swarm while preserving ownership.

3. **Server smoke test:**

   ```sh
   cargo run -p spacegame2d-server
   ```

   Expected output:

   ```
   server listening on the configured address
   ```

   The process exits with code 0 immediately.

### What the headless path validates

The `simulation` crate's test suite (`cargo test -p spacegame2d-simulation`) is the core interactive QA harness. It drives the same `Simulation::step(ShipInput)` loop that the GUI uses each tick, with inputs that mirror the keyboard controls. An agent can verify that the simulation logic behaves correctly by checking that all tests pass.

## GUI QA path (requires display + GPU)

This path launches the actual desktop application and exercises it through keyboard and mouse input. Use this when a display and GPU adapter are available.

### Launch

```sh
cargo run
```

For the embedded HUD on Ubuntu, install `libwebkit2gtk-4.1-0` and `xwayland`; builds additionally need `libwebkit2gtk-4.1-dev` and `pkg-config`. The client forces X11 so Wry hosts WebKitGTK as a child of the game window; Wayland sessions run through XWayland and need a valid `DISPLAY`. On Windows, verify the Evergreen WebView2 Runtime (install the Evergreen standalone runtime when absent). If WebKitGTK is blank on an affected Linux GPU/driver, retry once with `WEBKIT_DISABLE_DMABUF_RENDERER=1`; this is troubleshooting only, not an application default.

### HUD acceptance

1. Start the client without a server. Confirm the opaque connection form appears, its address is editable, and it does not auto-connect.
2. Submit an unreachable address. Within five seconds the editable form returns with a connection-failed message.
3. Start an attempt and select Cancel. The form is immediately editable again; a late result must not replace it.
4. Start the server and connect. Confirm the existing native window remains open and the WebView shrinks to the compact player panel; player 1 reports `01` / `CYAN` and player 2 reports `02` / `CORAL`.
5. Stop the server during play. Confirm the client returns to the full connection form instead of exiting.
6. Resize, maximize/restore, and change display scale in both pregame and connected modes. The pregame form fills the window and the compact panel stays 14 logical pixels from the upper-left; no independent GTK host window or stale gray surface appears.
7. Outside the compact panel, verify right-click destination, middle-drag camera, keyboard controls, and close behavior still work.
8. On an Ubuntu Wayland desktop, confirm logs record `display_backend="x11"` and the client is hosted through XWayland.

A window titled "Spacegame 2D" opens showing a black background with a subtle ring (the default 64 m death boundary) and the deterministic fleet spawned for the connected player and mirrored fleets received from the server.

### Controls

| Input | Action |
|---|---|
| `W` | Forward thrust (accelerates along ship heading) |
| `A` | Turn left (counterclockwise angular thrust) |
| `D` | Turn right (clockwise angular thrust) |
| `R` | Reset simulation (respawn ship at origin, preserve the monotonic tick) |
| Right-click | Set autopilot destination (ship navigates to clicked world position) |
| Middle-click drag | Pan the local camera; the world follows the pointer |
| Close window or `Alt+F4` | Exit |

### Meaningful interactions to verify

1. **Manual flight**: Press and hold `W` to thrust forward. The ship accelerates, reaches a maximum speed, and coasts with linear damping when `W` is released. Press `A` or `D` to rotate. The ship curves when thrusting and turning simultaneously.

2. **Autopilot navigation**: Right-click any visible point. A red marker appears at the clicked location. The ship turns to face the target and thrusts toward it, braking as it approaches. The ship settles near the target with near-zero velocity, and the marker clears when the autopilot deactivates. Right-click a new point mid-flight to redirect the ship.

3. **Reset**: Press `R` from either client. The server authorizes one reset event at the next fixed-tick boundary; both clients respawn the deterministic fleets, clear destinations, and preserve the monotonic tick.

4. **Camera**: Middle-drag the arena in both directions. The world follows the pointer, right-click targets remain aligned with the visible world, and `R` leaves the local camera position unchanged. At each edge, no more than about one-third of the visible axis lies outside the arena.

5. **World boundary**: Fly the ship beyond the visible ring (64 m radius). The ship is destroyed (disappears). Press `R` to respawn.

6. **Drone swarm**: Each player's fleet moves under the deterministic simulation and receives authoritative commands from the server. Drones that fly beyond the boundary are culled and disappear.

### Agent-driven GUI QA

When an agent needs to drive the GUI (e.g., in an environment with a virtual framebuffer), the application can be launched under `Xvfb` on Linux:

```sh
# Install Xvfb if needed (Debian/Ubuntu):
#   sudo apt-get install xvfb
xvfb-run -a cargo run
```

For screenshot-based verification, tools like `xdotool` can synthesize the keyboard/mouse events described above, and `import` (ImageMagick) or `grim` (Wayland) can capture the window.

## Test gate reference

The canonical test gate (run before any PR):

```sh
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

See [`AGENTS.md`](../AGENTS.md) for branch, PR, and ticketing conventions.
