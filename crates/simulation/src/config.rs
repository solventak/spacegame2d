use thiserror::Error;

pub const DEFAULT_FLEET_SIZE: u32 = 30;
pub const DEFAULT_WORLD_RADIUS_METERS: f32 = 64.0;
pub const MAX_PLAYERS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidanceConfig {
    pub friendly_comfort_radius_meters: f32,
    pub opposing_comfort_radius_meters: f32,
    pub friendly_strength: f32,
    pub opposing_strength: f32,
    pub prediction_horizon_seconds: f32,
    pub max_avoidance_acceleration: f32,
    pub opposing_speed_squared_scale: f32,
}

impl Default for AvoidanceConfig {
    fn default() -> Self {
        Self {
            friendly_comfort_radius_meters: 2.0,
            opposing_comfort_radius_meters: 4.0,
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
    #[error("world radius must be finite and greater than zero")]
    InvalidWorldRadius,
    #[error("avoidance configuration contains an invalid value")]
    InvalidAvoidanceConfig,
    #[error("opposing comfort radius must not be smaller than friendly comfort radius")]
    OpposingRadiusTooSmall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

    pub const fn fleet_size(self) -> u32 {
        self.fleet_size
    }

    pub const fn avoidance(self) -> AvoidanceConfig {
        self.avoidance
    }

    pub const fn world_radius_meters(self) -> f32 {
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

    pub fn with_avoidance(self, avoidance: AvoidanceConfig) -> Result<Self, SimulationConfigError> {
        let values = [
            avoidance.friendly_comfort_radius_meters,
            avoidance.opposing_comfort_radius_meters,
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
        if avoidance.opposing_comfort_radius_meters < avoidance.friendly_comfort_radius_meters {
            return Err(SimulationConfigError::OpposingRadiusTooSmall);
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
