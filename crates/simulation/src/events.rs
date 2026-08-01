use crate::combat::ImpactEntityId;
use crate::command::{PlayerId, UnitId};
use crate::objective::ObjectiveState;
use crate::structure::StaticStructureId;
use glam::Vec2;
use spacegame2d_protocol::Tick;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationEvent {
    ShotFired {
        tick: Tick,
        shooter_id: UnitId,
        muzzle_origin: Vec2,
        ray_endpoint: Vec2,
        impact_position: Vec2,
        impact_entity: Option<ImpactEntityId>,
    },
    CoreHitProtected {
        tick: Tick,
        core_id: StaticStructureId,
    },
    HullDepleted {
        tick: Tick,
        unit_id: UnitId,
        position: Vec2,
    },
    BoundaryCrossed {
        tick: Tick,
        unit_id: UnitId,
        position: Vec2,
    },
    ObjectiveTransition {
        tick: Tick,
        owner: PlayerId,
        relay_id: StaticStructureId,
        core_id: StaticStructureId,
        previous_state: ObjectiveState,
        next_state: ObjectiveState,
    },
    MatchResult {
        tick: Tick,
        outcome: MatchResult,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchResult {
    Victory {
        winner: PlayerId,
        loser: PlayerId,
        destroyed_core: StaticStructureId,
    },
    Draw {
        destroyed_cores: [StaticStructureId; 2],
    },
}

pub(crate) fn simulation_event_sort_key(event: &SimulationEvent) -> (u8, u32, u32) {
    match event {
        SimulationEvent::ShotFired { shooter_id, .. } => (0, shooter_id.0, 0),
        SimulationEvent::CoreHitProtected { core_id, .. } => (1, core_id.0, 0),
        SimulationEvent::HullDepleted { unit_id, .. } => (2, unit_id.0, 0),
        SimulationEvent::BoundaryCrossed { unit_id, .. } => (3, unit_id.0, 0),
        SimulationEvent::ObjectiveTransition { owner, core_id, .. } => {
            (4, owner.0 as u32, core_id.0)
        }
        SimulationEvent::MatchResult { .. } => (5, 0, 0),
    }
}
