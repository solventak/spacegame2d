use crate::combat::{
    FIRE_INTERVAL_TICKS, FIRING_TOLERANCE_RADIANS, MUZZLE_OFFSET_METERS, TARGET_HIT_RADIUS_METERS,
    TURRET_TRACKING_RADIANS_PER_SECOND, WEAPON_DAMAGE, WEAPON_RANGE_METERS,
};
use crate::command::{CommandScheduler, UnitId, World};
use crate::config::SimulationConfig;
use crate::flight_control::{AvoidanceEntityId, NeighborObservation, NeighborRelationship};
use crate::hitbox::Hitbox;
use glam::Vec2;
use spacegame2d_protocol::{AuthoritativeCommand, Tick};
use std::collections::{BTreeMap, BTreeSet};

/// Simulation tick rate in hertz. The integrator advances in fixed
/// [`FIXED_DT_SECONDS`] steps regardless of wall-clock frame timing.
pub const SIMULATION_HZ: u32 = 60;
/// Fixed timestep derived from [`SIMULATION_HZ`], in seconds.
pub const FIXED_DT_SECONDS: f32 = 1.0 / SIMULATION_HZ as f32;
/// Ship mass in kilograms, used to convert thrust into linear acceleration.
pub const SHIP_MASS_KG: f32 = 1.0;
/// Forward thrust force in newtons applied while `thrust` is held.
pub const FORWARD_THRUST_NEWTONS: f32 = 8.0;
/// Hard cap on ship speed in meters per second.
pub const MAX_SPEED_METERS_PER_SECOND: f32 = 8.0;
/// Linear velocity damping rate per second; the ship coasts to a stop without
/// active thrust.
pub const LINEAR_DAMPING_PER_SECOND: f32 = 0.8;
/// Ship moment of inertia in kg·m², used to convert torque into angular
/// acceleration.
pub const MOMENT_OF_INERTIA_KG_M2: f32 = 0.25;
/// Angular thrust torque in newton-meters applied while turning.
pub const ANGULAR_THRUST_NEWTON_METERS: f32 = 2.0;
/// Hard cap on angular speed in radians per second.
pub const MAX_ANGULAR_SPEED_RADIANS_PER_SECOND: f32 = 3.0;
/// Angular velocity damping rate per second; rotation decays without active
/// turn input.
pub const ANGULAR_DAMPING_PER_SECOND: f32 = 2.5;
const VELOCITY_EPSILON: f32 = 0.0001;
/// Default radius kept as a convenience for callers that render the default
/// prototype configuration. Authoritative simulation code reads the radius
/// from [`SimulationConfig`].
pub const WORLD_RADIUS_M: f32 = crate::config::DEFAULT_WORLD_RADIUS_METERS;

/// Per-tick discrete input applied to a ship by the player or an autopilot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlightInput {
    /// Apply forward thrust along the current heading.
    pub thrust: bool,
    /// Apply counterclockwise (left) angular thrust.
    pub turn_left: bool,
    /// Apply clockwise (right) angular thrust.
    pub turn_right: bool,
}

/// Integrated kinematic state of a single ship.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipState {
    /// Position in arena-space meters, origin at the arena center.
    pub position: Vec2,
    /// Velocity in meters per second.
    pub velocity: Vec2,
    /// Heading angle in radians. `0.0` points along +Y; positive is
    /// counterclockwise.
    pub heading_radians: f32,
    /// Angular velocity in radians per second.
    pub angular_velocity_radians_per_second: f32,
}

impl Default for ShipState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading_radians: 0.0,
            angular_velocity_radians_per_second: 0.0,
        }
    }
}

/// Fixed-timestep driver for the autopilot drone world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppliedAuthoritativeCommands {
    pub reset_applied: bool,
}

pub struct Simulation {
    tick: Tick,
    pub world: World,
    pub commands: CommandScheduler,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new(SimulationConfig::default())
    }
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        Self {
            tick: Tick::default(),
            world: World::new(config),
            commands: CommandScheduler::default(),
        }
    }
}

