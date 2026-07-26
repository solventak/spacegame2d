use thiserror::Error;

use crate::flight_control::AvoidanceProfiles;

pub const DEFAULT_FLEET_SIZE: u32 = 100;
pub const DEFAULT_WORLD_RADIUS_METERS: f32 = 64.0;
pub const MAX_PLAYERS: usize = 2;

pub type AvoidanceConfig = AvoidanceProfiles;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SimulationConfigError {
    #[error("fleet size must be greater than zero")]
    ZeroFleetSize,
    #[error("fleet size is too large for the simulation")]
    FleetSizeTooLarge,
    #[error("world radius must be finite and greater than zero")]
    InvalidWorldRadius,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationConfig {
    fleet_size: u32,
    world_radius_meters: f32,
    avoidance: AvoidanceConfig,
}

impl SimulationConfig {
    pub fn new(fleet_size: u32) -> Result<Self, SimulationConfigError> {
        if fleet_size == 0 {
            return Err(SimulationConfigError::ZeroFleetSize);
        }
        let total = usize::try_from(fleet_size)
            .ok()
            .and_then(|size| size.checked_mul(MAX_PLAYERS));
        if total.is_none() || total.unwrap() > u32::MAX as usize {
            return Err(SimulationConfigError::FleetSizeTooLarge);
        }
        Ok(Self {
            fleet_size,
            world_radius_meters: DEFAULT_WORLD_RADIUS_METERS,
            avoidance: AvoidanceConfig::default(),
        })
    }

    pub const fn fleet_size(&self) -> u32 {
        self.fleet_size
    }

    pub fn avoidance(&self) -> AvoidanceConfig {
        self.avoidance.clone()
    }

    pub const fn world_radius_meters(&self) -> f32 {
        self.world_radius_meters
    }

    pub fn with_world_radius_meters(
        self,
        world_radius_meters: f32,
    ) -> Result<Self, SimulationConfigError> {
        if !world_radius_meters.is_finite() || world_radius_meters <= 0.0 {
            return Err(SimulationConfigError::InvalidWorldRadius);
        }
        Ok(Self {
            world_radius_meters,
            ..self
        })
    }

    pub fn with_avoidance(self, avoidance: AvoidanceConfig) -> Self {
        Self { avoidance, ..self }
    }

    pub fn total_units(&self) -> usize {
        self.fleet_size as usize * MAX_PLAYERS
    }
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self::new(DEFAULT_FLEET_SIZE).expect("default fleet size is valid")
    }
}

impl TryFrom<u32> for SimulationConfig {
    type Error = SimulationConfigError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_two_configured_fleets() {
        let config = SimulationConfig::default();
        assert_eq!(config.fleet_size(), DEFAULT_FLEET_SIZE);
        assert_eq!(config.total_units(), (DEFAULT_FLEET_SIZE * 2) as usize);
        assert_eq!(config.world_radius_meters(), 64.0);
    }

    #[test]
    fn accepts_custom_fleet_size() {
        let config = SimulationConfig::new(3).unwrap();
        assert_eq!(config.fleet_size(), 3);
        assert_eq!(config.total_units(), 6);
    }

    #[test]
    fn rejects_zero_fleet_size() {
        assert_eq!(
            SimulationConfig::new(0),
            Err(SimulationConfigError::ZeroFleetSize)
        );
    }

    #[test]
    fn validates_custom_world_radius() {
        let config = SimulationConfig::default()
            .with_world_radius_meters(12.5)
            .unwrap();
        assert_eq!(config.world_radius_meters(), 12.5);
        for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                SimulationConfig::default().with_world_radius_meters(radius),
                Err(SimulationConfigError::InvalidWorldRadius)
            );
        }
    }
}
