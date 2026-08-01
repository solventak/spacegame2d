use std::time::{Duration, Instant};

use glam::Vec2;
use spacegame2d_protocol::{CommandRejected, CommandRejectionReason};
use spacegame2d_simulation::{UnitId, World};

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

#[derive(Debug)]
struct RouteMarker {
    position: Vec2,
    targets: Vec<UnitId>,
    activated: bool,
}

#[derive(Debug, Default)]
pub(crate) struct DestinationPresentation {
    pending: Vec<(u32, RouteMarker)>,
    confirmed: Vec<RouteMarker>,
    rejection: Option<(String, Instant)>,
}

impl DestinationPresentation {
    pub(crate) fn begin(&mut self, sequence: u32, destination: Vec2, target_unit_ids: Vec<u32>) {
        self.pending.push((
            sequence,
            RouteMarker {
                position: destination,
                targets: target_unit_ids.into_iter().map(UnitId).collect(),
                activated: false,
            },
        ));
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
        let spacegame2d_protocol::CommandData::SetDestination {
            destination,
            target_unit_ids,
        } = &command.command
        else {
            return;
        };
        let point = Vec2::new(
            f32::from_bits(destination[0]),
            f32::from_bits(destination[1]),
        );
        self.confirmed.push(RouteMarker {
            position: point,
            targets: target_unit_ids.iter().copied().map(UnitId).collect(),
            activated: false,
        });
        self.pending
            .retain(|(sequence, _)| *sequence != command.sequence);
    }

    pub(crate) fn rejected(&mut self, rejection: &CommandRejected, now: Instant) {
        self.pending
            .retain(|(sequence, _)| *sequence != rejection.sequence);
        self.rejection = Some((
            rejection_message(rejection.reason).to_owned(),
            now + Duration::from_secs(2),
        ));
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
        self.confirmed.clear();
        self.rejection = None;
    }

    pub(crate) fn markers(&mut self, world: &World) -> Vec<DestinationMarker> {
        self.confirmed.retain_mut(|marker| {
            let destination = world.project_destination(marker.position);
            let routing = marker.targets.iter().any(|id| {
                world.unit(*id).is_some_and(|unit| {
                    unit.autopilot.is_active() && unit.autopilot.destination() == Some(destination)
                })
            });
            marker.activated |= routing;
            !marker.activated || routing
        });
        self.pending
            .iter()
            .map(|(_, marker)| DestinationMarker {
                position: world.project_destination(marker.position),
                status: MarkerStatus::Pending,
            })
            .chain(self.confirmed.iter().map(|marker| DestinationMarker {
                position: world.project_destination(marker.position),
                status: MarkerStatus::Confirmed,
            }))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn marker(&mut self, world: &World) -> Option<DestinationMarker> {
        self.markers(world).into_iter().next()
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