impl Simulation {
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// Align a mirror simulation with the authoritative server clock.
    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
    }

    pub fn with_world_radius(world_radius: f32) -> Self {
        let config = SimulationConfig::default()
            .with_world_radius_meters(world_radius)
            .expect("test world radius must be valid");
        Self::new(config)
    }

    pub fn world_radius(&self) -> f32 {
        self.config().world_radius_meters()
    }

    pub fn config(&self) -> SimulationConfig {
        self.world.config()
    }
    pub fn world(&self) -> &World {
        &self.world
    }
    pub fn schedule_authoritative(&mut self, cmd: &AuthoritativeCommand) -> bool {
        if self.world.validate_authoritative(cmd).is_err() {
            return false;
        }
        self.schedule_authoritative_trusted(cmd)
    }

    pub fn apply_due_commands(
        &mut self,
        scheduled: &mut BTreeMap<Tick, Vec<AuthoritativeCommand>>,
    ) -> AppliedAuthoritativeCommands {
        let current_tick = self.tick;
        let due_ticks: Vec<Tick> = scheduled
            .range(..=current_tick)
            .map(|(tick, _)| *tick)
            .collect();
        let mut result = AppliedAuthoritativeCommands::default();
        for tick in due_ticks {
            let Some(commands) = scheduled.remove(&tick) else {
                continue;
            };
            for mut command in commands {
                if result.reset_applied {
                    break;
                }
                if command.execute_tick < current_tick {
                    command.execute_tick = current_tick;
                }
                if matches!(
                    command.command,
                    spacegame2d_protocol::CommandData::ResetSimulation
                ) {
                    scheduled.clear();
                    self.commands.clear_pending();
                    result.reset_applied = true;
                }
                self.schedule_authoritative_trusted(&command);
            }
        }
        result
    }

    /// Schedule a command already validated by the authoritative server.
    /// Client mirrors use this path because they do not maintain the server's
    /// complete ownership registry.
    pub fn schedule_authoritative_trusted(&mut self, cmd: &AuthoritativeCommand) -> bool {
        let Ok(command) = Box::<dyn crate::command::Command>::try_from(cmd) else {
            return false;
        };
        self.commands.schedule(cmd.execute_tick, command);
        true
    }

    /// Advance one deterministic tick by applying queued commands, stepping the
    /// authoritative World units, and deriving transient boundary events.
    pub fn step(&mut self) -> Result<Vec<SimulationEvent>, crate::command::CommandExecutionError> {
        self.commands.execute_pending(self.tick, &mut self.world)?;
        let observations = avoidance_observations(&self.world);
        for unit in self.world.units.iter_mut() {
            let owner = unit.owner;
            let neighbors = observations
                .iter()
                .filter(|neighbor| neighbor.entity_id != AvoidanceEntityId::Unit(unit.id))
                .map(|neighbor| NeighborObservation {
                    entity_id: neighbor.entity_id,
                    position: neighbor.position,
                    velocity: neighbor.velocity,
                    hitbox: neighbor.hitbox,
                    relationship: match neighbor.entity_id {
                        AvoidanceEntityId::StaticStructure(_) => {
                            NeighborRelationship::StaticStructure
                        }
                        AvoidanceEntityId::Unit(_)
                            if owner.is_some() && owner == neighbor.owner =>
                        {
                            NeighborRelationship::Friendly
                        }
                        AvoidanceEntityId::Unit(_) => NeighborRelationship::Opposing,
                    },
                })
                .collect::<Vec<_>>();
            let input = unit.autopilot.controls_for_tick_with_hitbox(
                &unit.state,
                unit.hitbox(),
                &neighbors,
            );
            step_ship(&mut unit.state, input);
        }
        let observations = combat_observations(&self.world);
        let mut events = Vec::new();
        let mut damage = BTreeMap::new();
        let mut shooters = observations
            .iter()
            .map(|observation| observation.id)
            .collect::<Vec<_>>();
        shooters.sort_unstable();
        for shooter_id in shooters {
            let shooter_index = self
                .world
                .units
                .iter()
                .position(|unit| unit.id == shooter_id)
                .expect("combat observation must reference a live unit");
            let shooter = &mut self.world.units[shooter_index];
            let previous_target = shooter.combat.turret.target;
            let target = valid_target(
                previous_target,
                shooter.owner,
                shooter.state.position,
                &observations,
            )
            .or_else(|| nearest_hostile(shooter.owner, shooter.state.position, &observations));
            if target != previous_target {
                shooter.combat.turret.cooldown_ticks_remaining = 0;
            }
            shooter.combat.turret.target = target;
            if shooter.combat.turret.cooldown_ticks_remaining > 0 {
                shooter.combat.turret.cooldown_ticks_remaining -= 1;
            }
            let Some(target_id) = target else {
                continue;
            };
            let target_position = observations
                .iter()
                .find(|observation| observation.id == target_id)
                .expect("valid target must be observed")
                .position;
            let desired_world_heading = heading_toward(target_position - shooter.state.position);
            let desired_local_heading =
                wrap_angle(desired_world_heading - shooter.state.heading_radians);
            let delta =
                wrap_angle(desired_local_heading - shooter.combat.turret.local_heading_radians);
            let max_turn = TURRET_TRACKING_RADIANS_PER_SECOND * FIXED_DT_SECONDS;
            shooter.combat.turret.local_heading_radians = wrap_angle(
                shooter.combat.turret.local_heading_radians + delta.clamp(-max_turn, max_turn),
            );
            let world_heading = wrap_angle(
                shooter.state.heading_radians + shooter.combat.turret.local_heading_radians,
            );
            let aligned =
                wrap_angle(desired_world_heading - world_heading).abs() <= FIRING_TOLERANCE_RADIANS;
            if !aligned || shooter.combat.turret.cooldown_ticks_remaining != 0 {
                continue;
            }
            let direction = forward_from_heading(world_heading);
            let muzzle_origin = shooter.state.position + direction * MUZZLE_OFFSET_METERS;
            let ray_endpoint = muzzle_origin + direction * WEAPON_RANGE_METERS;
            let hit = first_hostile_hit(
                shooter.id,
                shooter.owner,
                muzzle_origin,
                direction,
                &observations,
            );
            let hit_unit_id = hit.map(|(unit_id, _)| unit_id);
            let impact_position = hit.map_or(ray_endpoint, |(_, position)| position);
            if let Some(hit_unit_id) = hit_unit_id {
                *damage.entry(hit_unit_id).or_insert(0) += WEAPON_DAMAGE;
            }
            shooter.combat.turret.cooldown_ticks_remaining = FIRE_INTERVAL_TICKS;
            events.push(SimulationEvent::ShotFired {
                tick: self.tick,
                shooter_id,
                muzzle_origin,
                ray_endpoint,
                impact_position,
                hit_unit_id,
            });
        }
        for (unit_id, amount) in damage {
            if let Some(unit) = self.world.unit_mut(unit_id) {
                unit.combat.hull.current = unit.combat.hull.current.saturating_sub(amount);
            }
        }
        let world_radius = self.world_radius();
        self.world.units.retain(|unit| {
            if unit.combat.hull.current == 0 {
                events.push(SimulationEvent::HullDepleted {
                    tick: self.tick,
                    unit_id: unit.id,
                    position: unit.state.position,
                });
                false
            } else if is_out_of_bounds(unit.state.position, world_radius) {
                events.push(SimulationEvent::BoundaryCrossed {
                    tick: self.tick,
                    unit_id: unit.id,
                    position: unit.state.position,
                });
                false
            } else {
                true
            }
        });
        let live_ids = self
            .world
            .units
            .iter()
            .map(|unit| unit.id)
            .collect::<BTreeSet<_>>();
        for unit in &mut self.world.units {
            if unit
                .combat
                .turret
                .target
                .is_some_and(|target| !live_ids.contains(&target))
            {
                unit.combat.turret.target = None;
            }
        }
        events.sort_by_key(simulation_event_sort_key);
        self.tick = self.tick.increment(Tick::new(1));
        Ok(events)
    }
}

