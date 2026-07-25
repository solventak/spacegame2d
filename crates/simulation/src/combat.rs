//! Persistent deterministic combat state and first-pass weapon constants.

use crate::command::UnitId;

pub const MAX_HULL: u32 = 100;
pub const WEAPON_RANGE_METERS: f32 = 10.0;
pub const TURRET_TRACKING_RADIANS_PER_SECOND: f32 = 2.0;
pub const FIRING_TOLERANCE_RADIANS: f32 = 0.15;
pub const FIRE_INTERVAL_TICKS: u32 = 30;
pub const WEAPON_DAMAGE: u32 = 10;
pub const TARGET_HIT_RADIUS_METERS: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HullState {
    pub current: u32,
    pub maximum: u32,
}
impl HullState {
    pub const fn full() -> Self {
        Self {
            current: MAX_HULL,
            maximum: MAX_HULL,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurretState {
    pub heading_radians: f32,
    pub target: Option<UnitId>,
    pub cooldown_ticks_remaining: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatState {
    pub hull: HullState,
    pub turret: TurretState,
}
impl CombatState {
    pub const fn new(initial_heading_radians: f32) -> Self {
        Self {
            hull: HullState::full(),
            turret: TurretState {
                heading_radians: initial_heading_radians,
                target: None,
                cooldown_ticks_remaining: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_combat_state_is_ready_to_engage() {
        let combat = CombatState::new(0.75);
        assert_eq!(combat.hull, HullState::full());
        assert_eq!(combat.turret.heading_radians, 0.75);
        assert_eq!(combat.turret.target, None);
        assert_eq!(combat.turret.cooldown_ticks_remaining, 0);
    }
}
