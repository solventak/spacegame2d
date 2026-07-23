use crate::flight_control::{FlightController, FlightObservation};
use crate::simulation::{ShipInput, ShipState};
use glam::Vec2;

pub const DEFAULT_ARRIVAL_RADIUS_METERS: f32 = 0.30;
pub const DEFAULT_STOPPED_SPEED_METERS_PER_SECOND: f32 = 0.08;
pub const DEFAULT_THRUST_MIN_HOLD_TICKS: u64 = 12;
pub const DEFAULT_TURN_MIN_HOLD_TICKS: u64 = 9;
pub const DEFAULT_DECISION_COOLDOWN_TICKS: u64 = 18;

#[derive(Clone, Copy, Debug)]
pub struct ActuationConfig {
    pub thrust_min_hold_ticks: u64,
    pub turn_left_min_hold_ticks: u64,
    pub turn_right_min_hold_ticks: u64,
    pub decision_cooldown_ticks: u64,
}
impl Default for ActuationConfig {
    fn default() -> Self {
        Self {
            thrust_min_hold_ticks: DEFAULT_THRUST_MIN_HOLD_TICKS,
            turn_left_min_hold_ticks: DEFAULT_TURN_MIN_HOLD_TICKS,
            turn_right_min_hold_ticks: DEFAULT_TURN_MIN_HOLD_TICKS,
            decision_cooldown_ticks: DEFAULT_DECISION_COOLDOWN_TICKS,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Latch {
    value: bool,
    eligible_at: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CoarseActuator {
    thrust: Latch,
    left: Latch,
    right: Latch,
    next_decision_tick: u64,
}
impl CoarseActuator {
    pub fn apply(&mut self, desired: ShipInput, tick: u64, c: ActuationConfig) -> ShipInput {
        if tick < self.next_decision_tick {
            return self.output();
        }
        let mut changed = false;
        for (latch, wanted, hold) in [
            (&mut self.thrust, desired.thrust, c.thrust_min_hold_ticks),
            (
                &mut self.left,
                desired.turn_left,
                c.turn_left_min_hold_ticks,
            ),
            (
                &mut self.right,
                desired.turn_right,
                c.turn_right_min_hold_ticks,
            ),
        ] {
            if wanted != latch.value && tick >= latch.eligible_at {
                latch.value = wanted;
                latch.eligible_at = tick.saturating_add(hold);
                changed = true;
            }
        }
        if changed {
            self.next_decision_tick = tick.saturating_add(c.decision_cooldown_ticks);
        }
        self.output()
    }
    pub fn allow_immediate_decision(&mut self, tick: u64) {
        self.next_decision_tick = tick;
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn output(&self) -> ShipInput {
        ShipInput {
            thrust: self.thrust.value,
            turn_left: self.left.value,
            turn_right: self.right.value,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct AutopilotConfig {
    pub arrival_radius_meters: f32,
    pub stopped_speed_meters_per_second: f32,
    pub actuation: ActuationConfig,
}
impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            arrival_radius_meters: DEFAULT_ARRIVAL_RADIUS_METERS,
            stopped_speed_meters_per_second: DEFAULT_STOPPED_SPEED_METERS_PER_SECOND,
            actuation: ActuationConfig::default(),
        }
    }
}
pub struct Autopilot {
    controller: Box<dyn FlightController>,
    config: AutopilotConfig,
    destination: Option<Vec2>,
    active: bool,
    actuator: CoarseActuator,
}
impl Autopilot {
    pub fn new(controller: Box<dyn FlightController>, config: AutopilotConfig) -> Self {
        Self {
            controller,
            config,
            destination: None,
            active: false,
            actuator: CoarseActuator::default(),
        }
    }
    pub fn set_destination(&mut self, destination: Vec2, tick: u64) {
        self.destination = Some(destination);
        self.active = true;
        self.actuator.allow_immediate_decision(tick);
    }
    pub fn cancel_and_clear_destination(&mut self) {
        self.destination = None;
        self.active = false;
        self.actuator.clear();
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn destination(&self) -> Option<Vec2> {
        self.destination
    }
    pub fn controls_for_tick(&mut self, tick: u64, ship: &ShipState) -> ShipInput {
        let Some(destination) = self.destination else {
            return ShipInput::default();
        };
        if !self.active {
            return ShipInput::default();
        }
        let desired = self
            .controller
            .desired_input(FlightObservation::from_ship(ship, destination));
        let effective = self.actuator.apply(desired, tick, self.config.actuation);
        if ship.position.distance(destination) <= self.config.arrival_radius_meters
            && ship.velocity.length() <= self.config.stopped_speed_meters_per_second
            && effective == ShipInput::default()
        {
            self.active = false;
        }
        effective
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_control::{FlightController, FlightObservation};
    #[derive(Debug)]
    struct Script(ShipInput);
    impl FlightController for Script {
        fn name(&self) -> &'static str {
            "script"
        }
        fn desired_input(&self, _: FlightObservation) -> ShipInput {
            self.0
        }
    }
    fn ap(input: ShipInput) -> Autopilot {
        Autopilot::new(
            Box::new(Script(input)),
            AutopilotConfig {
                actuation: ActuationConfig {
                    thrust_min_hold_ticks: 3,
                    turn_left_min_hold_ticks: 2,
                    turn_right_min_hold_ticks: 2,
                    decision_cooldown_ticks: 4,
                },
                ..Default::default()
            },
        )
    }
    #[test]
    fn first_decision_is_immediate() {
        let mut a = ap(ShipInput {
            thrust: true,
            ..Default::default()
        });
        a.set_destination(Vec2::X, 0);
        assert!(a.controls_for_tick(0, &ShipState::default()).thrust);
    }
    #[test]
    fn holds_and_cooldown_are_enforced() {
        let mut a = ap(ShipInput {
            thrust: true,
            ..Default::default()
        });
        a.set_destination(Vec2::X, 0);
        assert!(a.controls_for_tick(0, &ShipState::default()).thrust);
        assert!(a.controls_for_tick(1, &ShipState::default()).thrust);
    }
    #[test]
    fn destination_replacement_keeps_marker() {
        let mut a = ap(ShipInput::default());
        a.set_destination(Vec2::X, 0);
        a.set_destination(Vec2::Y, 1);
        assert_eq!(a.destination(), Some(Vec2::Y));
    }
    #[test]
    fn arrival_requires_position_and_speed() {
        let mut a = ap(ShipInput::default());
        a.set_destination(Vec2::ZERO, 0);
        let s = ShipState {
            velocity: Vec2::X,
            ..Default::default()
        };
        assert!(a.controls_for_tick(0, &s) == ShipInput::default() && a.is_active());
        let s = ShipState::default();
        assert_eq!(a.controls_for_tick(1, &s), ShipInput::default());
        assert!(!a.is_active());
    }
    #[test]
    fn cancel_clears_destination() {
        let mut a = ap(ShipInput::default());
        a.set_destination(Vec2::X, 0);
        a.cancel_and_clear_destination();
        assert!(!a.is_active());
        assert_eq!(a.destination(), None);
    }
}
