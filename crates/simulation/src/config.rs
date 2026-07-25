use thiserror::Error;

pub const DEFAULT_FLEET_SIZE: u32 = 30;
pub const MAX_PLAYERS: usize = 2;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SimulationConfigError {
    #[error("fleet size must be greater than zero")]
    ZeroFleetSize,
    #[error("fleet size is too large for the simulation")]
    FleetSizeTooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulationConfig {
    fleet_size: u32,
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
        Ok(Self { fleet_size })
    }

    pub const fn fleet_size(self) -> u32 {
        self.fleet_size
    }

    pub fn total_units(self) -> usize {
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
    fn defaults_to_two_fleets_of_thirty() {
        let config = SimulationConfig::default();
        assert_eq!(config.fleet_size(), 30);
        assert_eq!(config.total_units(), 60);
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
}
