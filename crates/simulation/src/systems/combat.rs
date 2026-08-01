use std::collections::BTreeMap;

use glam::Vec2;
use spacegame2d_protocol::Tick;

use crate::combat::{
    CombatTargetId, FIRE_INTERVAL_TICKS, FIRING_TOLERANCE_RADIANS, ImpactEntityId,
    MUZZLE_OFFSET_METERS, TURRET_TRACKING_RADIANS_PER_SECOND, WEAPON_DAMAGE, WEAPON_RANGE_METERS,
};
use crate::command::{PlayerId, UnitId, World};
use crate::events::SimulationEvent;
use crate::hitbox::PositionedHitbox;
use crate::physics::{FIXED_DT_SECONDS, wrap_angle};

#[derive(Clone, Copy)]
struct TargetObservation {
    id: CombatTargetId,
    owner: Option<PlayerId>,
    position: Vec2,
}

#[derive(Clone, Copy)]
struct HitObservation {
    entity_id: ImpactEntityId,
    owner: Option<PlayerId>,
    hitbox: PositionedHitbox,
    command_core_exposed: bool,
    command_core_protected: bool,
}

#[derive(Clone, Copy)]
struct ImpactResolution {
    entity_id: ImpactEntityId,
    position: Vec2,
    owner: Option<PlayerId>,
    command_core_exposed: bool,
    command_core_protected: bool,
}

fn target_observations(world: &World) -> Vec<TargetObservation> {
    let mut observations = world
        .units
        .iter()
        .map(|unit| TargetObservation {
            id: CombatTargetId::Unit(unit.id),
            owner: unit.owner,
            position: unit.state.position,
        })
        .collect::<Vec<_>>();
    observations.extend(
        world
            .home_objective_pairs()
            .iter()
            .filter(|pair| pair.is_core_targetable())
            .map(|pair| {
                let core = world
                    .structures()
                    .iter()
                    .find(|structure| structure.id() == pair.core_id())
                    .expect("home objective Core must exist");
                TargetObservation {
                    id: CombatTargetId::CommandCore(core.id()),
                    owner: Some(core.owner()),
                    position: core.position(),
                }
            }),
    );
    observations.sort_unstable_by_key(|observation| observation.id);
    observations
}

fn hit_observations(world: &World) -> Vec<HitObservation> {
    let mut observations = world
        .units
        .iter()
        .map(|unit| HitObservation {
            entity_id: ImpactEntityId::Unit(unit.id),
            owner: unit.owner,
            hitbox: unit.positioned_hitbox(),
            command_core_exposed: false,
            command_core_protected: false,
        })
        .chain(world.structures().iter().map(|structure| {
            let core_pair = world
                .home_objective_pairs()
                .iter()
                .find(|pair| pair.core_id() == structure.id());
            HitObservation {
                entity_id: ImpactEntityId::StaticStructure(structure.id()),
                owner: Some(structure.owner()),
                hitbox: structure.positioned_hitbox(),
                command_core_exposed: core_pair.is_some_and(|pair| pair.is_core_exposed()),
                command_core_protected: core_pair.is_some_and(|pair| !pair.is_core_exposed()),
            }
        }))
        .collect::<Vec<_>>();
    observations.sort_unstable_by_key(|observation| observation.entity_id);
    observations
}

fn is_hostile(owner: Option<PlayerId>, other_owner: Option<PlayerId>) -> bool {
    matches!((owner, other_owner), (Some(owner), Some(other_owner)) if owner != other_owner)
}

fn valid_target(
    target: Option<CombatTargetId>,
    owner: Option<PlayerId>,
    position: Vec2,
    observations: &[TargetObservation],
) -> Option<CombatTargetId> {
    let target = target?;
    observations
        .iter()
        .find(|observation| {
            observation.id == target
                && is_hostile(owner, observation.owner)
                && (observation.position - position).length_squared()
                    <= WEAPON_RANGE_METERS * WEAPON_RANGE_METERS
        })
        .map(|observation| observation.id)
}

