//! Data-driven deterministic avoidance policy.

use thiserror::Error;

use super::NeighborRelationship;

/// Upper bound for independently capped avoidance accumulation groups.
pub const MAX_AVOIDANCE_GROUPS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AvoidanceGroupId(pub u8);

pub const MOBILE_AVOIDANCE_GROUP: AvoidanceGroupId = AvoidanceGroupId(1);
pub const STRUCTURE_AVOIDANCE_GROUP: AvoidanceGroupId = AvoidanceGroupId(2);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidanceGroup {
    id: AvoidanceGroupId,
    max_acceleration: f32,
}

impl AvoidanceGroup {
    pub fn new(id: AvoidanceGroupId, max_acceleration: f32) -> Result<Self, AvoidanceError> {
        validate_non_negative(max_acceleration)?;
        Ok(Self {
            id,
            max_acceleration,
        })
    }

    pub const fn id(self) -> AvoidanceGroupId {
        self.id
    }

    pub const fn max_acceleration(self) -> f32 {
        self.max_acceleration
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvoidanceProfile {
    relationship: NeighborRelationship,
    group: AvoidanceGroupId,
    comfort_clearance_meters: f32,
    strength: f32,
    speed_squared_scale: f32,
}

impl AvoidanceProfile {
    pub fn new(
        relationship: NeighborRelationship,
        group: AvoidanceGroupId,
        comfort_clearance_meters: f32,
        strength: f32,
        speed_squared_scale: f32,
    ) -> Result<Self, AvoidanceError> {
        for value in [comfort_clearance_meters, strength, speed_squared_scale] {
            validate_non_negative(value)?;
        }
        Ok(Self {
            relationship,
            group,
            comfort_clearance_meters,
            strength,
            speed_squared_scale,
        })
    }

    pub const fn relationship(self) -> NeighborRelationship {
        self.relationship
    }

    pub const fn group(self) -> AvoidanceGroupId {
        self.group
    }

    pub const fn comfort_clearance_meters(self) -> f32 {
        self.comfort_clearance_meters
    }

    pub const fn strength(self) -> f32 {
        self.strength
    }

    pub const fn speed_squared_scale(self) -> f32 {
        self.speed_squared_scale
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AvoidanceProfiles {
    prediction_horizon_seconds: f32,
    groups: Vec<AvoidanceGroup>,
    profiles: Vec<AvoidanceProfile>,
}

impl AvoidanceProfiles {
    pub fn new(
        prediction_horizon_seconds: f32,
        groups: Vec<AvoidanceGroup>,
        profiles: Vec<AvoidanceProfile>,
    ) -> Result<Self, AvoidanceError> {
        validate_non_negative(prediction_horizon_seconds)?;
        if groups.is_empty() || profiles.is_empty() {
            return Err(AvoidanceError::EmptyRegistry);
        }
        if groups.len() > MAX_AVOIDANCE_GROUPS {
            return Err(AvoidanceError::TooManyGroups);
        }
        if groups
            .iter()
            .enumerate()
            .any(|(index, group)| groups[..index].iter().any(|prior| prior.id == group.id))
        {
            return Err(AvoidanceError::DuplicateGroup);
        }
        if profiles
            .iter()
            .any(|profile| !groups.iter().any(|group| group.id == profile.group))
        {
            return Err(AvoidanceError::UnknownGroup);
        }
        Ok(Self {
            prediction_horizon_seconds,
            groups,
            profiles,
        })
    }

    pub const fn prediction_horizon_seconds(&self) -> f32 {
        self.prediction_horizon_seconds
    }

    pub fn groups(&self) -> &[AvoidanceGroup] {
        &self.groups
    }

    pub fn profiles(&self) -> &[AvoidanceProfile] {
        &self.profiles
    }

    pub fn group_index(&self, id: AvoidanceGroupId) -> usize {
        self.groups
            .iter()
            .position(|group| group.id == id)
            .expect("validated avoidance profile group must exist")
    }
}

impl Default for AvoidanceProfiles {
    fn default() -> Self {
        Self::new(
            0.75,
            vec![
                AvoidanceGroup::new(MOBILE_AVOIDANCE_GROUP, 12.0)
                    .expect("built-in mobile avoidance group is valid"),
                AvoidanceGroup::new(STRUCTURE_AVOIDANCE_GROUP, 24.0)
                    .expect("built-in structure avoidance group is valid"),
            ],
            vec![
                AvoidanceProfile::new(
                    NeighborRelationship::Friendly,
                    MOBILE_AVOIDANCE_GROUP,
                    2.0,
                    8.0,
                    0.0,
                )
                .expect("built-in friendly avoidance profile is valid"),
                AvoidanceProfile::new(
                    NeighborRelationship::Opposing,
                    MOBILE_AVOIDANCE_GROUP,
                    4.0,
                    24.0,
                    1.5,
                )
                .expect("built-in opposing avoidance profile is valid"),
                AvoidanceProfile::new(
                    NeighborRelationship::StaticStructure,
                    STRUCTURE_AVOIDANCE_GROUP,
                    6.0,
                    48.0,
                    2.0,
                )
                .expect("built-in structure avoidance profile is valid"),
            ],
        )
        .expect("built-in avoidance registry is valid")
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AvoidanceError {
    #[error("avoidance values must be finite and non-negative")]
    InvalidValue,
    #[error("avoidance registry requires at least one group and profile")]
    EmptyRegistry,
    #[error("avoidance registry has too many accumulation groups")]
    TooManyGroups,
    #[error("avoidance group identifiers must be unique")]
    DuplicateGroup,
    #[error("avoidance profile references an unknown accumulation group")]
    UnknownGroup,
}

fn validate_non_negative(value: f32) -> Result<(), AvoidanceError> {
    if !value.is_finite() || value < 0.0 {
        return Err(AvoidanceError::InvalidValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_register_profiles_and_groups_in_stable_order() {
        let profiles = AvoidanceProfiles::default();
        assert_eq!(
            profiles
                .groups()
                .iter()
                .map(|group| group.id())
                .collect::<Vec<_>>(),
            vec![MOBILE_AVOIDANCE_GROUP, STRUCTURE_AVOIDANCE_GROUP]
        );
        assert_eq!(
            profiles
                .profiles()
                .iter()
                .map(|profile| profile.relationship())
                .collect::<Vec<_>>(),
            vec![
                NeighborRelationship::Friendly,
                NeighborRelationship::Opposing,
                NeighborRelationship::StaticStructure,
            ]
        );
    }

    #[test]
    fn registry_rejects_invalid_group_registration() {
        let group = AvoidanceGroup::new(MOBILE_AVOIDANCE_GROUP, 1.0).unwrap();
        let profile = AvoidanceProfile::new(
            NeighborRelationship::Friendly,
            STRUCTURE_AVOIDANCE_GROUP,
            1.0,
            1.0,
            0.0,
        )
        .unwrap();
        assert_eq!(
            AvoidanceProfiles::new(0.5, vec![group], vec![profile]),
            Err(AvoidanceError::UnknownGroup)
        );
        assert_eq!(
            AvoidanceGroup::new(MOBILE_AVOIDANCE_GROUP, f32::NAN),
            Err(AvoidanceError::InvalidValue)
        );
    }
}