/// Immutable unit state sampled at the start of a simulation tick.
#[derive(Clone, Copy)]
struct TickAvoidanceObservation {
    entity_id: AvoidanceEntityId,
    owner: Option<crate::command::PlayerId>,
    position: Vec2,
    velocity: Vec2,
    hitbox: Hitbox,
}

fn avoidance_observations(world: &World) -> Vec<TickAvoidanceObservation> {
    let mut observations: Vec<TickAvoidanceObservation> = world
        .units
        .iter()
        .map(|unit| TickAvoidanceObservation {
            entity_id: AvoidanceEntityId::Unit(unit.id),
            owner: unit.owner,
            position: unit.state.position,
            velocity: unit.state.velocity,
            hitbox: unit.hitbox(),
        })
        .collect();
    observations.extend(
        world
            .structures()
            .iter()
            .map(|structure| TickAvoidanceObservation {
                entity_id: AvoidanceEntityId::StaticStructure(structure.id()),
                owner: Some(structure.owner()),
                position: structure.position(),
                velocity: Vec2::ZERO,
                hitbox: structure.hitbox(),
            }),
    );
    observations.sort_unstable_by_key(|observation| observation.entity_id);
    observations
}

#[derive(Clone, Copy)]
struct CombatObservation {
    id: UnitId,
    owner: Option<crate::command::PlayerId>,
    position: Vec2,
}

fn combat_observations(world: &World) -> Vec<CombatObservation> {
    let mut observations = world
        .units
        .iter()
        .map(|unit| CombatObservation {
            id: unit.id,
            owner: unit.owner,
            position: unit.state.position,
        })
        .collect::<Vec<_>>();
    observations.sort_unstable_by_key(|observation| observation.id);
    observations
}

fn is_hostile(
    owner: Option<crate::command::PlayerId>,
    other_owner: Option<crate::command::PlayerId>,
) -> bool {
    matches!((owner, other_owner), (Some(owner), Some(other_owner)) if owner != other_owner)
}

fn valid_target(
    target: Option<UnitId>,
    owner: Option<crate::command::PlayerId>,
    position: Vec2,
    observations: &[CombatObservation],
) -> Option<UnitId> {
    let target = target?;
    observations
        .iter()
        .find(|observation| {
            observation.id == target
                && is_hostile(owner, observation.owner)
                && (observation.position - position).length_squared()
                    <= WEAPON_RANGE_METERS * WEAPON_RANGE_METERS
        })
        .map(|observation| observation.id)
}

fn nearest_hostile(
    owner: Option<crate::command::PlayerId>,
    position: Vec2,
    observations: &[CombatObservation],
) -> Option<UnitId> {
    observations
        .iter()
        .filter(|observation| is_hostile(owner, observation.owner))
        .filter_map(|observation| {
            let distance_squared = (observation.position - position).length_squared();
            (distance_squared <= WEAPON_RANGE_METERS * WEAPON_RANGE_METERS)
                .then_some((distance_squared, observation.id))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)))
        .map(|(_, id)| id)
}

fn heading_toward(vector: Vec2) -> f32 {
    (-vector.x).atan2(vector.y)
}

fn forward_from_heading(heading: f32) -> Vec2 {
    Vec2::new(-heading.sin(), heading.cos())
}

fn first_hostile_hit(
    shooter_id: UnitId,
    owner: Option<crate::command::PlayerId>,
    origin: Vec2,
    direction: Vec2,
    observations: &[CombatObservation],
) -> Option<(UnitId, Vec2)> {
    observations
        .iter()
        .filter(|observation| observation.id != shooter_id && is_hostile(owner, observation.owner))
        .filter_map(|observation| {
            let offset = observation.position - origin;
            let projection = offset.dot(direction);
            if !(0.0..=WEAPON_RANGE_METERS).contains(&projection) {
                return None;
            }
            let perpendicular_squared = offset.length_squared() - projection * projection;
            let radius_squared = TARGET_HIT_RADIUS_METERS * TARGET_HIT_RADIUS_METERS;
            if perpendicular_squared > radius_squared {
                return None;
            }
            let entry_distance =
                (projection - (radius_squared - perpendicular_squared).sqrt()).max(0.0);
            Some((entry_distance, observation.id))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)))
        .map(|(entry_distance, id)| (id, origin + direction * entry_distance))
}