fn nearest_hostile(
    owner: Option<PlayerId>,
    position: Vec2,
    observations: &[TargetObservation],
) -> Option<CombatTargetId> {
    observations
        .iter()
        .filter(|observation| is_hostile(owner, observation.owner))
        .filter_map(|observation| {
            let distance_squared = (observation.position - position).length_squared();
            (distance_squared <= WEAPON_RANGE_METERS * WEAPON_RANGE_METERS).then_some((
                distance_squared,
                target_rank(observation.id),
                observation.id,
            ))
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.cmp(&right.2))
        })
        .map(|(_, _, id)| id)
}

fn target_rank(target: CombatTargetId) -> u8 {
    match target {
        CombatTargetId::Unit(_) => 0,
        CombatTargetId::CommandCore(_) => 1,
    }
}

fn heading_toward(vector: Vec2) -> f32 {
    (-vector.x).atan2(vector.y)
}

fn forward_from_heading(heading: f32) -> Vec2 {
    Vec2::new(-heading.sin(), heading.cos())
}

fn first_impact(
    shooter_id: UnitId,
    owner: Option<PlayerId>,
    origin: Vec2,
    direction: Vec2,
    observations: &[HitObservation],
) -> Option<ImpactResolution> {
    observations
        .iter()
        .filter(|observation| match observation.entity_id {
            ImpactEntityId::Unit(unit_id) => {
                unit_id != shooter_id && is_hostile(owner, observation.owner)
            }
            ImpactEntityId::StaticStructure(_) => true,
        })
        .filter_map(|observation| {
            observation
                .hitbox
                .ray_entry_distance(origin, direction, WEAPON_RANGE_METERS)
                .map(|entry_distance| (entry_distance, *observation))
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then(left.1.entity_id.cmp(&right.1.entity_id))
        })
        .map(|(entry_distance, observation)| ImpactResolution {
            entity_id: observation.entity_id,
            position: origin + direction * entry_distance,
            owner: observation.owner,
            command_core_exposed: observation.command_core_exposed,
            command_core_protected: observation.command_core_protected,
        })
}

