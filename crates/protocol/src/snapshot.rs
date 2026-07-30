use crate::command::AuthoritativeCommand;
use crate::tick::Tick;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldUnit {
    pub id: u32,
    pub owner: Option<u32>,
    pub position_bits: [u32; 2],
    pub velocity_bits: [u32; 2],
    pub heading_bits: u32,
    pub angular_velocity_bits: u32,
    pub active: bool,
    pub destination_bits: Option<[u32; 2]>,
    pub hull_current: u32,
    pub hull_maximum: u32,
    pub turret_local_heading_bits: u32,
    pub turret_target: Option<(u32, u32)>,
    pub turret_cooldown_ticks_remaining: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitialWorldState {
    pub snapshot_format_version: u32,
    pub simulation_version: u32,
    pub tick: Tick,
    pub world_radius_bits: u32,
    pub next_unit_id: u32,
    pub unit_id_exhausted: bool,
    pub units: Vec<WorldUnit>,
    pub state_hash: Vec<u8>,
    pub pending_commands: Vec<AuthoritativeCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateChecksum {
    pub tick: Tick,
    pub hash: Vec<u8>,
}
