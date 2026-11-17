use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::secret::Secret;

const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;
const DEFAULT_MAX_REDIRECTS: usize = 10;
const BYTES_PER_MEGABYTE: usize = 1024 * 1024;
const BYPASS_HOST_SEPARATOR: char = ',';
const BYPASS_HOST_JOINER: &str = ", ";

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

impl HttpVersion {
    pub const ALL: [HttpVersion; 3] = [HttpVersion::Auto, HttpVersion::Http1, HttpVersion::Http2];

    pub fn label(self) -> &'static str {
        match self {
            HttpVersion::Auto => "Auto",
            HttpVersion::Http1 => "HTTP/1.1",
            HttpVersion::Http2 => "HTTP/2",
        }
    }
}

impl ProxySettings {
    pub fn bypass_hosts_text(&self) -> String {
        self.bypass_hosts.join(BYPASS_HOST_JOINER)
    }

    pub fn username_text(&self) -> &str {
        self.username.as_deref().unwrap_or_default()
    }

    pub fn set_username_text(&mut self, text: &str) {
        self.username = (!text.is_empty()).then(|| text.to_string());
    }

    pub fn set_password_text(&mut self, text: &str) {
        self.password = (!text.is_empty()).then(|| Secret::new(text));
    }

    pub fn set_bypass_hosts_text(&mut self, text: &str) {
        let mut hosts: Vec<String> = Vec::new();
        for host in text.split(BYPASS_HOST_SEPARATOR).map(str::trim) {
            if !host.is_empty() && !hosts.iter().any(|known| known == host) {
                hosts.push(host.to_string());
            }
        }
        self.bypass_hosts = hosts;
    }

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

    pub fn max_response_megabytes(&self) -> u64 {
        (self.max_response_bytes / BYTES_PER_MEGABYTE) as u64
    }

    pub fn set_max_response_megabytes(&mut self, megabytes: u64) {
        self.max_response_bytes = usize::try_from(megabytes)
            .map_or(usize::MAX, |count| count.saturating_mul(BYTES_PER_MEGABYTE));
    }

    fn optional_duration(milliseconds: u64) -> Option<Duration> {
        (milliseconds > 0).then(|| Duration::from_millis(milliseconds))
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpVersion, ProxySettings, RequestSettings};

    #[test]
    fn http_versions_have_distinct_labels() {
        let labels: std::collections::HashSet<_> = HttpVersion::ALL
            .iter()
            .map(|version| version.label())
            .collect();
        assert_eq!(labels.len(), HttpVersion::ALL.len());
        assert!(HttpVersion::ALL.contains(&HttpVersion::default()));
    }

    #[test]
    fn response_size_converts_between_megabytes_and_bytes() {
        let mut settings = RequestSettings::default();
        assert_eq!(settings.max_response_megabytes(), 50);
        settings.set_max_response_megabytes(2);
        assert_eq!(settings.max_response_bytes, 2 * 1024 * 1024);
        settings.set_max_response_megabytes(0);
        assert_eq!(settings.response_limit(), usize::MAX);
        settings.set_max_response_megabytes(u64::MAX);
        assert_eq!(settings.max_response_bytes, usize::MAX);
    }

    #[test]
    fn empty_credentials_text_clears_the_value() {
        let mut proxy = ProxySettings::default();
        proxy.set_username_text("user");
        proxy.set_password_text("pw");
        assert_eq!(proxy.username_text(), "user");
        assert_eq!(proxy.password.as_ref().unwrap().expose(), "pw");
        proxy.set_username_text("");
        proxy.set_password_text("");
        assert_eq!(proxy.username_text(), "");
        assert!(proxy.username.is_none() && proxy.password.is_none());
    }

    #[test]
    fn bypass_hosts_text_is_normalized() {
        let mut proxy = ProxySettings::default();
        proxy.set_bypass_hosts_text(" localhost, ,*.corp.example ,localhost,");
        assert_eq!(proxy.bypass_hosts, ["localhost", "*.corp.example"]);
        assert_eq!(proxy.bypass_hosts_text(), "localhost, *.corp.example");
        proxy.set_bypass_hosts_text("");
        assert!(proxy.bypass_hosts.is_empty());
    }

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
