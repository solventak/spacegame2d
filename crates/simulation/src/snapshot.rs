//! Canonical deterministic simulation snapshots and state hashes.

use crate::{autopilot::AutopilotConfig, command::Unit, simulation::Simulation};
use spacegame2d_protocol::Tick;

pub const SNAPSHOT_FORMAT_VERSION: u16 = 3;
pub const STATE_HASH_BYTES: usize = 32;
pub type StateHash = [u8; STATE_HASH_BYTES];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationSnapshot {
    pub format_version: u16,
    pub simulation_version: u32,
    pub tick: Tick,
    pub world_radius_bits: u32,
    pub next_unit_id: u32,
    pub unit_id_exhausted: bool,
    pub units: Vec<UnitSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitSnapshot {
    pub id: u32,
    pub owner: Option<u8>,
    pub position_bits: [u32; 2],
    pub velocity_bits: [u32; 2],
    pub heading_bits: u32,
    pub angular_velocity_bits: u32,
    pub controller_kind: String,
    pub destination_bits: Option<[u32; 2]>,
    pub active: bool,
    pub arrival_radius_bits: u32,
    pub stopped_speed_bits: u32,
    pub hull_current: u32,
    pub hull_maximum: u32,
    pub turret_local_heading_bits: u32,
    pub turret_target: Option<u32>,
    pub turret_cooldown_ticks_remaining: u32,
}

impl Simulation {
    pub fn snapshot(&self) -> SimulationSnapshot {
        let (next_unit_id, unit_id_exhausted) = self.world.allocator_state();
        let mut units = self
            .world
            .units
            .iter()
            .map(UnitSnapshot::from)
            .collect::<Vec<_>>();
        units.sort_unstable_by_key(|unit| unit.id);
        SimulationSnapshot {
            format_version: SNAPSHOT_FORMAT_VERSION,
            simulation_version: spacegame2d_protocol::SIMULATION_VERSION,
            tick: self.tick(),
            world_radius_bits: self.world_radius().to_bits(),
            next_unit_id,
            unit_id_exhausted,
            units,
        }
    }

    pub fn state_hash(&self) -> StateHash {
        self.snapshot().state_hash()
    }
}

impl SimulationSnapshot {
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.tick.0);
        put_u32(&mut bytes, self.units.len() as u32);
        for unit in &self.units {
            unit.encode(&mut bytes);
        }
        bytes
    }

    pub fn state_hash(&self) -> StateHash {
        *blake3::hash(&self.encode_canonical()).as_bytes()
    }
}

impl From<&Unit> for UnitSnapshot {
    fn from(unit: &Unit) -> Self {
        let config: AutopilotConfig = unit.autopilot.config();
        Self {
            id: unit.id.0,
            owner: unit.owner.map(|owner| owner.0),
            position_bits: [
                unit.state.position.x.to_bits(),
                unit.state.position.y.to_bits(),
            ],
            velocity_bits: [
                unit.state.velocity.x.to_bits(),
                unit.state.velocity.y.to_bits(),
            ],
            heading_bits: unit.state.heading_radians.to_bits(),
            angular_velocity_bits: unit.state.angular_velocity_radians_per_second.to_bits(),
            controller_kind: unit.autopilot.controller_name().to_owned(),
            destination_bits: unit
                .autopilot
                .destination()
                .map(|destination| [destination.x.to_bits(), destination.y.to_bits()]),
            active: unit.autopilot.is_active(),
            arrival_radius_bits: config.arrival_radius_meters.to_bits(),
            stopped_speed_bits: config.stopped_speed_meters_per_second.to_bits(),
            hull_current: unit.combat.hull.current,
            hull_maximum: unit.combat.hull.maximum,
            turret_local_heading_bits: unit.combat.turret.local_heading_radians.to_bits(),
            turret_target: unit.combat.turret.target.map(|target| target.0),
            turret_cooldown_ticks_remaining: unit.combat.turret.cooldown_ticks_remaining,
        }
    }
}

