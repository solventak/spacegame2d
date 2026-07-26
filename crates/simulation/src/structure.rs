//! Deterministic static world structures.
//!
//! Structures are world objects with identity, position, collision geometry,
//! and render-relevant kind data. They deliberately do not carry movable-unit
//! state such as kinematics, ownership, autopilot, or combat.

use glam::Vec2;

use crate::command::PlayerId;
use crate::hitbox::{Hitbox, PositionedHitbox};

const COMMAND_CORE_VISUAL_RADIUS_METERS: f32 = 3.5;
const COMMAND_CORE_HITBOX_RADIUS_METERS: f32 = 3.85;
const SHIELD_RELAY_VISUAL_RADIUS_METERS: f32 = 2.5;
const SHIELD_RELAY_HITBOX_RADIUS_METERS: f32 = 2.75;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StaticStructureId(pub u32);

impl From<u32> for StaticStructureId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StaticStructureKind {
    CommandCore,
    ShieldRelay,
}

impl StaticStructureKind {
    pub const fn canonical_tag(self) -> u8 {
        match self {
            Self::CommandCore => 1,
            Self::ShieldRelay => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaticStructure {
    id: StaticStructureId,
    owner: PlayerId,
    kind: StaticStructureKind,
    position: Vec2,
    visual_radius_meters: f32,
    hitbox: Hitbox,
}

impl StaticStructure {
    pub(crate) fn new(
        id: StaticStructureId,
        owner: PlayerId,
        kind: StaticStructureKind,
        position: Vec2,
        visual_radius_meters: f32,
        hitbox: Hitbox,
    ) -> Self {
        Self {
            id,
            owner,
            kind,
            position,
            visual_radius_meters,
            hitbox,
        }
    }

    pub const fn id(self) -> StaticStructureId {
        self.id
    }

    pub const fn owner(self) -> PlayerId {
        self.owner
    }

    pub const fn kind(self) -> StaticStructureKind {
        self.kind
    }

    pub const fn position(self) -> Vec2 {
        self.position
    }

    pub const fn visual_radius_meters(self) -> f32 {
        self.visual_radius_meters
    }

    pub const fn hitbox(self) -> Hitbox {
        self.hitbox
    }

    pub fn positioned_hitbox(self) -> PositionedHitbox {
        self.hitbox.positioned_at(self.position)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeObjectivePair {
    owner: PlayerId,
    core_id: StaticStructureId,
    relay_id: StaticStructureId,
}

impl HomeObjectivePair {
    pub const fn owner(self) -> PlayerId {
        self.owner
    }

    pub const fn core_id(self) -> StaticStructureId {
        self.core_id
    }

    pub const fn relay_id(self) -> StaticStructureId {
        self.relay_id
    }
}

#[derive(Clone, Copy)]
pub(crate) struct HomeAreaDefinition {
    pub(crate) owner: PlayerId,
    pub(crate) core_id: StaticStructureId,
    pub(crate) core_position: Vec2,
    pub(crate) relay_id: StaticStructureId,
    pub(crate) relay_position: Vec2,
    pub(crate) fleet_spawn_center: Vec2,
}

pub(crate) const HOME_AREA_DEFINITIONS: [HomeAreaDefinition; 2] = [
    HomeAreaDefinition {
        owner: PlayerId(1),
        core_id: StaticStructureId(1),
        core_position: Vec2::new(-20.0, 0.0),
        relay_id: StaticStructureId(2),
        relay_position: Vec2::new(-10.0, 0.0),
        fleet_spawn_center: Vec2::new(-26.0, 0.0),
    },
    HomeAreaDefinition {
        owner: PlayerId(2),
        core_id: StaticStructureId(3),
        core_position: Vec2::new(20.0, 0.0),
        relay_id: StaticStructureId(4),
        relay_position: Vec2::new(10.0, 0.0),
        fleet_spawn_center: Vec2::new(26.0, 0.0),
    },
];

pub(crate) fn initial_home_objectives() -> (Vec<StaticStructure>, Vec<HomeObjectivePair>) {
    let mut structures = Vec::with_capacity(HOME_AREA_DEFINITIONS.len() * 2);
    let mut pairs = Vec::with_capacity(HOME_AREA_DEFINITIONS.len());
    for home in HOME_AREA_DEFINITIONS {
        structures.push(StaticStructure::new(
            home.core_id,
            home.owner,
            StaticStructureKind::CommandCore,
            home.core_position,
            COMMAND_CORE_VISUAL_RADIUS_METERS,
            Hitbox::circle(COMMAND_CORE_HITBOX_RADIUS_METERS)
                .expect("the built-in command-core hitbox is valid"),
        ));
        structures.push(StaticStructure::new(
            home.relay_id,
            home.owner,
            StaticStructureKind::ShieldRelay,
            home.relay_position,
            SHIELD_RELAY_VISUAL_RADIUS_METERS,
            Hitbox::circle(SHIELD_RELAY_HITBOX_RADIUS_METERS)
                .expect("the built-in shield-relay hitbox is valid"),
        ));
        pairs.push(HomeObjectivePair {
            owner: home.owner,
            core_id: home.core_id,
            relay_id: home.relay_id,
        });
    }
    (structures, pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitbox::HitboxShape;

    #[test]
    fn initial_definitions_are_stable_and_complete() {
        let (structures, pairs) = initial_home_objectives();
        assert_eq!(structures.len(), 4);
        assert_eq!(structures[0].id(), StaticStructureId(1));
        assert_eq!(structures[0].owner(), PlayerId(1));
        assert_eq!(structures[0].kind(), StaticStructureKind::CommandCore);
        assert_eq!(structures[0].position(), Vec2::new(-20.0, 0.0));
        assert_eq!(structures[0].visual_radius_meters(), 3.5);
        assert_eq!(structures[1].id(), StaticStructureId(2));
        assert_eq!(structures[1].owner(), PlayerId(1));
        assert_eq!(structures[1].kind(), StaticStructureKind::ShieldRelay);
        assert_eq!(structures[1].position(), Vec2::new(-10.0, 0.0));
        assert_eq!(structures[1].visual_radius_meters(), 2.5);
        assert_eq!(structures[2].id(), StaticStructureId(3));
        assert_eq!(structures[2].owner(), PlayerId(2));
        assert_eq!(structures[2].kind(), StaticStructureKind::CommandCore);
        assert_eq!(structures[2].position(), Vec2::new(20.0, 0.0));
        assert_eq!(structures[3].id(), StaticStructureId(4));
        assert_eq!(structures[3].owner(), PlayerId(2));
        assert_eq!(structures[3].kind(), StaticStructureKind::ShieldRelay);
        assert_eq!(structures[3].position(), Vec2::new(10.0, 0.0));
        for (structure, radius) in structures.iter().zip([3.85, 2.75, 3.85, 2.75]) {
            assert_eq!(
                structure.hitbox().shape(),
                HitboxShape::Circle(crate::hitbox::Circle::new(radius).unwrap())
            );
        }
        assert_eq!(
            pairs,
            vec![
                HomeObjectivePair {
                    owner: PlayerId(1),
                    core_id: StaticStructureId(1),
                    relay_id: StaticStructureId(2),
                },
                HomeObjectivePair {
                    owner: PlayerId(2),
                    core_id: StaticStructureId(3),
                    relay_id: StaticStructureId(4),
                },
            ]
        );
        assert_eq!(
            HOME_AREA_DEFINITIONS.map(|home| home.fleet_spawn_center),
            [Vec2::new(-26.0, 0.0), Vec2::new(26.0, 0.0)]
        );
    }
}
