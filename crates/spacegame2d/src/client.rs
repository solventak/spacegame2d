use std::collections::BTreeSet;

use glam::Vec2;
use spacegame2d_protocol::{CommandData, CommandRequest};
use spacegame2d_simulation::{PlayerId, UnitId, World};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClientControl {
    ClickSelect(Vec2),
    DragSelect { anchor: Vec2, cursor: Vec2 },
    CommitDrag,
    CancelDrag,
    ClearSelection,
    SetDestination(Vec2),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ClientEffect {
    Send(CommandRequest),
}

#[derive(Default)]
pub(crate) struct Client {
    selected: BTreeSet<UnitId>,
    preview: BTreeSet<UnitId>,
    drag: Option<(Vec2, Vec2)>,
    next_sequence: u32,
}

impl Client {
    pub(crate) fn new() -> Self {
        Self {
            next_sequence: 1,
            ..Self::default()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn selected(&self) -> &BTreeSet<UnitId> {
        &self.selected
    }
    #[allow(dead_code)]
    pub(crate) fn preview(&self) -> &BTreeSet<UnitId> {
        &self.preview
    }
    #[allow(dead_code)]
    pub(crate) fn drag(&self) -> Option<(Vec2, Vec2)> {
        self.drag
    }
    pub(crate) fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub(crate) fn clear(&mut self) {
        self.selected.clear();
        self.preview.clear();
        self.drag = None;
    }

    pub(crate) fn world_advanced(&mut self, world: &World, player: PlayerId) {
        self.selected.retain(|id| {
            world
                .unit(*id)
                .is_some_and(|unit| unit.owner == Some(player))
        });
        if self.drag.is_some() {
            self.refresh_preview(world, player);
        }
    }

    pub(crate) fn handle(
        &mut self,
        control: ClientControl,
        world: &World,
        player: PlayerId,
    ) -> Option<ClientEffect> {
        match control {
            ClientControl::ClickSelect(point) => {
                self.preview.clear();
                self.drag = None;
                self.selected = world
                    .units
                    .iter()
                    .filter(|unit| unit.owner == Some(player))
                    .filter(|unit| {
                        unit.positioned_hitbox().center().distance_squared(point)
                            <= match unit.hitbox().shape() {
                                spacegame2d_simulation::HitboxShape::Circle(circle) => {
                                    circle.radius_meters() * circle.radius_meters()
                                }
                            }
                    })
                    .min_by_key(|unit| {
                        (
                            unit.positioned_hitbox()
                                .center()
                                .distance_squared(point)
                                .to_bits(),
                            unit.id,
                        )
                    })
                    .map(|unit| [unit.id].into_iter().collect())
                    .unwrap_or_default();
                None
            }
            ClientControl::DragSelect { anchor, cursor } => {
                self.drag = Some((anchor, cursor));
                self.refresh_preview(world, player);
                None
            }
            ClientControl::CommitDrag => {
                if self.drag.take().is_some() {
                    self.selected = std::mem::take(&mut self.preview);
                }
                None
            }
            ClientControl::CancelDrag => {
                self.preview.clear();
                self.drag = None;
                None
            }
            ClientControl::ClearSelection => {
                if !self.dragging() {
                    self.selected.clear();
                }
                None
            }
            ClientControl::SetDestination(destination) => {
                if self.dragging() || self.selected.is_empty() {
                    return None;
                }
                let sequence = self.next_sequence;
                self.next_sequence = self.next_sequence.saturating_add(1);
                Some(ClientEffect::Send(CommandRequest {
                    sequence,
                    command: CommandData::SetDestination {
                        destination: [destination.x.to_bits(), destination.y.to_bits()],
                        target_unit_ids: self.selected.iter().map(|id| id.0).collect(),
                    },
                }))
            }
        }
    }

    fn refresh_preview(&mut self, world: &World, player: PlayerId) {
        let Some((a, b)) = self.drag else {
            return;
        };
        let min = a.min(b);
        let max = a.max(b);
        self.preview = world
            .units
            .iter()
            .filter(|unit| unit.owner == Some(player))
            .filter(|unit| {
                let center = unit.positioned_hitbox().center().clamp(min, max);
                let radius = match unit.hitbox().shape() {
                    spacegame2d_simulation::HitboxShape::Circle(circle) => circle.radius_meters(),
                };
                unit.state.position.distance_squared(center) <= radius * radius
            })
            .map(|unit| unit.id)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_simulation::World;

    #[test]
    fn drag_selection_replaces_committed_selection_and_captures_sorted_ids() {
        let mut world = World::demo();
        world.assign_player_fleet(PlayerId(1));
        let first = world.units[0].state.position;
        let second = world.units[1].state.position;
        let mut client = Client::new();
        client.handle(
            ClientControl::DragSelect {
                anchor: first - Vec2::splat(1.0),
                cursor: second + Vec2::splat(1.0),
            },
            &world,
            PlayerId(1),
        );
        client.handle(ClientControl::CommitDrag, &world, PlayerId(1));
        assert!(client.selected().contains(&world.units[0].id));
        let effect = client.handle(ClientControl::SetDestination(Vec2::X), &world, PlayerId(1));
        let Some(ClientEffect::Send(request)) = effect else {
            panic!("expected command")
        };
        let CommandData::SetDestination {
            target_unit_ids, ..
        } = request.command
        else {
            panic!("expected destination")
        };
        assert!(target_unit_ids.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn cancellation_preserves_committed_selection() {
        let mut world = World::demo();
        world.assign_player_fleet(PlayerId(1));
        let point = world.units[0].state.position;
        let mut client = Client::new();
        client.handle(ClientControl::ClickSelect(point), &world, PlayerId(1));
        client.handle(
            ClientControl::DragSelect {
                anchor: point,
                cursor: point + Vec2::splat(3.0),
            },
            &world,
            PlayerId(1),
        );
        client.handle(ClientControl::CancelDrag, &world, PlayerId(1));
        assert_eq!(client.selected(), &BTreeSet::from([world.units[0].id]));
        assert!(client.preview().is_empty());
    }
}