impl UnitSnapshot {
    fn encode(&self, bytes: &mut Vec<u8>) {
        put_u32(bytes, self.id);
        match self.owner {
            Some(owner) => {
                bytes.push(1);
                bytes.push(owner);
            }
            None => {
                bytes.push(0);
                bytes.push(0);
            }
        }
        for value in self.position_bits.iter().chain(self.velocity_bits.iter()) {
            put_u32(bytes, *value);
        }
        put_u32(bytes, self.heading_bits);
        put_u32(bytes, self.angular_velocity_bits);
        bytes.push(self.active as u8);
        put_u32(bytes, self.hull_current);
        put_u32(bytes, self.hull_maximum);
        put_u32(bytes, self.turret_local_heading_bits);
        match self.turret_target {
            Some(target) => {
                bytes.push(1);
                put_u32(bytes, target);
            }
            None => bytes.push(0),
        }
        put_u32(bytes, self.turret_cooldown_ticks_remaining);
        match self.destination_bits {
            Some(destination) => {
                bytes.push(1);
                put_u32(bytes, destination[0]);
                put_u32(bytes, destination[1]);
            }
            None => bytes.push(0),
        }
    }
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_simulations_have_equal_hashes() {
        assert_eq!(
            Simulation::default().state_hash(),
            Simulation::default().state_hash()
        );
    }

    #[test]
    fn unit_order_is_canonical() {
        let mut a = Simulation::default();
        let mut b = Simulation::default();
        b.world.units.swap(0, 1);
        assert_eq!(a.state_hash(), b.state_hash());
        a.world.units[0].state.position.x += 1.0;
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn advancing_tick_changes_hash() {
        let mut simulation = Simulation::default();
        let before = simulation.state_hash();
        simulation.step().unwrap();
        assert_ne!(before, simulation.state_hash());
    }

    #[test]
    fn combat_state_changes_hash() {
        let baseline = Simulation::default().state_hash();
        let mut hull = Simulation::default();
        hull.world.units[0].combat.hull.current -= 1;
        assert_ne!(hull.state_hash(), baseline);
        let mut heading = Simulation::default();
        heading.world.units[0].combat.turret.local_heading_radians = 1.0;
        assert_ne!(heading.state_hash(), baseline);
        let mut target = Simulation::default();
        let id = target.world.units[1].id;
        target.world.units[0].combat.turret.target = Some(id);
        assert_ne!(target.state_hash(), baseline);
        let mut cooldown = Simulation::default();
        cooldown.world.units[0]
            .combat
            .turret
            .cooldown_ticks_remaining = 1;
        assert_ne!(cooldown.state_hash(), baseline);
    }

    #[test]
    fn equivalent_authoritative_inputs_have_equal_hashes() {
        use spacegame2d_protocol::{AuthoritativeCommand, CommandData};
        let mut left = Simulation::default();
        let mut right = Simulation::default();
        left.world.assign_mirror_owners();
        right.world.assign_mirror_owners();
        let command = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 1,
            command: CommandData::SetDestination {
                destination: [1.0f32.to_bits(), 2.0f32.to_bits()],
            },
        };
        assert!(left.schedule_authoritative_trusted(&command));
        assert!(right.schedule_authoritative_trusted(&command));
        left.step().unwrap();
        right.step().unwrap();
        assert_eq!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn reset_and_repeated_commands_have_equal_hashes() {
        use spacegame2d_protocol::{AuthoritativeCommand, CommandData};
        let mut left = Simulation::default();
        let mut right = Simulation::default();
        left.world.assign_mirror_owners();
        right.world.assign_mirror_owners();
        let reset = AuthoritativeCommand {
            execute_tick: Tick::default(),
            player_slot: 1,
            sequence: 1,
            command: CommandData::ResetSimulation,
        };
        let destination = AuthoritativeCommand {
            execute_tick: Tick::from(1),
            player_slot: 2,
            sequence: 2,
            command: CommandData::SetDestination {
                destination: [3.0f32.to_bits(), (-2.0f32).to_bits()],
            },
        };
        for simulation in [&mut left, &mut right] {
            assert!(simulation.schedule_authoritative_trusted(&reset));
            simulation.step().unwrap();
            assert!(simulation.schedule_authoritative_trusted(&destination));
            simulation.step().unwrap();
        }
        assert_eq!(left.state_hash(), right.state_hash());
    }
}
