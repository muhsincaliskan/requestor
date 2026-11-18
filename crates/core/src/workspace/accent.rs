use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccentName {
    Blue,
    Teal,
    Green,
    Purple,
    Orange,
    Pink,
    #[default]
    #[serde(other)]
    Theme,
}

impl AccentName {
    pub const ALL: [AccentName; 7] = [
        AccentName::Theme,
        AccentName::Blue,
        AccentName::Teal,
        AccentName::Green,
        AccentName::Purple,
        AccentName::Orange,
        AccentName::Pink,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AccentName::Theme => "Follow the theme",
            AccentName::Blue => "Blue",
            AccentName::Teal => "Teal",
            AccentName::Green => "Green",
            AccentName::Purple => "Purple",
            AccentName::Orange => "Orange",
            AccentName::Pink => "Pink",
        }
    }
}
