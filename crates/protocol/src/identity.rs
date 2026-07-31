use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_DISPLAY_NAME_GRAPHEMES: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for DisplayName {
    type Error = DisplayNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized: String = value.trim().nfc().collect();
        if normalized.is_empty() {
            return Err(DisplayNameError::Required);
        }
        if normalized.chars().any(|character| {
            matches!(
                get_general_category(character),
                GeneralCategory::Control | GeneralCategory::Format
            )
        }) {
            return Err(DisplayNameError::ContainsControlOrFormat);
        }
        if normalized.graphemes(true).count() > MAX_DISPLAY_NAME_GRAPHEMES {
            return Err(DisplayNameError::TooLong);
        }
        Ok(Self(normalized))
    }
}

impl TryFrom<String> for DisplayName {
    type Error = DisplayNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayNameError {
    Required,
    ContainsControlOrFormat,
    TooLong,
}