pub(crate) fn run(world: &mut World, tick: Tick, events: &mut Vec<SimulationEvent>) {
    let target_observations = target_observations(world);
    let hit_observations = hit_observations(world);
    let mut unit_damage = BTreeMap::new();
    let mut core_damage = BTreeMap::new();
    let mut shooters = world.units.iter().map(|unit| unit.id).collect::<Vec<_>>();
    shooters.sort_unstable();
    for shooter_id in shooters {
        let shooter_index = world
            .units
            .iter()
            .position(|unit| unit.id == shooter_id)
            .expect("combat observation must reference a live unit");
        let shooter = &mut world.units[shooter_index];
        let previous_target = shooter.combat.turret.target;
        let target = valid_target(
            previous_target,
            shooter.owner,
            shooter.state.position,
            &target_observations,
        )
        .or_else(|| nearest_hostile(shooter.owner, shooter.state.position, &target_observations));
        if target != previous_target {
            shooter.combat.turret.cooldown_ticks_remaining = 0;
        }
        shooter.combat.turret.target = target;
        if shooter.combat.turret.cooldown_ticks_remaining > 0 {
            shooter.combat.turret.cooldown_ticks_remaining -= 1;
        }
        let Some(target_id) = target else {
            continue;
        };
        let target_position = target_observations
            .iter()
            .find(|observation| observation.id == target_id)
            .expect("valid target must be observed")
            .position;
        let desired_world_heading = heading_toward(target_position - shooter.state.position);
        let desired_local_heading =
            wrap_angle(desired_world_heading - shooter.state.heading_radians);
        let delta = wrap_angle(desired_local_heading - shooter.combat.turret.local_heading_radians);
        let max_turn = TURRET_TRACKING_RADIANS_PER_SECOND * FIXED_DT_SECONDS;
        shooter.combat.turret.local_heading_radians = wrap_angle(
            shooter.combat.turret.local_heading_radians + delta.clamp(-max_turn, max_turn),
        );
        let world_heading =
            wrap_angle(shooter.state.heading_radians + shooter.combat.turret.local_heading_radians);
        let aligned =
            wrap_angle(desired_world_heading - world_heading).abs() <= FIRING_TOLERANCE_RADIANS;
        if !aligned || shooter.combat.turret.cooldown_ticks_remaining != 0 {
            continue;
        }
        let direction = forward_from_heading(world_heading);
        let muzzle_origin = shooter.state.position + direction * MUZZLE_OFFSET_METERS;
        let ray_endpoint = muzzle_origin + direction * WEAPON_RANGE_METERS;
        let impact = first_impact(
            shooter.id,
            shooter.owner,
            muzzle_origin,
            direction,
            &hit_observations,
        );
        let impact_entity = impact.map(|impact| impact.entity_id);
        let impact_position = impact.map_or(ray_endpoint, |impact| impact.position);
        if let Some(impact) = impact {
            match impact.entity_id {
                ImpactEntityId::Unit(hit_unit_id) => {
                    *unit_damage.entry(hit_unit_id).or_insert(0) += WEAPON_DAMAGE;
                }
                ImpactEntityId::StaticStructure(core_id)
                    if impact.command_core_exposed && is_hostile(shooter.owner, impact.owner) =>
                {
                    *core_damage.entry(core_id).or_insert(0) += WEAPON_DAMAGE;
                }
                ImpactEntityId::StaticStructure(core_id) if impact.command_core_protected => {
                    events.push(SimulationEvent::CoreHitProtected { tick, core_id });
                }
                ImpactEntityId::StaticStructure(_) => {}
            }
        }
        shooter.combat.turret.cooldown_ticks_remaining = FIRE_INTERVAL_TICKS;
        events.push(SimulationEvent::ShotFired {
            tick,
            shooter_id,
            muzzle_origin,
            ray_endpoint,
            impact_position,
            impact_entity,
        });
    }
    for (unit_id, amount) in unit_damage {
        if let Some(unit) = world.unit_mut(unit_id) {
            unit.combat.hull.current = unit.combat.hull.current.saturating_sub(amount);
        }
    }
    for (core_id, amount) in core_damage {
        if let Some(pair) = world.home_objective_pair_mut(core_id) {
            pair.apply_core_damage(amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::command::{PlayerId, UnitId};
    use crate::hitbox::Hitbox;
    use crate::structure::StaticStructureId;

    #[test]
    fn equal_distance_impacts_use_type_then_numeric_id_tie_breaks() {
        let circle = Hitbox::circle(1.0)
            .unwrap()
            .positioned_at(Vec2::new(0.0, 5.0));
        let unit = HitObservation {
            entity_id: ImpactEntityId::Unit(UnitId(9)),
            owner: Some(PlayerId(2)),
            hitbox: circle,
            command_core_exposed: false,
            command_core_protected: false,
        };
        let structure = HitObservation {
            entity_id: ImpactEntityId::StaticStructure(StaticStructureId(1)),
            owner: Some(PlayerId(1)),
            hitbox: circle,
            command_core_exposed: false,
            command_core_protected: false,
        };
        let first = first_impact(
            UnitId(1),
            Some(PlayerId(1)),
            Vec2::ZERO,
            Vec2::Y,
            &[structure, unit],
        );
        assert_eq!(first.unwrap().entity_id, ImpactEntityId::Unit(UnitId(9)));

        let lower_id = HitObservation {
            entity_id: ImpactEntityId::StaticStructure(StaticStructureId(2)),
            owner: Some(PlayerId(1)),
            hitbox: circle,
            command_core_exposed: false,
            command_core_protected: false,
        };
        let higher_id = HitObservation {
            entity_id: ImpactEntityId::StaticStructure(StaticStructureId(7)),
            owner: Some(PlayerId(1)),
            hitbox: circle,
            command_core_exposed: false,
            command_core_protected: false,
        };
        let first = first_impact(
            UnitId(1),
            Some(PlayerId(1)),
            Vec2::ZERO,
            Vec2::Y,
            &[higher_id, lower_id],
        );
        assert_eq!(
            first.unwrap().entity_id,
            ImpactEntityId::StaticStructure(StaticStructureId(2))
        );
    }
}
