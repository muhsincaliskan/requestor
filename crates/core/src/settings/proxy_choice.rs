use serde_json::Value;

use super::{ProxySettings, RequestSettings};

const DIRECT_CONNECTION: &str = "none";
const CREDENTIALS_SEPARATOR: char = '@';
const SCHEME_SEPARATOR: &str = "://";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProxyChoice {
    #[default]
    Global,
    Direct,
    Custom(String),
}

impl ProxyChoice {
    pub fn is_custom(&self) -> bool {
        matches!(self, ProxyChoice::Custom(_))
    }

    pub fn same_kind(&self, other: &ProxyChoice) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn has_credentials(&self) -> bool {
        let ProxyChoice::Custom(url) = self else {
            return false;
        };
        let authority = url
            .split_once(SCHEME_SEPARATOR)
            .map_or(url.as_str(), |(_, rest)| rest);
        authority
            .split('/')
            .next()
            .is_some_and(|host| host.contains(CREDENTIALS_SEPARATOR))
    }

    pub(super) fn from_value(value: Option<&Value>) -> Self {
        match value {
            Some(Value::String(text)) if text == DIRECT_CONNECTION => ProxyChoice::Direct,
            Some(Value::String(url)) => ProxyChoice::Custom(url.clone()),
            _ => ProxyChoice::Global,
        }
    }

    pub(super) fn to_value(&self) -> Option<Value> {
        match self {
            ProxyChoice::Global => None,
            ProxyChoice::Direct => Some(Value::from(DIRECT_CONNECTION)),
            ProxyChoice::Custom(url) => Some(Value::from(url.as_str())),
        }
    }

    pub(super) fn apply_to(&self, settings: &mut RequestSettings) {
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
