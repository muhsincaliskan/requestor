use std::time::{SystemTime, UNIX_EPOCH};

use crate::identity::{random_u64, random_uuid};

const GUID_PLACEHOLDERS: [&str; 2] = ["$guid", "$randomUUID"];
const TIMESTAMP_PLACEHOLDER: &str = "$timestamp";
const RANDOM_INT_PLACEHOLDER: &str = "$randomInt";
const POSTMAN_RANDOM_INT_MAX: u64 = 1000;
const SECONDS_PER_DAY: u64 = 86_400;
pub(crate) const MILLIS_PER_SECOND: u64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dynamic {
    Uuid,
    Timestamp,
    TimestampMillis,
    RandomNumber { min: i64, max: i64 },
    NowIso,
}

impl Dynamic {
    pub fn from_placeholder(name: &str) -> Option<Dynamic> {
        match name {
            _ if GUID_PLACEHOLDERS.contains(&name) => Some(Dynamic::Uuid),
            TIMESTAMP_PLACEHOLDER => Some(Dynamic::Timestamp),
            RANDOM_INT_PLACEHOLDER => Some(Dynamic::RandomNumber {
                min: 0,
                max: POSTMAN_RANDOM_INT_MAX as i64,
            }),
            _ => None,
        }
    }

    pub fn generate(self) -> String {
        match self {
            Dynamic::Uuid => random_uuid(),
            Dynamic::Timestamp => elapsed().as_secs().to_string(),
            Dynamic::TimestampMillis => elapsed().as_millis().to_string(),
            Dynamic::RandomNumber { min, max } => random_between(min, max).to_string(),
            Dynamic::NowIso => iso_now(),
        }
    }
}

fn elapsed() -> std::time::Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

fn random_between(min: i64, max: i64) -> i64 {
    let (low, high) = if min <= max { (min, max) } else { (max, min) };
    let span = high.abs_diff(low).saturating_add(1);
    low.saturating_add_unsigned(random_u64() % span)
}

fn iso_now() -> String {
    let now = elapsed();
    let seconds = now.as_secs();
    let (year, month, day) = civil_date(seconds / SECONDS_PER_DAY);
    let time = seconds % SECONDS_PER_DAY;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        time / 3600,
        time / 60 % 60,
        time % 60,
        now.as_millis() % u128::from(MILLIS_PER_SECOND)
    )
}

fn civil_date(days: u64) -> (i64, u64, u64) {
    let shifted = days as i64 + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era as i64 + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{civil_date, random_between, Dynamic};

    #[test]
    fn dates_follow_the_civil_calendar() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(19_723), (2024, 1, 1));
        assert_eq!(civil_date(20_416), (2025, 11, 24));
    }

    #[test]
    fn random_numbers_stay_inside_the_range() {
        for _ in 0..50 {
            assert!((3..=5).contains(&random_between(3, 5)));
            assert!((3..=5).contains(&random_between(5, 3)));
        }
        assert_eq!(random_between(7, 7), 7);
    }

    #[test]
    fn postman_placeholders_map_to_dynamic_values() {
        assert_eq!(Dynamic::from_placeholder("$guid"), Some(Dynamic::Uuid));
        assert_eq!(
            Dynamic::from_placeholder("$timestamp"),
            Some(Dynamic::Timestamp)
        );
        assert!(Dynamic::from_placeholder("$nope").is_none());
        let iso = Dynamic::NowIso.generate();
        assert_eq!(iso.len(), 24);
        assert!(iso.ends_with('Z'));
    }
}
