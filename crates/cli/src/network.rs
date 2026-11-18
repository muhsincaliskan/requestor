use std::path::PathBuf;

use requestor_core::settings::{ClientCertificate, RequestSettings};

use crate::cursor::Cursor;
use crate::error::CliError;

pub const TIMEOUT_FLAG: &str = "--timeout-ms";
pub const INSECURE_FLAG: &str = "--insecure";
pub const NO_FOLLOW_FLAG: &str = "--no-follow-redirects";
pub const CERT_FLAG: &str = "--cert";
pub const KEY_FLAG: &str = "--key";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct NetworkOptions {
    pub timeout_ms: Option<u64>,
    pub insecure: bool,
    pub no_follow_redirects: bool,
    pub certificate: Option<PathBuf>,
    pub key: Option<PathBuf>,
}

impl NetworkOptions {
    pub fn accept(&mut self, flag: &str, cursor: &mut Cursor) -> Result<bool, CliError> {
        match flag {
            TIMEOUT_FLAG => self.timeout_ms = Some(cursor.number(flag)?),
            INSECURE_FLAG => self.insecure = true,
            NO_FOLLOW_FLAG => self.no_follow_redirects = true,
            CERT_FLAG => self.certificate = Some(PathBuf::from(cursor.value(flag)?)),
            KEY_FLAG => self.key = Some(PathBuf::from(cursor.value(flag)?)),
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn validated(self) -> Result<Self, CliError> {
        if self.key.is_some() && self.certificate.is_none() {
            return Err(CliError::usage(format!("{KEY_FLAG} needs {CERT_FLAG}")));
        }
        Ok(self)
    }

    pub fn to_settings(&self) -> RequestSettings {
        let mut settings = RequestSettings::default();
        if let Some(timeout_ms) = self.timeout_ms {
            settings.request_timeout_ms = timeout_ms;
        }
        settings.verify_ssl = !self.insecure;
        settings.follow_redirects = !self.no_follow_redirects;
        settings.client_certificates = self
            .certificate
            .iter()
            .map(|certificate| {
                ClientCertificate::for_every_host(certificate.clone(), self.key.clone())
            })
            .collect();
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
            certificate: Some(PathBuf::from("client.pem")),
            key: Some(PathBuf::from("client.key")),
        };
        let settings = options.to_settings();
        assert_eq!(settings.request_timeout_ms, 1500);
        assert!(!settings.verify_ssl);
        assert!(!settings.follow_redirects);
        assert_eq!(
            settings.client_certificates,
            [ClientCertificate::for_every_host(
                PathBuf::from("client.pem"),
                Some(PathBuf::from("client.key"))
            )]
        );
    }

    #[test]
    fn a_key_needs_a_certificate() {
        let options = NetworkOptions {
            key: Some(PathBuf::from("client.key")),
            ..NetworkOptions::default()
        };
        assert!(options.validated().is_err());
    }

    #[test]
    fn defaults_keep_core_settings() {
        let settings = NetworkOptions::default().to_settings();
        assert_eq!(settings, RequestSettings::default());
    }
}
