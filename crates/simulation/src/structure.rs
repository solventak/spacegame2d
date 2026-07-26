//! Deterministic static world structures.
//!
//! Structures are world objects with identity, position, collision geometry,
//! and render-relevant kind data. They deliberately do not carry movable-unit
//! state such as kinematics, ownership, autopilot, or combat.

use glam::Vec2;

use crate::hitbox::{Hitbox, PositionedHitbox};

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
    kind: StaticStructureKind,
    position: Vec2,
    visual_radius_meters: f32,
    hitbox: Hitbox,
}

impl StaticStructure {
    pub(crate) fn new(
        id: StaticStructureId,
        kind: StaticStructureKind,
        position: Vec2,
        visual_radius_meters: f32,
        hitbox: Hitbox,
    ) -> Self {
        Self {
            id,
            kind,
            position,
            visual_radius_meters,
            hitbox,
        }
    }

    pub const fn id(self) -> StaticStructureId {
        self.id
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

pub(crate) fn initial_static_structures() -> Vec<StaticStructure> {
    vec![
        StaticStructure::new(
            StaticStructureId(1),
            StaticStructureKind::CommandCore,
            Vec2::ZERO,
            3.5,
            Hitbox::circle(3.85).expect("the built-in command-core hitbox is valid"),
        ),
        StaticStructure::new(
            StaticStructureId(2),
            StaticStructureKind::ShieldRelay,
            Vec2::new(0.0, 10.0),
            2.5,
            Hitbox::circle(2.75).expect("the built-in shield-relay hitbox is valid"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitbox::HitboxShape;

    #[test]
    fn initial_definitions_are_stable_and_complete() {
        let structures = initial_static_structures();
        assert_eq!(structures.len(), 2);
        assert_eq!(structures[0].id(), StaticStructureId(1));
        assert_eq!(structures[0].kind(), StaticStructureKind::CommandCore);
        assert_eq!(structures[0].position(), Vec2::ZERO);
        assert_eq!(structures[0].visual_radius_meters(), 3.5);
        assert_eq!(structures[1].id(), StaticStructureId(2));
        assert_eq!(structures[1].kind(), StaticStructureKind::ShieldRelay);
        assert_eq!(structures[1].position(), Vec2::new(0.0, 10.0));
        assert_eq!(structures[1].visual_radius_meters(), 2.5);
        for (structure, radius) in structures.iter().zip([3.85, 2.75]) {
            assert_eq!(
                structure.hitbox().shape(),
                HitboxShape::Circle(crate::hitbox::Circle::new(radius).unwrap())
            );
        }
    }
}
