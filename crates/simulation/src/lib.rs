//! Core simulation for `spacegame2d`: a fixed-timestep 2D space ship model
//! with autopilot navigation and a drone fleet.
//!
//! # Modules
//!
//! - [`simulation`] — the ship physics integrator, world boundary, and the
//!   [`Simulation`][simulation::Simulation] driver that ticks a single
//!   player-controlled ship.
//! - [`autopilot`] — high-level destination targeting built on top of a
//!   swappable [`FlightController`][flight_control::FlightController].
//! - [`flight_control`] — the trait abstraction and concrete controllers
//!   (velocity-arrival) that turn a flight observation into ship input.
//! - [`fleet`] — a collection of autonomous [`Unit`][fleet::Unit]s (drones)
//!   sharing an arena, with spawn, step, cull, and reset operations.
//!
//! # Timestep
//!
//! The simulation runs at a fixed [`SIMULATION_HZ`][simulation::SIMULATION_HZ]
//! of 60 Hz. All integration constants are expressed in SI units and consumed
//! per-tick via [`FIXED_DT_SECONDS`][simulation::FIXED_DT_SECONDS].

pub mod autopilot;
pub mod combat;
pub mod command;
pub mod config;
pub mod fleet;
pub mod flight_control;
pub mod hitbox;
pub mod simulation;
pub mod snapshot;
pub mod structure;

pub use combat::{
    CombatState, FIRE_INTERVAL_TICKS, FIRING_TOLERANCE_RADIANS, HullState, MAX_HULL,
    MUZZLE_OFFSET_METERS, TARGET_HIT_RADIUS_METERS, TURRET_TRACKING_RADIANS_PER_SECOND,
    TurretState, WEAPON_DAMAGE, WEAPON_RANGE_METERS,
};
pub use command::{
    Command, CommandScheduler, PlayerId, RecordedCommand, Unit, UnitId, UnitIdAllocationError,
    World,
};
pub use config::{
    AvoidanceConfig, DEFAULT_FLEET_SIZE, DEFAULT_WORLD_RADIUS_METERS, MAX_PLAYERS,
    SimulationConfig, SimulationConfigError,
};
pub use hitbox::{
    Circle, DEFAULT_SHIP_HITBOX_RADIUS_METERS, Hitbox, HitboxError, HitboxShape, PositionedHitbox,
};
pub use simulation::SimulationEvent;
pub use structure::{StaticStructure, StaticStructureId, StaticStructureKind};
