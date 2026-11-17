use requestor_core::settings::RequestSettings;

use crate::cursor::Cursor;
use crate::error::CliError;

pub const TIMEOUT_FLAG: &str = "--timeout-ms";
pub const INSECURE_FLAG: &str = "--insecure";
pub const NO_FOLLOW_FLAG: &str = "--no-follow-redirects";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct NetworkOptions {
    pub timeout_ms: Option<u64>,
    pub insecure: bool,
    pub no_follow_redirects: bool,
}

impl NetworkOptions {
    pub fn accept(&mut self, flag: &str, cursor: &mut Cursor) -> Result<bool, CliError> {
        match flag {
            TIMEOUT_FLAG => self.timeout_ms = Some(cursor.number(flag)?),
            INSECURE_FLAG => self.insecure = true,
            NO_FOLLOW_FLAG => self.no_follow_redirects = true,
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn to_settings(&self) -> RequestSettings {
        let mut settings = RequestSettings::default();
        if let Some(timeout_ms) = self.timeout_ms {
            settings.request_timeout_ms = timeout_ms;
        }
        settings.verify_ssl = !self.insecure;
        settings.follow_redirects = !self.no_follow_redirects;
        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_map_onto_settings() {
        let options = NetworkOptions {
            timeout_ms: Some(1500),
            insecure: true,
            no_follow_redirects: true,
        };
        let settings = options.to_settings();
        assert_eq!(settings.request_timeout_ms, 1500);
        assert!(!settings.verify_ssl);
        assert!(!settings.follow_redirects);
    }

    #[test]
    fn defaults_keep_core_settings() {
        let settings = NetworkOptions::default().to_settings();
        assert_eq!(settings, RequestSettings::default());
    }
}