fn simulation_event_sort_key(event: &SimulationEvent) -> (u8, UnitId) {
    match event {
        SimulationEvent::ShotFired { shooter_id, .. } => (0, *shooter_id),
        SimulationEvent::HullDepleted { unit_id, .. } => (1, *unit_id),
        SimulationEvent::BoundaryCrossed { unit_id, .. } => (2, *unit_id),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationEvent {
    ShotFired {
        tick: Tick,
        shooter_id: UnitId,
        muzzle_origin: Vec2,
        ray_endpoint: Vec2,
        impact_position: Vec2,
        hit_unit_id: Option<UnitId>,
    },
    HullDepleted {
        tick: Tick,
        unit_id: UnitId,
        position: Vec2,
    },
    BoundaryCrossed {
        tick: Tick,
        unit_id: UnitId,
        position: Vec2,
    },
}

/// Returns `true` when `position` lies strictly outside a circle of
/// `world_radius` centered at the origin.
pub fn is_out_of_bounds(position: Vec2, world_radius: f32) -> bool {
    position.length() > world_radius
}

/// Integrate one tick of ship physics from autopilot flight input into `ship`.
pub fn step_ship(ship: &mut ShipState, input: FlightInput) {
    let turn_axis = input.turn_left as i32 - input.turn_right as i32;
    if turn_axis != 0 {
        ship.angular_velocity_radians_per_second += turn_axis as f32
            * (ANGULAR_THRUST_NEWTON_METERS / MOMENT_OF_INERTIA_KG_M2)
            * FIXED_DT_SECONDS;
    } else {
        ship.angular_velocity_radians_per_second *=
            (1.0 - ANGULAR_DAMPING_PER_SECOND * FIXED_DT_SECONDS).max(0.0);
    }
    ship.angular_velocity_radians_per_second = ship.angular_velocity_radians_per_second.clamp(
        -MAX_ANGULAR_SPEED_RADIANS_PER_SECOND,
        MAX_ANGULAR_SPEED_RADIANS_PER_SECOND,
    );
    ship.heading_radians = wrap_angle(
        ship.heading_radians + ship.angular_velocity_radians_per_second * FIXED_DT_SECONDS,
    );
    if input.thrust {
        let forward = Vec2::new(-ship.heading_radians.sin(), ship.heading_radians.cos());
        ship.velocity += forward * (FORWARD_THRUST_NEWTONS / SHIP_MASS_KG) * FIXED_DT_SECONDS;
    } else {
        ship.velocity *= (1.0 - LINEAR_DAMPING_PER_SECOND * FIXED_DT_SECONDS).max(0.0);
    }
    if ship.velocity.length() > MAX_SPEED_METERS_PER_SECOND {
        ship.velocity = ship.velocity.normalize() * MAX_SPEED_METERS_PER_SECOND;
    }
    if ship.velocity.length_squared() < VELOCITY_EPSILON * VELOCITY_EPSILON {
        ship.velocity = Vec2::ZERO;
    }
    if ship.angular_velocity_radians_per_second.abs() < VELOCITY_EPSILON {
        ship.angular_velocity_radians_per_second = 0.0;
    }
    ship.position += ship.velocity * FIXED_DT_SECONDS;
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    use spacegame2d_protocol::CommandData;
    use std::collections::BTreeMap;

    use crate::autopilot::{Autopilot, AutopilotConfig};
    use crate::command::PlayerId;
    use crate::flight_control::{
        ArrivalController, AvoidanceProfile, AvoidanceProfiles, NeighborRelationship,
    };

    #[test]
    fn starts_without_player_ship() {
        let sim = Simulation::default();
        assert_eq!(sim.tick(), Tick::new(0));
    }
    #[test]
    fn step_advances_tick_without_player_input() {
        let mut sim = Simulation::default();
        assert!(sim.step().unwrap().is_empty());
        assert_eq!(sim.tick(), Tick::new(1));
    }

    #[test]
    fn owned_static_structures_remain_stationary_static_avoidance_observations() {
        let mut simulation = Simulation::default();
        simulation.world.units.truncate(1);
        simulation.world.units[0].state.position = Vec2::new(4.0, 0.0);
        simulation.world.units[0]
            .autopilot
            .set_destination(Vec2::new(4.0, 10.0));

        let observations = avoidance_observations(&simulation.world);
        assert_eq!(
            observations
                .iter()
                .map(|observation| observation.entity_id)
                .collect::<Vec<_>>(),
            vec![
                AvoidanceEntityId::Unit(simulation.world.units[0].id),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(1)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(2)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(3)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(4)),
            ]
        );
        for (structure, (position, owner)) in observations[1..].iter().zip([
            (Vec2::new(-20.0, 0.0), PlayerId(1)),
            (Vec2::new(-10.0, 0.0), PlayerId(1)),
            (Vec2::new(20.0, 0.0), PlayerId(2)),
            (Vec2::new(10.0, 0.0), PlayerId(2)),
        ]) {
            assert_eq!(structure.position, position);
            assert_eq!(structure.velocity, Vec2::ZERO);
            assert_eq!(structure.owner, Some(owner));
        }

        let unit = &mut simulation.world.units[0];
        let neighbors = observations[1..]
            .iter()
            .map(|observation| NeighborObservation {
                entity_id: observation.entity_id,
                position: observation.position,
                velocity: observation.velocity,
                hitbox: observation.hitbox,
                relationship: NeighborRelationship::StaticStructure,
            })
            .collect::<Vec<_>>();
        let with_structure =
            unit.autopilot
                .controls_for_tick_with_hitbox(&unit.state, unit.hitbox(), &neighbors);
        unit.autopilot.set_destination(Vec2::new(4.0, 10.0));
        let without_structure =
            unit.autopilot
                .controls_for_tick_with_hitbox(&unit.state, unit.hitbox(), &[]);
        assert_ne!(with_structure, without_structure);
    }

    #[test]
    fn step_emits_boundary_crossed_and_removes_unit() {
        let mut sim = Simulation::with_world_radius(1.0);
        sim.world.units.truncate(1);
        sim.world.units[0].state.position = Vec2::new(1.1, 0.0);
        let unit_id = sim.world.units[0].id;

        let events = sim.step().unwrap();

        assert_eq!(
            events,
            vec![SimulationEvent::BoundaryCrossed {
                tick: Tick::default(),
                unit_id,
                position: Vec2::new(1.1, 0.0),
            }]
        );
        assert!(sim.world.units.is_empty());
        assert_eq!(sim.tick(), Tick::new(1));
    }

    #[test]
    fn boundary_events_are_sorted_by_unit_id() {
        let mut sim = Simulation::with_world_radius(1.0);
        sim.world.units.truncate(2);
        sim.world.units.swap(0, 1);
        for unit in &mut sim.world.units {
            unit.state.position = Vec2::new(1.1, 0.0);
        }
        let mut expected = sim
            .world
            .units
            .iter()
            .map(|unit| unit.id)
            .collect::<Vec<_>>();
        expected.sort_unstable();

        let events = sim.step().unwrap();
        let actual = events
            .iter()
            .map(|event| match event {
                SimulationEvent::ShotFired { shooter_id, .. } => *shooter_id,
                SimulationEvent::HullDepleted { unit_id, .. }
                | SimulationEvent::BoundaryCrossed { unit_id, .. } => *unit_id,
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }
    #[test]
    fn physics_input_remains_deterministic_for_autopilot() {
        let mut a = ShipState::default();
        let mut b = ShipState::default();
        let input = FlightInput {
            thrust: true,
            turn_left: true,
            turn_right: false,
        };
        step_ship(&mut a, input);
        step_ship(&mut b, input);
        assert_eq!(a, b);
    }

    type UnitSnapshot = (
        UnitId,
        Option<PlayerId>,
        ShipState,
        crate::combat::CombatState,
        Option<Vec2>,
        bool,
    );

    fn unit_snapshot(world: &World) -> Vec<UnitSnapshot> {
        world
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    u.owner,
                    u.state,
                    u.combat,
                    u.autopilot.destination(),
                    u.autopilot.is_active(),
                )
            })
            .collect()
    }

    #[derive(Debug, PartialEq)]
    struct AuthoritativeSnapshot {
        tick: Tick,
        units: Vec<UnitSnapshot>,
        events: Vec<SimulationEvent>,
    }

    fn authoritative_set_destination(
        execute_tick: u64,
        player_slot: u32,
        sequence: u32,
        _unit_id: u32,
        destination: [u32; 2],
    ) -> AuthoritativeCommand {
        AuthoritativeCommand {
            execute_tick: Tick::from(execute_tick),
            player_slot,
            sequence,
            command: CommandData::SetDestination { destination },
        }
    }

    #[test]
    fn unit_id_and_owner_are_stable_across_commands() {
        // CMD-002: stable identity and ownership.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let id_before = sim.world.units[0].id;
        let owner_before = sim.world.units[0].owner;
        let cmd = authoritative_set_destination(0, 1, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        sim.step().unwrap();
        assert_eq!(sim.world.units[0].id, id_before);
        assert_eq!(sim.world.units[0].owner, owner_before);
    }

    #[test]
    fn cross_player_command_is_rejected() {
        // CMD-004: ownership validation.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let pos_before = sim.world.units[0].state.position;
        let cmd = authoritative_set_destination(0, 2, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(!sim.schedule_authoritative(&cmd));
        let events = sim.step().unwrap();
        assert_eq!(sim.world.units[0].state.position, pos_before);
        assert!(events.is_empty());
        assert!(sim.commands.history().is_empty());
    }

    #[test]
    fn invalid_commands_leave_world_unchanged() {
        // CMD-006: invalid commands cause no state mutation or panic.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let snapshot_before = unit_snapshot(&sim.world);

        let bad_slot = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 0,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        };
        assert!(!sim.schedule_authoritative(&bad_slot));

        let unknown_unit =
            authoritative_set_destination(0, 2, 2, 999, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        assert!(!sim.schedule_authoritative(&unknown_unit));

        let nan_destination = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 3,
            command: CommandData::SetDestination {
                destination: [f32::NAN.to_bits(), 0.0f32.to_bits()],
            },
        };
        assert!(!sim.schedule_authoritative(&nan_destination));

        let events = sim.step().unwrap();
        assert_eq!(unit_snapshot(&sim.world), snapshot_before);
        assert!(events.is_empty());
        assert!(sim.commands.history().is_empty());
    }

    #[test]
    fn commands_execute_before_physics_and_tick_advances_by_one() {
        // CMD-007: fixed tick order. The command must be applied before physics
        // so the same tick produces a steering result based on the new destination.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        // Position the unit far from the requested destination and face it so
        // the autopilot produces thrust on the very first tick.
        sim.world.units[0].state.position = Vec2::new(10.0, 0.0);
        // Forward vector is (-sin(h), cos(h)); heading = PI/2 points along -X.
        sim.world.units[0].state.heading_radians = std::f32::consts::FRAC_PI_2;
        let cmd = authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        let tick_before = sim.tick();
        sim.step().unwrap();
        assert_eq!(sim.tick(), tick_before + Tick::new(1));
        // The command applied before physics: the destination is set and the
        // unit has moved toward it in the same tick.
        assert_eq!(sim.world.units[0].autopilot.destination(), Some(Vec2::ZERO));
        assert_ne!(sim.world.units[0].state.position, Vec2::new(10.0, 0.0));
    }

    #[test]
    fn set_destination_does_not_teleport_units() {
        // CMD-010: no teleport.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let pos_before = sim.world.units[0].state.position;
        let cmd =
            authoritative_set_destination(0, 1, 1, 1, [100.0f32.to_bits(), 100.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&cmd));
        sim.step().unwrap();
        assert!(
            sim.world.units[0].state.position.distance(pos_before)
                <= MAX_SPEED_METERS_PER_SECOND * FIXED_DT_SECONDS + 1.0e-6
        );
    }

    #[test]
    fn history_records_only_accepted_commands() {
        // CMD-014: accepted-only history.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        let accepted =
            authoritative_set_destination(0, 1, 1, 1, [1.0f32.to_bits(), 2.0f32.to_bits()]);
        let rejected =
            authoritative_set_destination(0, 2, 2, 1, [3.0f32.to_bits(), 4.0f32.to_bits()]);
        assert!(sim.schedule_authoritative(&accepted));
        assert!(!sim.schedule_authoritative(&rejected));
        sim.step().unwrap();
        assert_eq!(sim.commands.history().len(), 1);
        assert_eq!(sim.commands.history()[0].execute_tick(), Tick::new(0));
    }

    #[test]
    fn cross_peer_replay_is_deterministic_tick_by_tick() {
        // DEP-005: compare the authoritative per-tick snapshot, rather than
        // rather than only comparing a legacy aggregate at the end.
        let commands = vec![
            authoritative_set_destination(1, 1, 1, 1, [6.0f32.to_bits(), 5.0f32.to_bits()]),
            authoritative_set_destination(3, 1, 2, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]),
        ];
        let ticks = 10;

        let mut a = Simulation::default();
        a.world.units[0].owner = Some(PlayerId(1));
        let mut a_snapshots = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == a.tick() {
                    a.schedule_authoritative(cmd);
                }
            }
            let events = a.step().unwrap();
            a_snapshots.push(AuthoritativeSnapshot {
                tick: a.tick(),
                units: unit_snapshot(&a.world),
                events,
            });
        }

        let mut b = Simulation::default();
        b.world.units[0].owner = Some(PlayerId(1));
        let mut b_snapshots = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == b.tick() {
                    b.schedule_authoritative(cmd);
                }
            }
            let events = b.step().unwrap();
            b_snapshots.push(AuthoritativeSnapshot {
                tick: b.tick(),
                units: unit_snapshot(&b.world),
                events,
            });
        }

        assert_eq!(a_snapshots, b_snapshots);
    }

    #[test]
    fn cross_peer_boundary_event_vectors_are_non_empty_at_same_ticks() {
        let command =
            authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 100.0f32.to_bits()]);
        let mut peers = [
            Simulation::with_world_radius(1.0),
            Simulation::with_world_radius(1.0),
        ];
        for peer in &mut peers {
            peer.world.units.truncate(1);
            peer.world.units[0].owner = Some(PlayerId(1));
            peer.world.units[0].state.position = Vec2::new(0.0, 0.9);
            peer.world.units[0].state.heading_radians = 0.0;
            assert!(peer.schedule_authoritative(&command));
        }

        let mut event_vectors: [Vec<Vec<SimulationEvent>>; 2] = [Vec::new(), Vec::new()];
        for _ in 0..20 {
            for (events, peer) in event_vectors.iter_mut().zip(&mut peers) {
                events.push(peer.step().unwrap());
            }
        }

        assert_eq!(event_vectors[0], event_vectors[1]);
        let non_empty: Vec<_> = event_vectors[0]
            .iter()
            .enumerate()
            .filter(|(_, events)| !events.is_empty())
            .collect();
        assert_eq!(non_empty.len(), 1);
        assert_eq!(non_empty[0].0, 9);
        assert!(matches!(
            non_empty[0].1.as_slice(),
            [SimulationEvent::BoundaryCrossed { tick, .. }] if *tick == Tick::from(9)
        ));
    }

    #[test]
    fn authoritative_command_drives_unit_across_boundary_on_two_peers() {
        let command =
            authoritative_set_destination(0, 1, 1, 1, [0.0f32.to_bits(), 100.0f32.to_bits()]);
        let mut peers = [
            Simulation::with_world_radius(1.0),
            Simulation::with_world_radius(1.0),
        ];
        for peer in &mut peers {
            peer.world.units.truncate(1);
            peer.world.units[0].owner = Some(PlayerId(1));
            peer.world.units[0].state.position = Vec2::new(0.0, 0.9);
            peer.world.units[0].state.heading_radians = 0.0;
            assert!(peer.schedule_authoritative(&command));
        }

        let mut control = Simulation::with_world_radius(1.0);
        control.world.units.truncate(1);
        control.world.units[0].owner = Some(PlayerId(1));
        control.world.units[0].state.position = Vec2::new(0.0, 0.9);
        control.world.units[0].state.heading_radians = 0.0;

        let mut peer_events: [Vec<Vec<SimulationEvent>>; 2] = [Vec::new(), Vec::new()];
        for _ in 0..30 {
            for (events, peer) in peer_events.iter_mut().zip(&mut peers) {
                events.push(peer.step().unwrap());
            }
            assert!(control.step().unwrap().is_empty());
        }

        assert!(peer_events[0].iter().any(|events| !events.is_empty()));
        assert_eq!(peer_events[0], peer_events[1]);
        assert!(peers.iter().all(|peer| peer.world.units.is_empty()));
    }

    #[test]
    fn recorded_history_replays_to_identical_state_and_events() {
        // CMD-008: replay determinism.
        let commands = vec![
            authoritative_set_destination(1, 1, 1, 1, [6.0f32.to_bits(), 5.0f32.to_bits()]),
            authoritative_set_destination(3, 1, 2, 1, [0.0f32.to_bits(), 0.0f32.to_bits()]),
        ];
        let ticks = 10;

        let mut original = Simulation::default();
        original.world.units[0].owner = Some(PlayerId(1));
        let mut original_events = Vec::new();
        for _ in 0..ticks {
            for cmd in &commands {
                if cmd.execute_tick == original.tick() {
                    original.schedule_authoritative(cmd);
                }
            }
            original_events.extend(original.step().unwrap());
        }
        let history = original.commands.history().to_vec();

        let mut replay = Simulation::default();
        replay.world.units[0].owner = Some(PlayerId(1));
        let replay_events = CommandScheduler::replay(&history, &mut replay, Tick::from(ticks - 1));

        assert_eq!(replay.tick(), original.tick());
        assert_eq!(unit_snapshot(&replay.world), unit_snapshot(&original.world));
        assert_eq!(replay_events.unwrap(), original_events);
    }

    #[test]
    fn avoidance_improves_pairwise_separation_without_bypassing_physics() {
        fn configured_simulation(avoidance_strength: f32) -> Simulation {
            let mut simulation = Simulation::with_world_radius(100.0);
            simulation.world.units.truncate(3);
            let states = [
                ShipState {
                    position: Vec2::new(-1.5, 0.0),
                    ..Default::default()
                },
                ShipState {
                    position: Vec2::new(1.5, 0.0),
                    ..Default::default()
                },
                ShipState {
                    position: Vec2::new(0.0, -1.5),
                    ..Default::default()
                },
            ];
            let defaults = AvoidanceProfiles::default();
            let profiles = defaults
                .profiles()
                .iter()
                .map(|profile| {
                    let strength = match profile.relationship() {
                        NeighborRelationship::Friendly | NeighborRelationship::Opposing => {
                            avoidance_strength
                        }
                        NeighborRelationship::StaticStructure => profile.strength(),
                    };
                    AvoidanceProfile::new(
                        profile.relationship(),
                        profile.group(),
                        profile.comfort_clearance_meters(),
                        strength,
                        profile.speed_squared_scale(),
                    )
                    .unwrap()
                })
                .collect();
            let avoidance = AvoidanceProfiles::new(
                defaults.prediction_horizon_seconds(),
                defaults.groups().to_vec(),
                profiles,
            )
            .unwrap();
            for (unit, state) in simulation.world.units.iter_mut().zip(states) {
                let mut controller_config = ArrivalController::default().config;
                controller_config.avoidance = avoidance.clone();
                unit.state = state;
                unit.autopilot = Autopilot::new(
                    Box::new(ArrivalController {
                        config: controller_config,
                    }),
                    AutopilotConfig::default(),
                );
                unit.autopilot.set_destination(Vec2::new(0.0, 6.0));
            }
            simulation
        }

        fn minimum_separation(simulation: &Simulation) -> f32 {
            simulation
                .world
                .units
                .iter()
                .enumerate()
                .flat_map(|(index, unit)| {
                    simulation.world.units[index + 1..]
                        .iter()
                        .map(move |other| unit.state.position.distance(other.state.position))
                })
                .fold(f32::INFINITY, f32::min)
        }

        fn run(mut simulation: Simulation) -> (f32, Vec<ShipState>) {
            let mut minimum = f32::INFINITY;
            for _ in 0..360 {
                simulation.step().unwrap();
                minimum = minimum.min(minimum_separation(&simulation));
                for unit in &simulation.world.units {
                    assert!(unit.state.position.is_finite());
                    assert!(unit.state.velocity.is_finite());
                    assert!(unit.state.velocity.length() <= MAX_SPEED_METERS_PER_SECOND);
                    assert!(unit.state.angular_velocity_radians_per_second.is_finite());
                    assert!(
                        unit.state.angular_velocity_radians_per_second.abs()
                            <= MAX_ANGULAR_SPEED_RADIANS_PER_SECOND
                    );
                }
            }
            (
                minimum,
                simulation
                    .world
                    .units
                    .iter()
                    .map(|unit| unit.state)
                    .collect(),
            )
        }

        let (baseline_separation, baseline_states) = run(configured_simulation(0.0));
        let (avoidance_separation, avoidance_states) = run(configured_simulation(8.0));
        assert!(
            avoidance_separation > baseline_separation,
            "avoidance separation {avoidance_separation} should exceed baseline {baseline_separation}"
        );
        assert!(
            avoidance_states
                .iter()
                .all(|state| state.position.distance(Vec2::new(0.0, 6.0)) < 6.0),
            "avoidance should preserve destination progress"
        );

        let (_, repeated_states) = run(configured_simulation(8.0));
        assert_eq!(avoidance_states, repeated_states);
        assert_ne!(baseline_states, avoidance_states);
    }

    #[test]
    fn set_tick_advances_clock() {
        let mut sim = Simulation::default();
        sim.set_tick(Tick::from(42));
        assert_eq!(sim.tick(), Tick::new(42));
    }

    #[test]
    fn world_accessors_and_custom_radius() {
        let sim = Simulation::with_world_radius(2.0);
        assert_eq!(sim.world_radius(), 2.0);
        assert_eq!(sim.world().units.len(), crate::command::MAX_UNITS);
        assert_eq!(sim.config().fleet_size(), 30);
        let custom = Simulation::new(crate::SimulationConfig::new(3).unwrap());
        assert_eq!(custom.world().units.len(), 6);
    }

    #[test]
    fn step_ship_damps_velocity_without_thrust() {
        let mut ship = ShipState {
            velocity: Vec2::new(4.0, 0.0),
            ..Default::default()
        };
        let speed = ship.velocity.length();
        step_ship(&mut ship, FlightInput::default());
        assert!(ship.velocity.length() < speed);
        assert!(ship.velocity.length() > 0.0);
    }

    #[test]
    fn reset_cutover_discards_future_commands_and_preserves_tick() {
        let mut sim = Simulation::new(crate::SimulationConfig::new(3).unwrap());
        sim.world.connect_player(PlayerId(1));
        sim.set_tick(Tick::new(4));
        let reset = AuthoritativeCommand {
            execute_tick: Tick::new(4),
            player_slot: 1,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        let destination = AuthoritativeCommand {
            execute_tick: Tick::new(5),
            player_slot: 1,
            sequence: 2,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        };
        let mut scheduled = BTreeMap::from([
            (Tick::new(4), vec![reset]),
            (Tick::new(5), vec![destination]),
        ]);
        let applied = sim.apply_due_commands(&mut scheduled);
        assert!(applied.reset_applied);
        assert!(scheduled.is_empty());
        assert_eq!(sim.tick(), Tick::new(4));
        sim.step().unwrap();
        assert_eq!(sim.tick(), Tick::new(5));
        assert!(
            sim.world
                .units
                .iter()
                .all(|unit| unit.autopilot.destination().is_none())
        );
    }

    #[test]
    fn reset_simulation_records_in_history_and_replays() {
        // Covers ResetSimulation::record and command_from_record for reset.
        let mut sim = Simulation::default();
        sim.world.units[0].owner = Some(PlayerId(1));
        sim.world.connect_player(PlayerId(1));
        let reset = AuthoritativeCommand {
            execute_tick: Tick::from(2),
            player_slot: 1,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        assert!(sim.schedule_authoritative(&reset));
        sim.step().unwrap();
        sim.step().unwrap();
        sim.step().unwrap();
        assert_eq!(sim.commands.history().len(), 1);
        assert_eq!(sim.commands.history()[0].execute_tick(), Tick::new(2));
        assert!(matches!(
            sim.commands.history()[0],
            crate::command::RecordedCommand::ResetSimulation { .. }
        ));

        let history = sim.commands.history().to_vec();
        let mut replay = Simulation::default();
        replay.world.units[0].owner = Some(PlayerId(1));
        CommandScheduler::replay(&history, &mut replay, sim.tick() - Tick::new(1)).unwrap();
        assert_eq!(replay.tick(), sim.tick());
        assert_eq!(replay.world.units[0].owner, Some(PlayerId(1)));
    }
    fn combat_simulation(unit_count: u32) -> Simulation {
        let mut simulation = Simulation::new(crate::SimulationConfig::new(unit_count).unwrap());
        for unit in &mut simulation.world.units {
            unit.state.position = Vec2::new(100.0, 100.0);
        }
        simulation
    }

    #[test]
    fn aligned_target_is_acquired_and_hit_immediately() {
        let mut simulation = combat_simulation(1);
        let shooter_id = simulation.world.units[0].id;
        let target_id = simulation.world.units[1].id;
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::ZERO;
        simulation.world.units[1].state.position = Vec2::new(0.0, 5.0);

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(target_id)
        );
        assert_eq!(
            simulation.world.units[1].combat.hull.current,
            simulation.world.units[1].combat.hull.maximum - WEAPON_DAMAGE
        );
        assert!(
            matches!(events.as_slice(), [SimulationEvent::ShotFired { shooter_id: id, hit_unit_id: Some(hit), .. }] if *id == shooter_id && *hit == target_id)
        );
    }

    #[test]
    fn intervening_hostile_is_hit_without_retargeting() {
        let mut simulation = combat_simulation(2);
        simulation.world.units.truncate(3);
        let shooter_id = simulation.world.units[0].id;
        let target_id = simulation.world.units[1].id;
        let interceptor_id = simulation.world.units[2].id;
        simulation.world.units[0].owner = Some(PlayerId(1));
        simulation.world.units[1].owner = Some(PlayerId(2));
        simulation.world.units[2].owner = Some(PlayerId(2));
        simulation.world.units[0].state.position = Vec2::ZERO;
        simulation.world.units[1].state.position = Vec2::new(0.0, 8.0);
        simulation.world.units[2].state.position = Vec2::new(0.0, 4.0);
        simulation.world.units[0].combat.turret.target = Some(target_id);
        simulation.world.units[2].combat.hull.current = WEAPON_DAMAGE;

        let events = simulation.step().unwrap();

        assert_eq!(
            simulation.world.units[0].combat.turret.target,
            Some(target_id)
        );
        assert!(simulation.world.unit(interceptor_id).is_none());
        assert!(
            matches!(events.as_slice(), [SimulationEvent::ShotFired { shooter_id: id, hit_unit_id: Some(hit), .. }, SimulationEvent::HullDepleted { unit_id, .. }] if *id == shooter_id && *hit == interceptor_id && *unit_id == interceptor_id)
        );
    }
}
