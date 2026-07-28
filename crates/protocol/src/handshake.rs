use std::io;

use crate::SIMULATION_VERSION;
use crate::error::invalid;
use crate::identity::{DisplayName, DisplayNameError};
use crate::tick::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    StateChecksums,
    WorldSnapshots,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientHello {
    pub simulation_version: u32,
    pub capabilities: Vec<Capability>,
    pub display_name: String,
}

impl ClientHello {
    pub fn display_name(&self) -> Result<DisplayName, DisplayNameError> {
        DisplayName::try_from(self.display_name.as_str())
    }

    pub fn is_compatible(&self) -> bool {
        self.simulation_version == SIMULATION_VERSION
            && self.capabilities.contains(&Capability::StateChecksums)
            && self.capabilities.contains(&Capability::WorldSnapshots)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerHello {
    pub simulation_version: u32,
    pub simulation_hz: u32,
    pub player_slot: u32,
    pub server_tick: Tick,
    pub fleet_size: u32,
    pub world_radius_bits: u32,
    pub capabilities: Vec<Capability>,
}

impl ServerHello {
    pub fn validate(&self, simulation_hz: u32) -> io::Result<()> {
        if self.simulation_version != SIMULATION_VERSION {
            return Err(invalid("simulation version mismatch"));
        }
        if self.simulation_hz != simulation_hz {
            return Err(invalid("simulation frequency mismatch"));
        }
        if self.player_slot == 0 || u8::try_from(self.player_slot).is_err() {
            return Err(invalid("server assigned invalid player slot"));
        }
        if self.fleet_size == 0 {
            return Err(invalid("server assigned invalid fleet size"));
        }
        let world_radius = f32::from_bits(self.world_radius_bits);
        if !world_radius.is_finite() || world_radius <= 0.0 {
            return Err(invalid("server assigned invalid world radius"));
        }
        if !self.capabilities.contains(&Capability::StateChecksums) {
            return Err(invalid("server does not support state checksums"));
        }
        if !self.capabilities.contains(&Capability::WorldSnapshots) {
            return Err(invalid("server does not support world snapshots"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakeRejectionReason {
    ServerFull,
    IncompatibleVersion,
    MissingRequiredCapability,
    InvalidHandshake,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakeRejected {
    pub reason: HandshakeRejectionReason,
}
