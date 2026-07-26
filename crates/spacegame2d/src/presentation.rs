use std::time::{Duration, Instant};

use glam::Vec2;
use spacegame2d_protocol::{CommandRejected, CommandRejectionReason};
use spacegame2d_simulation::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkerStatus {
    Pending,
    Confirmed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DestinationMarker {
    pub(crate) position: Vec2,
    pub(crate) status: MarkerStatus,
}

#[derive(Debug, Default)]
pub(crate) struct DestinationPresentation {
    pending: Option<(u32, Vec2)>,
    confirmed: Option<Vec2>,
    rejection: Option<(String, Instant)>,
}

impl DestinationPresentation {
    pub(crate) fn begin(&mut self, sequence: u32, destination: Vec2) {
        self.pending = Some((sequence, destination));
        self.rejection = None;
    }

    pub(crate) fn authoritative(
        &mut self,
        local_slot: u32,
        command: &spacegame2d_protocol::AuthoritativeCommand,
    ) {
        if command.command == spacegame2d_protocol::CommandData::ResetSimulation {
            self.clear();
            return;
        }
        if command.player_slot != local_slot {
            return;
        }
        let spacegame2d_protocol::CommandData::SetDestination { destination } = &command.command
        else {
            return;
        };
        let point = Vec2::new(
            f32::from_bits(destination[0]),
            f32::from_bits(destination[1]),
        );
        self.confirmed = Some(point);
        if self
            .pending
            .is_some_and(|(sequence, _)| sequence == command.sequence)
        {
            self.pending = None;
        }
    }

    pub(crate) fn rejected(&mut self, rejection: &CommandRejected, now: Instant) {
        if self
            .pending
            .is_some_and(|(sequence, _)| sequence == rejection.sequence)
        {
            self.pending = None;
        }
        self.rejection = Some((
            rejection_message(rejection.reason).to_owned(),
            now + Duration::from_secs(2),
        ));
    }

    pub(crate) fn clear(&mut self) {
        self.pending = None;
        self.confirmed = None;
        self.rejection = None;
    }

    pub(crate) fn marker(&self, world: &World) -> Option<DestinationMarker> {
        self.pending
            .map(|(_, position)| DestinationMarker {
                position: world.project_destination(position),
                status: MarkerStatus::Pending,
            })
            .or_else(|| {
                self.confirmed.map(|position| DestinationMarker {
                    position: world.project_destination(position),
                    status: MarkerStatus::Confirmed,
                })
            })
    }

    pub(crate) fn rejection_text(&mut self, now: Instant) -> Option<&str> {
        if self
            .rejection
            .as_ref()
            .is_some_and(|(_, deadline)| *deadline <= now)
        {
            self.rejection = None;
        }
        self.rejection.as_ref().map(|(message, _)| message.as_str())
    }
}

fn rejection_message(reason: CommandRejectionReason) -> &'static str {
    match reason {
        CommandRejectionReason::InvalidPlayer => "Command rejected: invalid player",
        CommandRejectionReason::UnauthorizedFleet => "Command rejected: unauthorized fleet",
        CommandRejectionReason::NonFiniteDestination => "Command rejected: invalid destination",
        CommandRejectionReason::DestinationOutsideArena => "Command rejected: outside arena",
        CommandRejectionReason::InvalidCommand => "Command rejected: invalid command",
    }
}
