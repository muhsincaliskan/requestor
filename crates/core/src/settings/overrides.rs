use serde_json::{Map, Value};

use super::{HttpVersion, ProxySettings, RequestSettings};

const PROTOCOL_PROFILE_BEHAVIOR: &str = "protocolProfileBehavior";
const FOLLOW_REDIRECTS: &str = "followRedirects";
const MAX_REDIRECTS: &str = "maxRedirects";
const STRICT_SSL: &str = "strictSSL";
const PROTOCOL_VERSION: &str = "protocolVersion";
const TIMEOUT: &str = "requestorTimeoutMs";
const PROXY: &str = "requestorProxy";
const CLIENT_CERTIFICATES: &str = "requestorClientCertificates";
const DIRECT_CONNECTION: &str = "none";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProxyChoice {
    #[default]
    Global,
    Direct,
    Custom(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestOverrides {
    pub http_version: Option<HttpVersion>,
    pub timeout_ms: Option<u64>,
    pub verify_ssl: Option<bool>,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<usize>,
    pub proxy: ProxyChoice,
    pub client_certificates: Option<bool>,
}

impl ProxyChoice {
    pub fn label(&self) -> &'static str {
        match self {
            ProxyChoice::Global => "App setting",
            ProxyChoice::Direct => "No proxy",
            ProxyChoice::Custom(_) => "Custom proxy",
        }
    }

    fn from_value(value: Option<&Value>) -> Self {
        match value {
            Some(Value::String(text)) if text == DIRECT_CONNECTION => ProxyChoice::Direct,
            Some(Value::String(url)) => ProxyChoice::Custom(url.clone()),
            _ => ProxyChoice::Global,
        }
    }

    fn to_value(&self) -> Option<Value> {
        match self {
            ProxyChoice::Global => None,
            ProxyChoice::Direct => Some(Value::from(DIRECT_CONNECTION)),
            ProxyChoice::Custom(url) => Some(Value::from(url.trim())),
        }
    }

    fn apply_to(&self, settings: &mut RequestSettings) {
        match self {
            ProxyChoice::Global => {}
            ProxyChoice::Custom(url) if url.trim().is_empty() => {}
            ProxyChoice::Direct => {
                settings.use_system_proxy = false;
                settings.proxy = None;
            }
            ProxyChoice::Custom(url) => {
                settings.use_system_proxy = false;
                settings.proxy = Some(ProxySettings {
                    url: url.trim().to_string(),
                    ..ProxySettings::default()
                });
            }
        }
    }
}

impl RequestOverrides {
    pub fn from_extra(extra: &Map<String, Value>) -> Self {
        let Some(Value::Object(behavior)) = extra.get(PROTOCOL_PROFILE_BEHAVIOR) else {
            return RequestOverrides::default();
        };
        RequestOverrides {
            http_version: read(behavior, PROTOCOL_VERSION),
            timeout_ms: read(behavior, TIMEOUT),
            verify_ssl: read(behavior, STRICT_SSL),
            follow_redirects: read(behavior, FOLLOW_REDIRECTS),
            max_redirects: read(behavior, MAX_REDIRECTS),
            proxy: ProxyChoice::from_value(behavior.get(PROXY)),
            client_certificates: read(behavior, CLIENT_CERTIFICATES),
        }
    }

    pub fn write_into(&self, extra: &mut Map<String, Value>) {
        let mut behavior = match extra.remove(PROTOCOL_PROFILE_BEHAVIOR) {
            Some(Value::Object(behavior)) => behavior,
            Some(other) if self.is_empty() => {
                extra.insert(PROTOCOL_PROFILE_BEHAVIOR.to_string(), other);
                return;
            }
            _ => Map::new(),
        };
        put(
            &mut behavior,
            PROTOCOL_VERSION,
            self.http_version.map(to_value),
        );
        put(&mut behavior, TIMEOUT, self.timeout_ms.map(Value::from));
        put(&mut behavior, STRICT_SSL, self.verify_ssl.map(Value::from));
        put(
            &mut behavior,
            FOLLOW_REDIRECTS,
            self.follow_redirects.map(Value::from),
        );
        put(
            &mut behavior,
            MAX_REDIRECTS,
            self.max_redirects.map(Value::from),
        );
        put(&mut behavior, PROXY, self.proxy.to_value());
        put(
            &mut behavior,
            CLIENT_CERTIFICATES,
            self.client_certificates.map(Value::from),
        );
        if !behavior.is_empty() {
            extra.insert(
                PROTOCOL_PROFILE_BEHAVIOR.to_string(),
                Value::Object(behavior),
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == RequestOverrides::default()
    }

    pub fn count(&self) -> usize {
        [
            self.http_version.is_some(),
            self.timeout_ms.is_some(),
            self.verify_ssl.is_some(),
            self.follow_redirects.is_some(),
            self.max_redirects.is_some(),
            self.proxy != ProxyChoice::Global,
            self.client_certificates.is_some(),
        ]
        .into_iter()
        .filter(|set| *set)
        .count()
    }

    pub fn apply_to(&self, global: &RequestSettings) -> RequestSettings {
        let mut settings = global.clone();
        settings.http_version = self.http_version.unwrap_or(global.http_version);
        settings.request_timeout_ms = self.timeout_ms.unwrap_or(global.request_timeout_ms);
        settings.verify_ssl = self.verify_ssl.unwrap_or(global.verify_ssl);
        settings.follow_redirects = self.follow_redirects.unwrap_or(global.follow_redirects);
        settings.max_redirects = self.max_redirects.unwrap_or(global.max_redirects);
        self.proxy.apply_to(&mut settings);
        if self.client_certificates == Some(false) {
            settings.client_certificates.clear();
        }
        settings
    }

    pub fn timeout_text(&self) -> String {
        number_text(self.timeout_ms)
    }

    pub fn set_timeout_text(&mut self, text: &str) {
        self.timeout_ms = parse_number(text);
    }

    pub fn max_redirects_text(&self) -> String {
        number_text(self.max_redirects)
    }

    pub fn set_max_redirects_text(&mut self, text: &str) {
        self.max_redirects = parse_number(text);
    }
}

fn read<T: serde::de::DeserializeOwned>(behavior: &Map<String, Value>, key: &str) -> Option<T> {
    serde_json::from_value(behavior.get(key)?.clone()).ok()
}

fn to_value(version: HttpVersion) -> Value {
    serde_json::to_value(version).unwrap_or(Value::Null)
}

fn put(behavior: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(value) => behavior.insert(key.to_string(), value),
        None => behavior.remove(key),
    };
}

fn number_text<T: ToString>(value: Option<T>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn parse_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}
