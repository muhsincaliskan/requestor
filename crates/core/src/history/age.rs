use std::fmt;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const JUST_NOW: &str = "just now";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Age {
    seconds: u64,
}

impl Age {
    pub fn between(earlier: u64, later: u64) -> Self {
        Age {
            seconds: later.saturating_sub(earlier),
        }
    }
}

impl fmt::Display for Age {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.seconds;
        if seconds < MINUTE {
            formatter.write_str(JUST_NOW)
        } else if seconds < HOUR {
            write!(formatter, "{} min ago", seconds / MINUTE)
        } else if seconds < DAY {
            write!(formatter, "{} h ago", seconds / HOUR)
        } else {
            write!(formatter, "{} d ago", seconds / DAY)
        }
    }
}
