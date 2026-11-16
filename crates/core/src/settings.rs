use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::secret::Secret;

const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;
const DEFAULT_MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpVersion {
    #[default]
    Auto,
    Http1,
    Http2,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxySettings {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<Secret>,
    pub bypass_hosts: Vec<String>,
}

impl ProxySettings {
    pub fn has_credentials(&self) -> bool {
        self.username.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestSettings {
    pub http_version: HttpVersion,
    pub request_timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub max_response_bytes: usize,
    pub verify_ssl: bool,
    pub cookies_enabled: bool,
    pub follow_redirects: bool,
    pub max_redirects: usize,
    pub send_no_cache_header: bool,
    pub send_postman_token_header: bool,
    pub use_system_proxy: bool,
    pub proxy: Option<ProxySettings>,
}

impl Default for RequestSettings {
    fn default() -> Self {
        RequestSettings {
            http_version: HttpVersion::default(),
            request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            verify_ssl: true,
            cookies_enabled: true,
            follow_redirects: true,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            send_no_cache_header: true,
            send_postman_token_header: true,
            use_system_proxy: true,
            proxy: None,
        }
    }
}

impl RequestSettings {
    pub fn request_timeout(&self) -> Option<Duration> {
        Self::optional_duration(self.request_timeout_ms)
    }

    pub fn connect_timeout(&self) -> Option<Duration> {
        Self::optional_duration(self.connect_timeout_ms)
    }

    pub fn response_limit(&self) -> usize {
        match self.max_response_bytes {
            0 => usize::MAX,
            limit => limit,
        }
    }

    fn optional_duration(milliseconds: u64) -> Option<Duration> {
        (milliseconds > 0).then(|| Duration::from_millis(milliseconds))
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpVersion, RequestSettings};

    #[test]
    fn zero_timeout_means_no_timeout() {
        let settings = RequestSettings {
            request_timeout_ms: 0,
            ..RequestSettings::default()
        };
        assert_eq!(settings.request_timeout(), None);
        assert!(settings.connect_timeout().is_some());
    }

    #[test]
    fn zero_response_limit_means_unlimited() {
        let settings = RequestSettings {
            max_response_bytes: 0,
            ..RequestSettings::default()
        };
        assert_eq!(settings.response_limit(), usize::MAX);
        assert_eq!(
            RequestSettings::default().response_limit(),
            RequestSettings::default().max_response_bytes
        );
    }

    #[test]
    fn defaults_are_safe() {
        let settings = RequestSettings::default();
        assert!(settings.verify_ssl);
        assert!(settings.follow_redirects);
        assert_eq!(settings.http_version, HttpVersion::Auto);
    }

    #[test]
    fn partial_documents_fall_back_to_defaults() {
        let settings: RequestSettings =
            serde_json::from_str(r#"{"verify_ssl":false,"http_version":"http2"}"#).unwrap();
        assert!(!settings.verify_ssl);
        assert_eq!(settings.http_version, HttpVersion::Http2);
        assert_eq!(
            settings.max_redirects,
            RequestSettings::default().max_redirects
        );
    }

    #[test]
    fn proxy_password_is_masked_in_debug_output() {
        let json =
            r#"{"proxy":{"url":"http://p:8080","username":"u","password":"pw","bypass_hosts":[]}}"#;
        let settings: RequestSettings = serde_json::from_str(json).unwrap();
        assert!(!format!("{settings:?}").contains("pw\""));
    }
}
