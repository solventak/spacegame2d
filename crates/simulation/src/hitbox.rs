//! Unit-owned physical geometry used by simulation systems.
//!
//! A [`Hitbox`] describes a unit in local space. It has no independently
//! mutable world position; callers resolve it at an owning unit's current
//! position with [`Hitbox::positioned_at`]. This keeps geometry coupled to the
//! authoritative unit state without duplicating a center coordinate.

use glam::Vec2;
use thiserror::Error;

/// Radius of the shared movable-ship hull used by the initial hitbox system.
pub const DEFAULT_SHIP_HITBOX_RADIUS_METERS: f32 = 0.60;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum HitboxError {
    #[error("circle radius must be finite and non-negative")]
    InvalidCircleRadius,
}

/// Circular local-space geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    radius_meters: f32,
}

impl Circle {
    pub fn new(radius_meters: f32) -> Result<Self, HitboxError> {
        if !radius_meters.is_finite() || radius_meters < 0.0 {
            return Err(HitboxError::InvalidCircleRadius);
        }
        Ok(Self { radius_meters })
    }

    pub const fn radius_meters(self) -> f32 {
        self.radius_meters
    }
}

/// Extensible local-space hitbox shape model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitboxShape {
    Circle(Circle),
}

/// Physical geometry owned by a unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    shape: HitboxShape,
}

impl Hitbox {
    pub fn circle(radius_meters: f32) -> Result<Self, HitboxError> {
        Ok(Self {
            shape: HitboxShape::Circle(Circle::new(radius_meters)?),
        })
    }

    pub fn default_ship() -> Self {
        Self::circle(DEFAULT_SHIP_HITBOX_RADIUS_METERS)
            .expect("the built-in ship hitbox radius is valid")
    }

    pub const fn shape(self) -> HitboxShape {
        self.shape
    }

    pub fn positioned_at(self, center: Vec2) -> PositionedHitbox {
        PositionedHitbox {
            center,
            shape: self.shape,
        }
    }
}

/// A hitbox resolved into arena space for a single observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedHitbox {
    center: Vec2,
    shape: HitboxShape,
}

impl PositionedHitbox {
    pub const fn center(self) -> Vec2 {
        self.center
    }

    pub const fn shape(self) -> HitboxShape {
        self.shape
    }

    pub fn contact_distance_to(self, other: Self) -> f32 {
        match (self.shape, other.shape) {
            (HitboxShape::Circle(left), HitboxShape::Circle(right)) => {
                left.radius_meters() + right.radius_meters()
            }
        }
    }

    pub fn clearance_to(self, other: Self) -> f32 {
        self.center.distance(other.center) - self.contact_distance_to(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_validates_its_radius() {
        assert_eq!(Circle::new(0.6).unwrap().radius_meters(), 0.6);
        assert_eq!(Circle::new(0.0).unwrap().radius_meters(), 0.0);
        for radius in [-0.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(Circle::new(radius), Err(HitboxError::InvalidCircleRadius));
        }
    }

    #[test]
    fn circle_clearance_is_signed_and_symmetric() {
        let left = Hitbox::circle(0.6).unwrap().positioned_at(Vec2::ZERO);
        let separated = Hitbox::circle(0.4).unwrap().positioned_at(Vec2::X * 2.0);
        let tangent = Hitbox::circle(0.4).unwrap().positioned_at(Vec2::X);
        let overlapping = Hitbox::circle(0.4).unwrap().positioned_at(Vec2::X * 0.5);

        assert_eq!(left.contact_distance_to(separated), 1.0);
        assert_eq!(left.clearance_to(separated), 1.0);
        assert_eq!(left.clearance_to(tangent), 0.0);
        assert_eq!(left.clearance_to(overlapping), -0.5);
        assert_eq!(left.clearance_to(separated), separated.clearance_to(left));
    }
}
