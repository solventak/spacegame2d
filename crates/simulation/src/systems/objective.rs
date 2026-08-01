use spacegame2d_protocol::Tick;

use crate::command::World;
use crate::events::{MatchResult, SimulationEvent};
use crate::objective::{CAPTURE_RADIUS_SQUARED_METERS, ObjectivePresence, advance_pair};

pub(crate) fn run(
    world: &mut World,
    tick: Tick,
    frozen: bool,
    events: &mut Vec<SimulationEvent>,
) -> Option<MatchResult> {
    let destroyed_cores = world
        .home_objective_pairs()
        .iter()
        .filter(|pair| pair.core_health_current() == 0)
        .map(|pair| (pair.owner(), pair.core_id()))
        .collect::<Vec<_>>();
    if !destroyed_cores.is_empty() {
        return Some(if destroyed_cores.len() == 2 {
            MatchResult::Draw {
                destroyed_cores: [destroyed_cores[0].1, destroyed_cores[1].1],
            }
        } else {
            let (loser, destroyed_core) = destroyed_cores[0];
            let winner = world
                .home_objective_pairs()
                .iter()
                .find(|pair| pair.owner() != loser)
                .expect("two-player match has an opposing Core")
                .owner();
            MatchResult::Victory {
                winner,
                loser,
                destroyed_core,
            }
        });
    }

    let samples = world
        .home_objective_pairs()
        .iter()
        .map(|pair| {
            let relay = world
                .structures()
                .iter()
                .find(|structure| structure.id() == pair.relay_id())
                .expect("home objective relay must exist");
            let mut presence = ObjectivePresence::default();
            for unit in &world.units {
                let Some(owner) = unit.owner else {
                    continue;
                };
                if unit.state.position.distance_squared(relay.position())
                    > CAPTURE_RADIUS_SQUARED_METERS
                {
                    continue;
                }
                if owner == pair.owner() {
                    presence.has_defender = true;
                } else {
                    presence.has_attacker = true;
                }
            }
            (pair.owner(), pair.relay_id(), pair.core_id(), presence)
        })
        .collect::<Vec<_>>();
    for ((owner, relay_id, core_id, presence), pair) in
        samples.into_iter().zip(world.home_objective_pairs_mut())
    {
        if let Some((previous_state, next_state)) = advance_pair(pair, presence, frozen) {
            events.push(SimulationEvent::ObjectiveTransition {
                tick,
                owner,
                relay_id,
                core_id,
                previous_state,
                next_state,
            });
        }
    }
    None
}
