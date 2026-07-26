//! Client-only combat effect state derived from authoritative simulation events.

use std::time::{Duration, Instant};

use glam::Vec2;
use spacegame2d_simulation::SimulationEvent;

pub(crate) const TRACER_LIFETIME: Duration = Duration::from_millis(60);
pub(crate) const FLASH_LIFETIME: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TracerEffect {
    pub(crate) start: Vec2,
    pub(crate) end: Vec2,
    started_at: Instant,
    expires_at: Instant,
}

impl TracerEffect {
    pub(crate) fn opacity(self, now: Instant) -> f32 {
        remaining_fraction(self.started_at, self.expires_at, now)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HitFlashEffect {
    pub(crate) position: Vec2,
    started_at: Instant,
    expires_at: Instant,
}

impl HitFlashEffect {
    pub(crate) fn opacity(self, now: Instant) -> f32 {
        remaining_fraction(self.started_at, self.expires_at, now)
    }

    pub(crate) fn scale(self, now: Instant) -> f32 {
        0.75 + (1.0 - self.opacity(now)) * 0.25
    }
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct CombatPresentation {
    tracers: Vec<TracerEffect>,
    flashes: Vec<HitFlashEffect>,
}

impl CombatPresentation {
    pub(crate) fn ingest(&mut self, now: Instant, events: &[SimulationEvent]) {
        for event in events {
            let SimulationEvent::ShotFired {
                muzzle_origin,
                impact_position,
                impact_entity,
                ..
            } = *event
            else {
                continue;
            };
            self.tracers.push(TracerEffect {
                start: muzzle_origin,
                end: impact_position,
                started_at: now,
                expires_at: now + TRACER_LIFETIME,
            });
            if impact_entity.is_some() {
                self.flashes.push(HitFlashEffect {
                    position: impact_position,
                    started_at: now,
                    expires_at: now + FLASH_LIFETIME,
                });
            }
        }
    }

    pub(crate) fn retain_active(&mut self, now: Instant) {
        self.tracers.retain(|effect| effect.expires_at > now);
        self.flashes.retain(|effect| effect.expires_at > now);
    }

    pub(crate) fn clear(&mut self) {
        self.tracers.clear();
        self.flashes.clear();
    }

    pub(crate) fn tracers(&self) -> &[TracerEffect] {
        &self.tracers
    }
    pub(crate) fn flashes(&self) -> &[HitFlashEffect] {
        &self.flashes
    }
}

fn remaining_fraction(started_at: Instant, expires_at: Instant, now: Instant) -> f32 {
    let duration = expires_at.duration_since(started_at).as_secs_f32();
    if duration == 0.0 || now >= expires_at {
        return 0.0;
    }
    1.0 - now.duration_since(started_at).as_secs_f32() / duration
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacegame2d_protocol::Tick;
    use spacegame2d_simulation::{ImpactEntityId, StaticStructureId, UnitId};

    fn shot(impact_entity: Option<ImpactEntityId>) -> SimulationEvent {
        SimulationEvent::ShotFired {
            tick: Tick::new(1),
            shooter_id: UnitId(1),
            muzzle_origin: Vec2::new(1.0, 2.0),
            ray_endpoint: Vec2::new(1.0, 14.0),
            impact_position: Vec2::new(1.0, 14.0),
            impact_entity,
        }
    }

    #[test]
    fn miss_creates_only_a_tracer_and_expires_at_the_lifetime() {
        let now = Instant::now();
        let mut presentation = CombatPresentation::default();
        presentation.ingest(now, &[shot(None)]);
        assert_eq!(presentation.tracers.len(), 1);
        assert!(presentation.flashes.is_empty());
        presentation.retain_active(now + TRACER_LIFETIME - Duration::from_nanos(1));
        assert_eq!(presentation.tracers.len(), 1);
        presentation.retain_active(now + TRACER_LIFETIME);
        assert!(presentation.tracers.is_empty());
    }

    #[test]
    fn impact_uses_the_authoritative_boundary_position() {
        let now = Instant::now();
        let mut presentation = CombatPresentation::default();
        presentation.ingest(now, &[shot(Some(ImpactEntityId::Unit(UnitId(9))))]);
        assert_eq!(presentation.flashes()[0].position, Vec2::new(1.0, 14.0));
    }

    #[test]
    fn same_tick_events_are_not_coalesced_and_clear_removes_them() {
        let now = Instant::now();
        let mut presentation = CombatPresentation::default();
        presentation.ingest(
            now,
            &[
                shot(Some(ImpactEntityId::Unit(UnitId(3)))),
                shot(Some(ImpactEntityId::StaticStructure(StaticStructureId(4)))),
            ],
        );
        assert_eq!(presentation.tracers.len(), 2);
        assert_eq!(presentation.flashes.len(), 2);
        presentation.clear();
        assert!(presentation.tracers.is_empty() && presentation.flashes.is_empty());
    }
}
