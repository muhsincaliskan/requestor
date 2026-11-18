use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::secret::Secret;

mod certificate;

pub use certificate::ClientCertificate;

const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_RESPONSE_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_EVENT_STREAM_BYTES: usize = 16 * BYTES_PER_MEGABYTE;
pub const MAX_SOCKET_MESSAGE_BYTES: usize = 16 * BYTES_PER_MEGABYTE;
const DEFAULT_MAX_REDIRECTS: usize = 10;
pub const BYTES_PER_MEGABYTE: usize = 1024 * 1024;
const BYPASS_HOST_SEPARATOR: char = ',';
const BYPASS_HOST_JOINER: &str = ", ";
const BYPASS_EVERYTHING: &str = "*";
const DOMAIN_SEPARATOR: char = '.';

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

    pub fn bypasses(&self, host: &str) -> bool {
        self.bypass_hosts.iter().any(|entry| {
            let domain = entry.trim_start_matches(DOMAIN_SEPARATOR);
            entry == BYPASS_EVERYTHING
                || host.eq_ignore_ascii_case(domain)
                || host.to_ascii_lowercase().ends_with(&format!(
                    "{DOMAIN_SEPARATOR}{}",
                    domain.to_ascii_lowercase()
                ))
        })
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
    pub client_certificates: Vec<ClientCertificate>,
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
            client_certificates: Vec::new(),
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

    pub fn event_stream_limit(&self) -> usize {
        self.response_limit().min(MAX_EVENT_STREAM_BYTES)
    }

    pub fn socket_message_limit(&self) -> usize {
        self.response_limit().min(MAX_SOCKET_MESSAGE_BYTES)
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
mod tests;
