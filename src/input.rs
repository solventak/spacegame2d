use crate::simulation::{ShipInput, SimulationCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Thrust,
    TurnLeft,
    TurnRight,
    Reset,
}

#[derive(Default)]
struct MovementKeyState {
    physically_held: bool,
    blocked_until_release: bool,
}

#[derive(Default)]
pub struct InputController {
    thrust: MovementKeyState,
    left: MovementKeyState,
    right: MovementKeyState,
    pending_reset: bool,
}

impl InputController {
    pub fn press(&mut self, key: ControlKey) {
        match key {
            ControlKey::Reset => {
                self.pending_reset = true;
                for state in [&mut self.thrust, &mut self.left, &mut self.right] {
                    if state.physically_held {
                        state.blocked_until_release = true;
                    }
                }
            }
            ControlKey::Thrust => self.thrust.physically_held = true,
            ControlKey::TurnLeft => self.left.physically_held = true,
            ControlKey::TurnRight => self.right.physically_held = true,
        }
    }
    pub fn release(&mut self, key: ControlKey) {
        let state = match key {
            ControlKey::Thrust => Some(&mut self.thrust),
            ControlKey::TurnLeft => Some(&mut self.left),
            ControlKey::TurnRight => Some(&mut self.right),
            ControlKey::Reset => None,
        };
        if let Some(state) = state {
            state.physically_held = false;
            state.blocked_until_release = false;
        }
    }
    pub fn controls(&self) -> ShipInput {
        ShipInput {
            thrust: self.thrust.physically_held && !self.thrust.blocked_until_release,
            turn_left: self.left.physically_held && !self.left.blocked_until_release,
            turn_right: self.right.physically_held && !self.right.blocked_until_release,
        }
    }
    pub fn take_command(&mut self) -> Option<SimulationCommand> {
        self.pending_reset.then(|| {
            self.pending_reset = false;
            SimulationCommand::ResetSimulation
        })
    }
    pub fn suppress_held_movement_until_release(&mut self) {
        for state in [&mut self.thrust, &mut self.left, &mut self.right] {
            if state.physically_held {
                state.blocked_until_release = true;
            }
        }
    }
    pub fn clear_for_focus_loss(&mut self) {
        self.thrust = MovementKeyState::default();
        self.left = MovementKeyState::default();
        self.right = MovementKeyState::default();
        self.pending_reset = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_keys_toggle_controls() {
        let mut c = InputController::default();
        c.press(ControlKey::Thrust);
        assert!(c.controls().thrust);
        c.release(ControlKey::Thrust);
        assert!(!c.controls().thrust);
    }
    #[test]
    fn reset_is_one_shot() {
        let mut c = InputController::default();
        c.press(ControlKey::Reset);
        assert_eq!(c.take_command(), Some(SimulationCommand::ResetSimulation));
        assert_eq!(c.take_command(), None);
    }
    #[test]
    fn reset_blocks_held_controls_until_release_and_repress() {
        let mut c = InputController::default();
        c.press(ControlKey::Thrust);
        c.press(ControlKey::TurnLeft);
        c.press(ControlKey::Reset);
        assert_eq!(c.controls(), ShipInput::default());
        assert_eq!(c.take_command(), Some(SimulationCommand::ResetSimulation));
        c.press(ControlKey::Thrust);
        assert_eq!(c.controls(), ShipInput::default());
        c.release(ControlKey::Thrust);
        c.press(ControlKey::Thrust);
        assert!(c.controls().thrust);
        assert!(!c.controls().turn_left);
    }
    #[test]
    fn focus_loss_clears_controls() {
        let mut c = InputController::default();
        c.press(ControlKey::Thrust);
        c.clear_for_focus_loss();
        assert_eq!(c.controls(), ShipInput::default());
    }
}
