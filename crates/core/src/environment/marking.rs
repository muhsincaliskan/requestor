use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Environment;

const COLOR_KEY: &str = "requestorColor";
const PROTECTED_KEY: &str = "requestorProtected";
const PRODUCTION_NAMES: [&str; 3] = ["prod", "production", "live"];
const WORD_SEPARATORS: [char; 4] = [' ', '-', '_', '.'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentColor {
    Gray,
    Green,
    Blue,
    Purple,
    Amber,
    Red,
}

impl EnvironmentColor {
    pub const ALL: [EnvironmentColor; 6] = [
        EnvironmentColor::Gray,
        EnvironmentColor::Green,
        EnvironmentColor::Blue,
        EnvironmentColor::Purple,
        EnvironmentColor::Amber,
        EnvironmentColor::Red,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EnvironmentColor::Gray => "Gray",
            EnvironmentColor::Green => "Green",
            EnvironmentColor::Blue => "Blue",
            EnvironmentColor::Purple => "Purple",
            EnvironmentColor::Amber => "Amber",
            EnvironmentColor::Red => "Red",
        }
    }
}

impl Environment {
    pub fn color(&self) -> Option<EnvironmentColor> {
        self.chosen_color()
            .or_else(|| self.is_protected().then_some(EnvironmentColor::Red))
    }

    pub fn chosen_color(&self) -> Option<EnvironmentColor> {
        serde_json::from_value(self.extra.get(COLOR_KEY)?.clone()).ok()
    }

    pub fn set_color(&mut self, color: Option<EnvironmentColor>) {
        match color.and_then(|color| serde_json::to_value(color).ok()) {
            Some(value) => self.extra.insert(COLOR_KEY.to_string(), value),
            None => self.extra.remove(COLOR_KEY),
        };
    }

    pub fn is_protected(&self) -> bool {
        match self.extra.get(PROTECTED_KEY) {
            Some(Value::Bool(protected)) => *protected,
            _ => self.is_named_like_production(),
        }
    }

    pub fn set_protected(&mut self, protected: bool) {
        if protected == self.is_named_like_production() {
            self.extra.remove(PROTECTED_KEY);
        } else {
            self.extra
                .insert(PROTECTED_KEY.to_string(), Value::Bool(protected));
        }
    }

    fn is_named_like_production(&self) -> bool {
        self.name
            .to_lowercase()
            .split(WORD_SEPARATORS)
            .any(|word| PRODUCTION_NAMES.contains(&word))
    }
}
