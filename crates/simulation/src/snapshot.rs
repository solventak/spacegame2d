//! Canonical deterministic simulation snapshots and state hashes.

use crate::{
    autopilot::AutopilotConfig,
    command::Unit,
    hitbox::HitboxShape,
    simulation::Simulation,
    structure::{HomeObjectivePair, StaticStructure},
};
use spacegame2d_protocol::Tick;

pub const SNAPSHOT_FORMAT_VERSION: u16 = 5;
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
    pub structures: Vec<StaticStructureSnapshot>,
    pub home_objective_pairs: Vec<HomeObjectivePairSnapshot>,
    pub units: Vec<UnitSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticStructureSnapshot {
    pub id: u32,
    pub owner: u8,
    pub kind_tag: u8,
    pub position_bits: [u32; 2],
    pub visual_radius_bits: u32,
    pub hitbox_shape_tag: u8,
    pub hitbox_radius_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeObjectivePairSnapshot {
    pub owner: u8,
    pub core_id: u32,
    pub relay_id: u32,
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
        let mut structures = self
            .world
            .structures()
            .iter()
            .map(StaticStructureSnapshot::from)
            .collect::<Vec<_>>();
        structures.sort_unstable_by_key(|structure| structure.id);
        let mut home_objective_pairs = self
            .world
            .home_objective_pairs()
            .iter()
            .map(HomeObjectivePairSnapshot::from)
            .collect::<Vec<_>>();
        home_objective_pairs.sort_unstable_by_key(|pair| (pair.owner, pair.core_id, pair.relay_id));
        SimulationSnapshot {
            format_version: SNAPSHOT_FORMAT_VERSION,
            simulation_version: spacegame2d_protocol::SIMULATION_VERSION,
            tick: self.tick(),
            world_radius_bits: self.world_radius().to_bits(),
            next_unit_id,
            unit_id_exhausted,
            structures,
            home_objective_pairs,
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
        put_u32(&mut bytes, self.world_radius_bits);
        put_u64(&mut bytes, self.tick.0);
        put_u32(&mut bytes, self.structures.len() as u32);
        for structure in &self.structures {
            structure.encode(&mut bytes);
        }
        put_u32(&mut bytes, self.home_objective_pairs.len() as u32);
        for pair in &self.home_objective_pairs {
            pair.encode(&mut bytes);
        }
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

impl From<&StaticStructure> for StaticStructureSnapshot {
    fn from(structure: &StaticStructure) -> Self {
        let HitboxShape::Circle(circle) = structure.hitbox().shape();
        Self {
            id: structure.id().0,
            owner: structure.owner().0,
            kind_tag: structure.kind().canonical_tag(),
            position_bits: [
                structure.position().x.to_bits(),
                structure.position().y.to_bits(),
            ],
            visual_radius_bits: structure.visual_radius_meters().to_bits(),
            hitbox_shape_tag: 1,
            hitbox_radius_bits: circle.radius_meters().to_bits(),
        }
    }
}

impl StaticStructureSnapshot {
    fn encode(&self, bytes: &mut Vec<u8>) {
        put_u32(bytes, self.id);
        bytes.push(self.owner);
        bytes.push(self.kind_tag);
        for value in self.position_bits {
            put_u32(bytes, value);
        }
        put_u32(bytes, self.visual_radius_bits);
        bytes.push(self.hitbox_shape_tag);
        put_u32(bytes, self.hitbox_radius_bits);
    }
}

impl From<&HomeObjectivePair> for HomeObjectivePairSnapshot {
    fn from(pair: &HomeObjectivePair) -> Self {
        Self {
            owner: pair.owner().0,
            core_id: pair.core_id().0,
            relay_id: pair.relay_id().0,
        }
    }
}

impl HomeObjectivePairSnapshot {
    fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.owner);
        put_u32(bytes, self.core_id);
        put_u32(bytes, self.relay_id);
    }
}

impl From<&Unit> for UnitSnapshot {
    fn from(unit: &Unit) -> Self {
        // Every movable unit currently uses the shared code-defined ship
        // hitbox. Once hitboxes become runtime-configurable, their geometry
        // must be included in this canonical snapshot and state hash.
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
    fn structures_are_canonical_snapshot_state() {
        let snapshot = Simulation::default().snapshot();
        assert_eq!(snapshot.format_version, SNAPSHOT_FORMAT_VERSION);
        assert_eq!(snapshot.structures.len(), 4);
        for (structure, (id, owner, kind, position, visual_radius, hitbox_radius)) in
            snapshot.structures.iter().zip([
                (1, 1, 1, [-20.0_f32, 0.0], 3.5_f32, 3.85_f32),
                (2, 1, 2, [-10.0_f32, 0.0], 2.5_f32, 2.75_f32),
                (3, 2, 1, [20.0_f32, 0.0], 3.5_f32, 3.85_f32),
                (4, 2, 2, [10.0_f32, 0.0], 2.5_f32, 2.75_f32),
            ])
        {
            assert_eq!(structure.id, id);
            assert_eq!(structure.owner, owner);
            assert_eq!(structure.kind_tag, kind);
            assert_eq!(
                structure.position_bits,
                [position[0].to_bits(), position[1].to_bits()]
            );
            assert_eq!(structure.visual_radius_bits, visual_radius.to_bits());
            assert_eq!(structure.hitbox_shape_tag, 1);
            assert_eq!(structure.hitbox_radius_bits, hitbox_radius.to_bits());
        }
        assert_eq!(
            snapshot.home_objective_pairs,
            vec![
                HomeObjectivePairSnapshot {
                    owner: 1,
                    core_id: 1,
                    relay_id: 2,
                },
                HomeObjectivePairSnapshot {
                    owner: 2,
                    core_id: 3,
                    relay_id: 4,
                },
            ]
        );
    }

    #[test]
    fn every_structure_definition_field_contributes_to_the_hash() {
        let baseline = Simulation::default().snapshot();
        for mutate in [
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].id += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].owner += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].kind_tag += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].position_bits[0] += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].visual_radius_bits += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].hitbox_shape_tag += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.structures[0].hitbox_radius_bits += 1,
        ] {
            let mut changed = baseline.clone();
            mutate(&mut changed);
            assert_ne!(changed.state_hash(), baseline.state_hash());
        }
    }

    #[test]
    fn every_home_pair_field_contributes_to_the_hash() {
        let baseline = Simulation::default().snapshot();
        for mutate in [
            |snapshot: &mut SimulationSnapshot| snapshot.home_objective_pairs[0].owner += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.home_objective_pairs[0].core_id += 1,
            |snapshot: &mut SimulationSnapshot| snapshot.home_objective_pairs[0].relay_id += 1,
        ] {
            let mut changed = baseline.clone();
            mutate(&mut changed);
            assert_ne!(changed.state_hash(), baseline.state_hash());
        }
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
    fn world_radius_changes_canonical_bytes_and_hash() {
        let default = Simulation::default();
        let custom = Simulation::with_world_radius(32.0);
        assert_ne!(
            default.snapshot().world_radius_bits,
            custom.snapshot().world_radius_bits
        );
        assert_ne!(
            default.snapshot().encode_canonical(),
            custom.snapshot().encode_canonical()
        );
        assert_ne!(default.state_hash(), custom.state_hash());
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
