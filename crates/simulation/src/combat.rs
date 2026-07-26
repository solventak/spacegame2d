//! Persistent deterministic combat state and first-pass weapon constants.

use crate::{command::UnitId, structure::StaticStructureId};

pub const MAX_HULL: u32 = 100;
pub const WEAPON_RANGE_METERS: f32 = 12.0;
pub const TURRET_TRACKING_RADIANS_PER_SECOND: f32 = std::f32::consts::PI;
pub const FIRING_TOLERANCE_RADIANS: f32 = 0.10;
pub const FIRE_INTERVAL_TICKS: u32 = 15;
/// Each hit removes roughly one third of the previous hull damage, giving
/// ships enough time to maneuver and retarget during an engagement.
pub const WEAPON_DAMAGE: u32 = 6;
pub const MUZZLE_OFFSET_METERS: f32 = 0.5;

/// Stable identity of an entity a shot physically impacts.
///
/// Variant order is intentional: when two ray entries are equally distant, a
/// unit wins over a static structure, then the numeric ID breaks ties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ImpactEntityId {
    Unit(UnitId),
    StaticStructure(StaticStructureId),
}

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
    /// Barrel angle in the hull's local frame. The world barrel direction is
    /// this angle plus the ship heading.
    pub local_heading_radians: f32,
    pub target: Option<UnitId>,
    pub cooldown_ticks_remaining: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatState {
    pub hull: HullState,
    pub turret: TurretState,
}
impl CombatState {
    pub const fn new() -> Self {
        Self {
            hull: HullState::full(),
            turret: TurretState {
                local_heading_radians: 0.0,
                target: None,
                cooldown_ticks_remaining: 0,
            },
        }
    }
}

impl Default for CombatState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_combat_state_is_ready_to_engage() {
        let combat = CombatState::new();
        assert_eq!(combat.hull, HullState::full());
        assert_eq!(combat.turret.local_heading_radians, 0.0);
        assert_eq!(combat.turret.target, None);
        assert_eq!(combat.turret.cooldown_ticks_remaining, 0);
    }
}
