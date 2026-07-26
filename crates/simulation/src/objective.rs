//! Authoritative deterministic Shield Relay breach state.

use crate::structure::HomeObjectivePair;

use crate::simulation::SIMULATION_HZ;

pub const CAPTURE_RADIUS_METERS: f32 = 10.0;
pub const CAPTURE_RADIUS_SQUARED_METERS: f32 = CAPTURE_RADIUS_METERS * CAPTURE_RADIUS_METERS;
pub const BREACH_DURATION_TICKS: u32 = 12 * SIMULATION_HZ;
pub const DECAY_DURATION_TICKS: u32 = 6 * SIMULATION_HZ;
pub const EXPOSURE_DURATION_TICKS: u32 = 8 * SIMULATION_HZ;
pub const BREACH_PROGRESS_PER_TICK: u32 = 1;
pub const BREACH_DECAY_PER_TICK: u32 = BREACH_DURATION_TICKS / DECAY_DURATION_TICKS;

const _: () = assert!(BREACH_DURATION_TICKS % DECAY_DURATION_TICKS == 0);
const _: () = assert!(BREACH_DECAY_PER_TICK > 0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectiveState {
    Protected,
    Breaching,
    Contested,
    Decaying,
    Exposed,
}

impl ObjectiveState {
    pub const fn canonical_tag(self) -> u8 {
        match self {
            Self::Protected => 1,
            Self::Breaching => 2,
            Self::Contested => 3,
            Self::Decaying => 4,
            Self::Exposed => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectivePresence {
    pub has_attacker: bool,
    pub has_defender: bool,
}

/// Advance one pair after presence has been sampled for this tick.
///
/// The `frozen` input is deliberately owned by the caller: SWA-40 will wire it
/// to terminal match state without duplicating terminal state in objectives.
pub(crate) fn advance_pair(
    pair: &mut HomeObjectivePair,
    presence: ObjectivePresence,
    frozen: bool,
) -> Option<(ObjectiveState, ObjectiveState)> {
    if frozen {
        return None;
    }
    let previous = pair.state();
    let mut state = previous;
    let mut progress = pair.breach_progress_ticks();
    let mut exposure = pair.exposure_ticks_remaining();

    if state == ObjectiveState::Exposed {
        exposure = exposure.saturating_sub(1);
        if exposure == 0 {
            state = ObjectiveState::Protected;
            progress = 0;
        }
    } else if presence.has_attacker && presence.has_defender {
        state = ObjectiveState::Contested;
        exposure = 0;
    } else if presence.has_attacker {
        progress = (progress + BREACH_PROGRESS_PER_TICK).min(BREACH_DURATION_TICKS);
        exposure = 0;
        state = if progress == BREACH_DURATION_TICKS {
            exposure = EXPOSURE_DURATION_TICKS;
            ObjectiveState::Exposed
        } else {
            ObjectiveState::Breaching
        };
    } else if progress > 0 {
        progress = progress.saturating_sub(BREACH_DECAY_PER_TICK);
        exposure = 0;
        state = if progress == 0 {
            ObjectiveState::Protected
        } else {
            ObjectiveState::Decaying
        };
    } else {
        state = ObjectiveState::Protected;
        exposure = 0;
    }

    pair.set_objective_state(state, progress, exposure);
    (previous != state).then_some((previous, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::initial_home_objectives;

    fn pair() -> HomeObjectivePair {
        initial_home_objectives().1[0]
    }

    #[test]
    fn breach_and_decay_rates_are_exact_integer_tick_rates() {
        assert_eq!(BREACH_DURATION_TICKS, 720);
        assert_eq!(DECAY_DURATION_TICKS, 360);
        assert_eq!(EXPOSURE_DURATION_TICKS, 480);
        assert_eq!(BREACH_DECAY_PER_TICK, 2);
    }

    #[test]
    fn contested_presence_prevents_completion() {
        let mut pair = pair();
        pair.set_objective_state(ObjectiveState::Breaching, BREACH_DURATION_TICKS - 1, 0);
        assert_eq!(
            advance_pair(
                &mut pair,
                ObjectivePresence {
                    has_attacker: true,
                    has_defender: true
                },
                false,
            ),
            Some((ObjectiveState::Breaching, ObjectiveState::Contested))
        );
        assert_eq!(pair.breach_progress_ticks(), BREACH_DURATION_TICKS - 1);
    }

    #[test]
    fn decay_uses_the_fixed_full_scale_rate_and_saturates() {
        let mut pair = pair();
        pair.set_objective_state(ObjectiveState::Breaching, BREACH_DURATION_TICKS, 0);
        for _ in 0..DECAY_DURATION_TICKS {
            advance_pair(&mut pair, ObjectivePresence::default(), false);
        }
        assert_eq!(pair.state(), ObjectiveState::Protected);
        assert_eq!(pair.breach_progress_ticks(), 0);
        assert_eq!(pair.exposure_ticks_remaining(), 0);
    }

    #[test]
    fn exposure_ignores_presence_then_recovers() {
        let mut pair = pair();
        pair.set_objective_state(ObjectiveState::Breaching, BREACH_DURATION_TICKS - 1, 0);
        assert_eq!(
            advance_pair(
                &mut pair,
                ObjectivePresence {
                    has_attacker: true,
                    has_defender: false
                },
                false
            ),
            Some((ObjectiveState::Breaching, ObjectiveState::Exposed))
        );
        for _ in 0..EXPOSURE_DURATION_TICKS - 1 {
            assert_eq!(
                advance_pair(
                    &mut pair,
                    ObjectivePresence {
                        has_attacker: true,
                        has_defender: true
                    },
                    false
                ),
                None
            );
            assert_eq!(pair.state(), ObjectiveState::Exposed);
        }
        assert_eq!(
            advance_pair(
                &mut pair,
                ObjectivePresence {
                    has_attacker: true,
                    has_defender: false
                },
                false
            ),
            Some((ObjectiveState::Exposed, ObjectiveState::Protected))
        );
        assert_eq!(pair.breach_progress_ticks(), 0);
    }

    #[test]
    fn frozen_pair_does_not_change() {
        let mut pair = pair();
        let before = pair;
        assert_eq!(
            advance_pair(
                &mut pair,
                ObjectivePresence {
                    has_attacker: true,
                    has_defender: false
                },
                true
            ),
            None
        );
        assert_eq!(pair, before);
    }
}
