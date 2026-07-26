use thiserror::Error;

pub const DEFAULT_FLEET_SIZE: u32 = 30;
pub const MAX_PLAYERS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidanceConfig {
    /// Desired hull-to-hull clearance for friendly units.
    pub friendly_comfort_clearance_meters: f32,
    /// Desired hull-to-hull clearance for opposing units.
    pub opposing_comfort_clearance_meters: f32,
    pub friendly_strength: f32,
    pub opposing_strength: f32,
    pub prediction_horizon_seconds: f32,
    pub max_avoidance_acceleration: f32,
    pub opposing_speed_squared_scale: f32,
}

impl Default for AvoidanceConfig {
    fn default() -> Self {
        Self {
            friendly_comfort_clearance_meters: 2.0,
            opposing_comfort_clearance_meters: 4.0,
            friendly_strength: 8.0,
            opposing_strength: 24.0,
            prediction_horizon_seconds: 0.75,
            max_avoidance_acceleration: 12.0,
            opposing_speed_squared_scale: 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SimulationConfigError {
    #[error("fleet size must be greater than zero")]
    ZeroFleetSize,
    #[error("fleet size is too large for the simulation")]
    FleetSizeTooLarge,
    #[error("avoidance configuration contains an invalid value")]
    InvalidAvoidanceConfig,
    #[error("opposing comfort clearance must not be smaller than friendly comfort clearance")]
    OpposingClearanceTooSmall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationConfig {
    fleet_size: u32,
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
            avoidance: AvoidanceConfig::default(),
        })
    }

    pub const fn fleet_size(self) -> u32 {
        self.fleet_size
    }

    pub const fn avoidance(self) -> AvoidanceConfig {
        self.avoidance
    }

    pub fn with_avoidance(self, avoidance: AvoidanceConfig) -> Result<Self, SimulationConfigError> {
        let values = [
            avoidance.friendly_comfort_clearance_meters,
            avoidance.opposing_comfort_clearance_meters,
            avoidance.friendly_strength,
            avoidance.opposing_strength,
            avoidance.prediction_horizon_seconds,
            avoidance.max_avoidance_acceleration,
            avoidance.opposing_speed_squared_scale,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(SimulationConfigError::InvalidAvoidanceConfig);
        }
        if avoidance.opposing_comfort_clearance_meters < avoidance.friendly_comfort_clearance_meters
        {
            return Err(SimulationConfigError::OpposingClearanceTooSmall);
        }
        Ok(Self { avoidance, ..self })
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
