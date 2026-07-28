use std::collections::BTreeMap;

use spacegame2d_protocol::{
    MatchTiming, OpponentPresence, PlayerColor, SessionParticipant, SessionSnapshot, Tick,
};

#[derive(Debug)]
pub(crate) struct SessionDelivery {
    pub recipient_slot: u32,
    pub snapshot: SessionSnapshot,
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

    pub(crate) fn depart(&mut self, slot: u32) -> Vec<SessionDelivery> {
        if self.participants.remove(&slot).is_none() {
            return Vec::new();
        }
        match self.participants.len() {
            0 => {
                self.match_started_at = None;
                Vec::new()
            }
            1 => {
                let survivor = *self.participants.keys().next().expect("one participant");
                self.participants
                    .get_mut(&survivor)
                    .expect("survivor")
                    .presence_revision += 1;
                vec![self.delivery(survivor).expect("valid survivor snapshot")]
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
            departed[0].snapshot.opponent_presence,
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
