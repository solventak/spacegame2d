use spacegame2d_protocol::{
    OpponentPresence as ProtocolOpponentPresence, PlayerColor as ProtocolPlayerColor,
};
use spacegame2d_simulation::simulation::SIMULATION_HZ;
use spacegame2d_ui_protocol::{
    MatchClockHudModel, MatchParticipantHudModel, MatchSessionResetReason, MatchSessionState,
    OpponentPresence, PlayerColor,
};

use crate::network::NetworkSession;

pub struct MatchSessionPresenter {
    sequence: u64,
    last_opponent: Option<MatchParticipantHudModel>,
    last_elapsed_whole_seconds: Option<u64>,
    state: MatchSessionState,
}

impl Default for MatchSessionPresenter {
    fn default() -> Self {
        Self {
            sequence: 0,
            last_opponent: None,
            last_elapsed_whole_seconds: None,
            state: MatchSessionState::Reset {
                sequence: 0,
                reason: MatchSessionResetReason::Startup,
            },
        }
    }
}

impl MatchSessionPresenter {
    pub fn state(&self) -> &MatchSessionState {
        &self.state
    }

    pub fn reset(&mut self, reason: MatchSessionResetReason) -> &MatchSessionState {
        self.sequence = self.sequence.saturating_add(1);
        self.last_opponent = None;
        self.last_elapsed_whole_seconds = None;
        self.state = MatchSessionState::Reset {
            sequence: self.sequence,
            reason,
        };
        &self.state
    }

    pub fn update(&mut self, session: &NetworkSession) -> Result<&MatchSessionState, &'static str> {
        let snapshot = session.session_snapshot();
        let local = participant(snapshot.local_participant())?;
        let presence = presence(snapshot.opponent_presence);
        let next = match session.match_started_at() {
            None => {
                if snapshot.opponent_presence != ProtocolOpponentPresence::Waiting {
                    return Err("inactive match timing must be waiting");
                }
                self.last_opponent = None;
                self.last_elapsed_whole_seconds = None;
                MatchSessionState::Waiting {
                    sequence: self.next_sequence(),
                    local_player: local,
                    opponent_presence: presence,
                    presence_revision: snapshot.presence_revision,
                }
            }
            Some(started_at_tick) => {
                let current_opponent = snapshot
                    .participants
                    .iter()
                    .find(|participant| participant.player_slot != snapshot.local_player_slot)
                    .map(participant)
                    .transpose()?;
                if let Some(opponent) = current_opponent.as_ref() {
                    self.last_opponent = Some(opponent.clone());
                }
                let opponent = self
                    .last_opponent
                    .clone()
                    .ok_or("active match is missing opponent identity")?;
                if snapshot.opponent_presence == ProtocolOpponentPresence::Waiting {
                    return Err("active match timing cannot be waiting");
                }
                let elapsed = session
                    .elapsed_match_seconds()
                    .ok_or("active match is missing elapsed time")?;
                self.last_elapsed_whole_seconds = Some(elapsed);
                MatchSessionState::Active {
                    sequence: self.next_sequence(),
                    local_player: local,
                    opponent_player: opponent,
                    opponent_presence: presence,
                    presence_revision: snapshot.presence_revision,
                    clock: MatchClockHudModel {
                        started_at_tick: started_at_tick.0,
                        current_tick: session.local_tick().0,
                        ticks_per_second: SIMULATION_HZ,
                        elapsed_whole_seconds: elapsed,
                    },
                }
            }
        };
        self.state = next;
        Ok(&self.state)
    }

    pub fn update_if_elapsed_changed(
        &mut self,
        session: &NetworkSession,
    ) -> Result<Option<&MatchSessionState>, &'static str> {
        let Some(elapsed) = session.elapsed_match_seconds() else {
            return Ok(None);
        };
        if self.last_elapsed_whole_seconds == Some(elapsed) {
            return Ok(None);
        }
        self.update(session).map(Some)
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.sequence
    }
}

fn participant(
    participant: &spacegame2d_protocol::SessionParticipant,
) -> Result<MatchParticipantHudModel, &'static str> {
    let (color, color_hex) = match participant.color {
        ProtocolPlayerColor::Cyan if participant.player_slot == 1 => (PlayerColor::Cyan, "#22CFE8"),
        ProtocolPlayerColor::Coral if participant.player_slot == 2 => {
            (PlayerColor::Coral, "#FF6A47")
        }
        _ => return Err("participant color does not match slot"),
    };
    Ok(MatchParticipantHudModel {
        player_slot: participant.player_slot,
        display_name: participant.display_name.clone(),
        color,
        color_hex: color_hex.into(),
    })
}

fn presence(value: ProtocolOpponentPresence) -> OpponentPresence {
    match value {
        ProtocolOpponentPresence::Waiting => OpponentPresence::Waiting,
        ProtocolOpponentPresence::Present => OpponentPresence::Present,
        ProtocolOpponentPresence::Disconnected => OpponentPresence::Disconnected,
    }
}
