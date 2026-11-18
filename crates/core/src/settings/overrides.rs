use serde_json::{Map, Value};

use super::{HttpVersion, ProxyChoice, RequestSettings};

const PROTOCOL_PROFILE_BEHAVIOR: &str = "protocolProfileBehavior";
const FOLLOW_REDIRECTS: &str = "followRedirects";
const MAX_REDIRECTS: &str = "maxRedirects";
const STRICT_SSL: &str = "strictSSL";
const PROTOCOL_VERSION: &str = "protocolVersion";
const TIMEOUT: &str = "requestorTimeoutMs";
const PROXY: &str = "requestorProxy";
const SKIP_CLIENT_CERTIFICATES: &str = "requestorSkipClientCertificates";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestOverrides {
    pub http_version: Option<HttpVersion>,
    pub timeout_ms: Option<u64>,
    pub verify_ssl: Option<bool>,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<usize>,
    pub proxy: ProxyChoice,
    pub skip_client_certificates: bool,
}

impl RequestOverrides {
    pub fn from_extra(extra: &Map<String, Value>) -> Self {
        match extra.get(PROTOCOL_PROFILE_BEHAVIOR) {
            Some(Value::Object(behavior)) => RequestOverrides::from_behavior(behavior),
            _ => RequestOverrides::default(),
        }
    }

    pub fn stored_in(extra: &Map<String, Value>) -> Option<Self> {
        extra
            .contains_key(PROTOCOL_PROFILE_BEHAVIOR)
            .then(|| RequestOverrides::from_extra(extra))
    }

    pub fn write_into(&self, extra: &mut Map<String, Value>) {
        let previous = RequestOverrides::from_extra(extra);
        let changed: Vec<_> = self
            .entries()
            .into_iter()
            .zip(previous.entries())
            .filter(|(current, before)| current != before)
            .map(|(current, _)| current)
            .collect();
        if changed.is_empty() {
            return;
        }
        let mut behavior = match extra.remove(PROTOCOL_PROFILE_BEHAVIOR) {
            Some(Value::Object(behavior)) => behavior,
            _ => Map::new(),
        };
        for (key, value) in changed {
            match value {
                Some(value) => behavior.insert(key.to_string(), value),
                None => behavior.remove(key),
            };
        }
        if !behavior.is_empty() {
            extra.insert(
                PROTOCOL_PROFILE_BEHAVIOR.to_string(),
                Value::Object(behavior),
            );
        }
    }

    pub fn store_in(&self, extra: &mut Map<String, Value>) {
        self.write_into(extra);
        extra
            .entry(PROTOCOL_PROFILE_BEHAVIOR)
            .or_insert_with(|| Value::Object(Map::new()));
    }

    pub fn is_empty(&self) -> bool {
        *self == RequestOverrides::default()
    }

    pub fn count(&self) -> usize {
        self.entries()
            .iter()
            .filter(|(_, value)| value.is_some())
            .count()
    }

    pub fn lowers_security(&self) -> bool {
        self.verify_ssl == Some(false) || self.proxy.uses_custom_proxy()
    }

    pub fn apply_to(&self, global: &RequestSettings) -> RequestSettings {
        let mut settings = global.clone();
        settings.http_version = self.http_version.unwrap_or(global.http_version);
        settings.request_timeout_ms = self.timeout_ms.unwrap_or(global.request_timeout_ms);
        settings.verify_ssl = self.verify_ssl.unwrap_or(global.verify_ssl);
        settings.follow_redirects = self.follow_redirects.unwrap_or(global.follow_redirects);
        settings.max_redirects = self.max_redirects.unwrap_or(global.max_redirects);
        self.proxy.apply_to(&mut settings);
        if self.skip_client_certificates {
            settings.client_certificates.clear();
        }
        settings
    }

    pub fn timeout_text(&self) -> String {
        number_text(self.timeout_ms)
    }

    pub fn set_timeout_text(&mut self, text: &str) {
        if let Some(timeout) = parse_number(text) {
            self.timeout_ms = timeout;
        }
    }

    pub fn max_redirects_text(&self) -> String {
        number_text(self.max_redirects)
    }

    pub fn set_max_redirects_text(&mut self, text: &str) {
        if let Some(limit) = parse_number(text) {
            self.max_redirects = limit;
        }
    }

    fn from_behavior(behavior: &Map<String, Value>) -> Self {
        RequestOverrides {
            http_version: read(behavior, PROTOCOL_VERSION),
            timeout_ms: read(behavior, TIMEOUT),
            verify_ssl: read(behavior, STRICT_SSL),
            follow_redirects: read(behavior, FOLLOW_REDIRECTS),
            max_redirects: read(behavior, MAX_REDIRECTS),
            proxy: ProxyChoice::from_value(behavior.get(PROXY)),
            skip_client_certificates: read(behavior, SKIP_CLIENT_CERTIFICATES).unwrap_or(false),
        }
    }

    fn entries(&self) -> [(&'static str, Option<Value>); 7] {
        [
            (
                PROTOCOL_VERSION,
                self.http_version.map(HttpVersion::to_value),
            ),
            (TIMEOUT, self.timeout_ms.map(Value::from)),
            (STRICT_SSL, self.verify_ssl.map(Value::from)),
            (FOLLOW_REDIRECTS, self.follow_redirects.map(Value::from)),
            (MAX_REDIRECTS, self.max_redirects.map(Value::from)),
            (PROXY, self.proxy.to_value()),
            (
                SKIP_CLIENT_CERTIFICATES,
                self.skip_client_certificates.then_some(Value::Bool(true)),
            ),
        ]
    }
}

fn read<T: serde::de::DeserializeOwned>(behavior: &Map<String, Value>, key: &str) -> Option<T> {
    serde_json::from_value(behavior.get(key)?.clone()).ok()
}

fn number_text<T: ToString>(value: Option<T>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn parse_number<T: std::str::FromStr>(text: &str) -> Option<Option<T>> {
    let text = text.trim();
    if text.is_empty() {
        return Some(None);
    }
    text.parse().ok().map(Some)
}
