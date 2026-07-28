use serde::Serialize;

pub const PLAYER_ONE_COLOR: [f32; 4] = [0.0, 0.9, 1.0, 1.0];
pub const PLAYER_TWO_COLOR: [f32; 4] = [1.0, 0.35, 0.2, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayerColor {
    Cyan,
    Coral,
}

impl PlayerColor {
    pub fn for_slot(player_slot: u32) -> Self {
        if player_slot == 2 {
            Self::Coral
        } else {
            Self::Cyan
        }
    }

    pub const fn render_rgba(self) -> [f32; 4] {
        match self {
            Self::Cyan => PLAYER_ONE_COLOR,
            Self::Coral => PLAYER_TWO_COLOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_colours_preserve_the_native_mapping() {
        assert_eq!(PlayerColor::for_slot(1), PlayerColor::Cyan);
        assert_eq!(PlayerColor::for_slot(2), PlayerColor::Coral);
        assert_eq!(PlayerColor::for_slot(0), PlayerColor::Cyan);
        assert_eq!(PlayerColor::Cyan.render_rgba(), PLAYER_ONE_COLOR);
        assert_eq!(PlayerColor::Coral.render_rgba(), PLAYER_TWO_COLOR);
    }
}
