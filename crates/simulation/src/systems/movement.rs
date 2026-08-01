use glam::Vec2;

use crate::command::World;
use crate::flight_control::{AvoidanceEntityId, NeighborObservation, NeighborRelationship};
use crate::hitbox::Hitbox;
use crate::simulation::step_ship;

/// Immutable unit state sampled at the start of a simulation tick.
#[derive(Clone, Copy)]
struct TickAvoidanceObservation {
    entity_id: AvoidanceEntityId,
    owner: Option<crate::command::PlayerId>,
    position: Vec2,
    velocity: Vec2,
    hitbox: Hitbox,
}

fn avoidance_observations(world: &World) -> Vec<TickAvoidanceObservation> {
    let mut observations: Vec<TickAvoidanceObservation> = world
        .units
        .iter()
        .map(|unit| TickAvoidanceObservation {
            entity_id: AvoidanceEntityId::Unit(unit.id),
            owner: unit.owner,
            position: unit.state.position,
            velocity: unit.state.velocity,
            hitbox: unit.hitbox(),
        })
        .collect();
    observations.extend(
        world
            .structures()
            .iter()
            .map(|structure| TickAvoidanceObservation {
                entity_id: AvoidanceEntityId::StaticStructure(structure.id()),
                owner: Some(structure.owner()),
                position: structure.position(),
                velocity: Vec2::ZERO,
                hitbox: structure.hitbox(),
            }),
    );
    observations.sort_unstable_by_key(|observation| observation.entity_id);
    observations
}

pub(crate) fn run(world: &mut World) {
    let observations = avoidance_observations(world);
    for unit in world.units.iter_mut() {
        let owner = unit.owner;
        let neighbors = observations
            .iter()
            .filter(|neighbor| neighbor.entity_id != AvoidanceEntityId::Unit(unit.id))
            .map(|neighbor| NeighborObservation {
                entity_id: neighbor.entity_id,
                position: neighbor.position,
                velocity: neighbor.velocity,
                hitbox: neighbor.hitbox,
                relationship: match neighbor.entity_id {
                    AvoidanceEntityId::StaticStructure(_) => NeighborRelationship::StaticStructure,
                    AvoidanceEntityId::Unit(_) if owner.is_some() && owner == neighbor.owner => {
                        NeighborRelationship::Friendly
                    }
                    AvoidanceEntityId::Unit(_) => NeighborRelationship::Opposing,
                },
            })
            .collect::<Vec<_>>();
        let input =
            unit.autopilot
                .controls_for_tick_with_hitbox(&unit.state, unit.hitbox(), &neighbors);
        step_ship(&mut unit.state, input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::flight_control::{AvoidanceEntityId, NeighborObservation, NeighborRelationship};
    use crate::simulation::Simulation;

    #[test]
    fn owned_static_structures_remain_stationary_static_avoidance_observations() {
        let mut simulation = Simulation::default();
        simulation.world.units.truncate(1);
        let nearby_structure = simulation.world.structures()[3];
        let unit_position = nearby_structure.position() - Vec2::X * 4.0;
        let destination = unit_position + Vec2::Y * 10.0;
        simulation.world.units[0].state.position = unit_position;
        simulation.world.units[0]
            .autopilot
            .set_destination(destination);

        let observations = avoidance_observations(&simulation.world);
        assert_eq!(
            observations
                .iter()
                .map(|observation| observation.entity_id)
                .collect::<Vec<_>>(),
            vec![
                AvoidanceEntityId::Unit(simulation.world.units[0].id),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(1)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(2)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(3)),
                AvoidanceEntityId::StaticStructure(crate::StaticStructureId(4)),
            ]
        );
        for (observation, structure) in observations[1..].iter().zip(simulation.world.structures())
        {
            assert_eq!(observation.position, structure.position());
            assert_eq!(observation.velocity, Vec2::ZERO);
            assert_eq!(observation.owner, Some(structure.owner()));
        }

        let unit = &mut simulation.world.units[0];
        let neighbors = observations[1..]
            .iter()
            .map(|observation| NeighborObservation {
                entity_id: observation.entity_id,
                position: observation.position,
                velocity: observation.velocity,
                hitbox: observation.hitbox,
                relationship: NeighborRelationship::StaticStructure,
            })
            .collect::<Vec<_>>();
        let with_structure =
            unit.autopilot
                .controls_for_tick_with_hitbox(&unit.state, unit.hitbox(), &neighbors);
        unit.autopilot.set_destination(destination);
        let without_structure =
            unit.autopilot
                .controls_for_tick_with_hitbox(&unit.state, unit.hitbox(), &[]);
        assert_ne!(with_structure, without_structure);
    }
}
