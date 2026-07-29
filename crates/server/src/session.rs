use std::collections::BTreeMap;

use spacegame2d_protocol::{
    MatchTiming, OpponentPresence, PlayerColor, SessionParticipant, SessionSnapshot, Tick,
};

#[derive(Debug)]
pub(crate) struct SessionDelivery {
    pub recipient_slot: u32,
    pub snapshot: SessionSnapshot,
}

#[derive(Debug, Default)]
pub(crate) struct SessionDeparture {
    pub deliveries: Vec<SessionDelivery>,
    pub active_match_ended: bool,
}

#[derive(Debug)]
struct ParticipantState {
    display_name: String,
    presence_revision: u64,
    has_seen_opponent: bool,
}

#[derive(Default)]
pub(crate) struct MatchSession {
    participants: BTreeMap<u32, ParticipantState>,
    match_started_at: Option<Tick>,
}

impl MatchSession {
    pub(crate) fn accept(
        &mut self,
        slot: u32,
        display_name: String,
        tick: Tick,
    ) -> Result<Vec<SessionDelivery>, &'static str> {
        if self.participants.contains_key(&slot) || !(1..=2).contains(&slot) {
            return Err("invalid session slot acceptance");
        }
        self.participants.insert(
            slot,
            ParticipantState {
                display_name,
                presence_revision: 0,
                has_seen_opponent: self.participants.len() == 1,
            },
        );
        if self.participants.len() == 2 {
            self.match_started_at.get_or_insert(tick);
            for participant in self.participants.values_mut() {
                participant.has_seen_opponent = true;
            }
            let existing_slot = if slot == 1 { 2 } else { 1 };
            self.participants
                .get_mut(&existing_slot)
                .expect("existing participant")
                .presence_revision += 1;
            Ok(vec![self.delivery(slot)?, self.delivery(existing_slot)?])
        } else {
            Ok(vec![self.delivery(slot)?])
        }
    }

    pub(crate) fn depart(&mut self, slot: u32) -> SessionDeparture {
        if self.participants.remove(&slot).is_none() {
            return SessionDeparture::default();
        }
        match self.participants.len() {
            0 => {
                let active_match_ended = self.match_started_at.is_some();
                self.match_started_at = None;
                SessionDeparture {
                    deliveries: Vec::new(),
                    active_match_ended,
                }
            }
            1 => {
                let survivor = *self.participants.keys().next().expect("one participant");
                self.participants
                    .get_mut(&survivor)
                    .expect("survivor")
                    .presence_revision += 1;
                SessionDeparture {
                    deliveries: vec![self.delivery(survivor).expect("valid survivor snapshot")],
                    active_match_ended: false,
                }
            }
            _ => unreachable!("two-player session has at most two participants"),
        }
    }

    fn delivery(&self, recipient_slot: u32) -> Result<SessionDelivery, &'static str> {
        let participant = self
            .participants
            .get(&recipient_slot)
            .ok_or("unknown session recipient")?;
        let opponent_presence = match self.participants.len() {
            2 => OpponentPresence::Present,
            1 if participant.has_seen_opponent => OpponentPresence::Disconnected,
            1 => OpponentPresence::Waiting,
            _ => return Err("empty session has no snapshot"),
        };
        let snapshot = SessionSnapshot {
            local_player_slot: recipient_slot,
            participants: self
                .participants
                .iter()
                .map(|(&player_slot, participant)| SessionParticipant {
                    player_slot,
                    display_name: participant.display_name.clone(),
                    color: if player_slot == 1 {
                        PlayerColor::Cyan
                    } else {
                        PlayerColor::Coral
                    },
                })
                .collect(),
            opponent_presence,
            presence_revision: participant.presence_revision,
            match_timing: self
                .match_started_at
                .map(|started_at_tick| MatchTiming::Active { started_at_tick })
                .unwrap_or(MatchTiming::Inactive),
        };
        snapshot
            .validate()
            .map_err(|_| "invalid session snapshot")?;
        Ok(SessionDelivery {
            recipient_slot,
            snapshot,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_anchor(deliveries: &[SessionDelivery]) -> Tick {
        let anchors = deliveries
            .iter()
            .map(|delivery| match delivery.snapshot.match_timing {
                MatchTiming::Active { started_at_tick } => started_at_tick,
                MatchTiming::Inactive => panic!("expected active match timing"),
            })
            .collect::<Vec<_>>();
        assert!(!anchors.is_empty());
        assert!(anchors.iter().all(|anchor| *anchor == anchors[0]));
        anchors[0]
    }

    #[test]
    fn active_match_ends_when_the_last_participant_departs() {
        let mut session = MatchSession::default();
        session.accept(1, "Rook".into(), Tick::from(10)).unwrap();
        let joined = session.accept(2, "Nova".into(), Tick::from(20)).unwrap();
        assert_eq!(active_anchor(&joined), Tick::from(20));

        let survivor = session.depart(2);
        assert_eq!(active_anchor(&survivor.deliveries), Tick::from(20));
        assert_eq!(session.match_started_at, Some(Tick::from(20)));

        let ended = session.depart(1);
        assert!(ended.deliveries.is_empty());
        assert!(ended.active_match_ended);
        assert!(session.participants.is_empty());
        assert_eq!(session.match_started_at, None);
    }

    #[test]
    fn replacement_preserves_the_active_match_start_anchor() {
        let mut session = MatchSession::default();
        session.accept(1, "Rook".into(), Tick::from(10)).unwrap();
        let joined = session.accept(2, "Nova".into(), Tick::from(20)).unwrap();
        let original_anchor = active_anchor(&joined);

        session.depart(2);
        let replacement = session.accept(2, "Echo".into(), Tick::from(140)).unwrap();
        assert_eq!(active_anchor(&replacement), original_anchor);
        assert_eq!(Tick::from(140) - original_anchor, Tick::from(120));
    }

    #[test]
    fn next_two_player_match_receives_a_new_start_anchor() {
        let mut session = MatchSession::default();
        session.accept(1, "Rook".into(), Tick::from(10)).unwrap();
        let first_match = session.accept(2, "Nova".into(), Tick::from(20)).unwrap();
        let first_anchor = active_anchor(&first_match);

        session.depart(2);
        session.depart(1);

        let waiting = session.accept(1, "Rook".into(), Tick::from(40)).unwrap();
        assert!(matches!(
            waiting[0].snapshot.match_timing,
            MatchTiming::Inactive
        ));
        let next_match = session.accept(2, "Nova".into(), Tick::from(50)).unwrap();
        let next_anchor = active_anchor(&next_match);
        assert_eq!(next_anchor, Tick::from(50));
        assert_ne!(next_anchor, first_anchor);
    }

    #[test]
    fn join_depart_and_replacement_preserve_authoritative_state() {
        let mut session = MatchSession::default();
        let first = session.accept(1, "Rook".into(), Tick::from(10)).unwrap();
        assert_eq!(
            first[0].snapshot.opponent_presence,
            OpponentPresence::Waiting
        );
        let joined = session.accept(2, "Nova".into(), Tick::from(20)).unwrap();
        assert!(joined.iter().all(|delivery| matches!(
            delivery.snapshot.match_timing,
            MatchTiming::Active {
                started_at_tick: Tick(20)
            }
        )));
        let departed = session.depart(2);
        assert_eq!(
            departed.deliveries[0].snapshot.opponent_presence,
            OpponentPresence::Disconnected
        );
        let replacement = session.accept(2, "Echo".into(), Tick::from(30)).unwrap();
        assert!(replacement.iter().all(|delivery| {
            delivery
                .snapshot
                .participants
                .iter()
                .all(|participant| participant.display_name != "Nova")
        }));
        assert!(replacement.iter().all(|delivery| matches!(
            delivery.snapshot.match_timing,
            MatchTiming::Active {
                started_at_tick: Tick(20)
            }
        )));
        session.depart(1);
        session.depart(2);
        let waiting = session.accept(1, "Rook".into(), Tick::from(40)).unwrap();
        assert!(matches!(
            waiting[0].snapshot.match_timing,
            MatchTiming::Inactive
        ));
        let next_match = session.accept(2, "Nova".into(), Tick::from(50)).unwrap();
        assert!(next_match.iter().all(|delivery| matches!(
            delivery.snapshot.match_timing,
            MatchTiming::Active {
                started_at_tick: Tick(50)
            }
        )));
    }
}
