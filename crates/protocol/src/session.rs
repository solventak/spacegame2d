use std::io;

use crate::error::invalid;
use crate::identity::DisplayName;
use crate::tick::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerColor {
    Cyan,
    Coral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpponentPresence {
    Waiting,
    Present,
    Disconnected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchTiming {
    Inactive,
    Active { started_at_tick: Tick },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionParticipant {
    pub player_slot: u32,
    pub display_name: String,
    pub color: PlayerColor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub local_player_slot: u32,
    pub participants: Vec<SessionParticipant>,
    pub opponent_presence: OpponentPresence,
    pub presence_revision: u64,
    pub match_timing: MatchTiming,
}

impl SessionSnapshot {
    pub fn validate(&self) -> io::Result<()> {
        if !(1..=2).contains(&self.local_player_slot)
            || self.participants.is_empty()
            || self.participants.len() > 2
        {
            return Err(invalid("invalid session roster"));
        }
        let mut previous = 0;
        let mut local_count = 0;
        for participant in &self.participants {
            if !(1..=2).contains(&participant.player_slot) || participant.player_slot <= previous {
                return Err(invalid("invalid session participant slots"));
            }
            previous = participant.player_slot;
            if DisplayName::try_from(participant.display_name.as_str())
                .map_err(|_| invalid("invalid session display name"))?
                .as_str()
                != participant.display_name
            {
                return Err(invalid("noncanonical session display name"));
            }
            let expected = if participant.player_slot == 1 {
                PlayerColor::Cyan
            } else {
                PlayerColor::Coral
            };
            if participant.color != expected {
                return Err(invalid("invalid session participant color"));
            }
            local_count += usize::from(participant.player_slot == self.local_player_slot);
        }
        if local_count != 1 {
            return Err(invalid("session local player missing"));
        }
        let active = matches!(self.match_timing, MatchTiming::Active { .. });
        match self.opponent_presence {
            OpponentPresence::Waiting if self.participants.len() == 1 && !active => Ok(()),
            OpponentPresence::Present if self.participants.len() == 2 && active => Ok(()),
            OpponentPresence::Disconnected if self.participants.len() == 1 && active => Ok(()),
            _ => Err(invalid("inconsistent session state")),
        }
    }

    pub fn local_participant(&self) -> &SessionParticipant {
        self.participants
            .iter()
            .find(|participant| participant.player_slot == self.local_player_slot)
            .expect("validated session snapshot has local participant")
    }
}
