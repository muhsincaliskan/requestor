use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::Param;
use super::variable::Variable;
use crate::text::StringOrList;

const SCHEME_EXTRA: &str = "+-.";
pub const SCHEME_SEPARATOR: &str = "://";
const HOST_SEPARATOR: &str = ".";
const PORT_SEPARATOR: &str = ":";
const PATH_SEPARATOR: &str = "/";
const QUERY_MARK: &str = "?";
const QUERY_SEPARATOR: &str = "&";
const HASH_MARK: &str = "#";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlParts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Url {
    Raw(String),
    Parts(Box<UrlParts>),
}

impl Url {
    pub fn split_scheme(text: &str) -> Option<(&str, &str)> {
        let (scheme, rest) = text.split_once(SCHEME_SEPARATOR)?;
        let valid = !scheme.is_empty()
            && scheme.chars().all(|character| {
                character.is_ascii_alphanumeric() || SCHEME_EXTRA.contains(character)
            });
        valid.then_some((scheme, rest))
    }

    pub fn to_text(&self) -> String {
        match self {
            Url::Raw(text) => text.clone(),
            Url::Parts(parts) => parts.to_text(),
        }
    }
}

impl UrlParts {
    pub fn parse(raw: &str) -> Self {
        let (without_hash, hash) = split_once_optional(raw, HASH_MARK);
        let (without_query, query) = split_once_optional(without_hash, QUERY_MARK);
        let (protocol, remainder) = match without_query.split_once(SCHEME_SEPARATOR) {
            Some((protocol, remainder)) => (Some(protocol), remainder),
            None => (None, without_query),
        };
        let (authority, path) = match remainder.split_once(PATH_SEPARATOR) {
            Some((authority, path)) => (authority, Some(path)),
            None => (remainder, None),
        };
        let (host, port) = split_port(authority);
        UrlParts {
            raw: Some(raw.to_string()),
            protocol: protocol.map(str::to_string),
            host: Some(StringOrList::List(
                host.split(HOST_SEPARATOR).map(str::to_string).collect(),
            )),
            path: path.map(|path| {
                StringOrList::List(path.split(PATH_SEPARATOR).map(str::to_string).collect())
            }),
            port: port.map(str::to_string),
            query: query.map(|query| {
                query
                    .split(QUERY_SEPARATOR)
                    .map(Param::from_query_pair)
                    .collect()
            }),
            variable: None,
            hash: hash.map(str::to_string),
            extra: Map::new(),
        }
    }

    pub fn to_text(&self) -> String {
        if let Some(raw) = &self.raw {
            return raw.clone();
        }
        format!(
            "{}{}{}{}{}",
            self.scheme_text(),
            self.authority_text(),
            self.path_text(),
            self.query_text(),
            self.hash_text()
        )
    }

    fn scheme_text(&self) -> String {
        self.protocol
            .as_ref()
            .map(|name| format!("{name}{SCHEME_SEPARATOR}"))
            .unwrap_or_default()
    }

    fn authority_text(&self) -> String {
        let host = self
            .host
            .as_ref()
            .map(|host| host.joined(HOST_SEPARATOR))
            .unwrap_or_default();
        match &self.port {
            Some(port) => format!("{host}{PORT_SEPARATOR}{port}"),
            None => host,
        }
    }

    fn path_text(&self) -> String {
        let path = self
            .path
            .as_ref()
            .map(|path| path.joined(PATH_SEPARATOR))
            .unwrap_or_default();
        match path.trim_start_matches(PATH_SEPARATOR) {
            "" => String::new(),
            trimmed => format!("{PATH_SEPARATOR}{trimmed}"),
        }
    }

    fn query_text(&self) -> String {
        let pairs: Vec<String> = self
            .query
            .iter()
            .flatten()
            .filter(|param| param.is_enabled())
            .map(Param::query_pair)
            .collect();
        if pairs.is_empty() {
            return String::new();
        }
        format!("{QUERY_MARK}{}", pairs.join(QUERY_SEPARATOR))
    }

    fn hash_text(&self) -> String {
        self.hash
            .as_ref()
            .map(|hash| format!("{HASH_MARK}{hash}"))
            .unwrap_or_default()
    }
}

fn split_once_optional<'a>(text: &'a str, separator: &str) -> (&'a str, Option<&'a str>) {
    match text.split_once(separator) {
        Some((before, after)) => (before, Some(after)),
        None => (text, None),
    }
}

fn split_port(authority: &str) -> (&str, Option<&str>) {
    match authority.rsplit_once(PORT_SEPARATOR) {
        Some((host, port))
            if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            (host, Some(port))
        }
        _ => (authority, None),
    }
}

#[cfg(test)]
mod tests {
    use super::{Url, UrlParts};

    fn parts(json: &str) -> UrlParts {
        match serde_json::from_str::<Url>(json).unwrap() {
            Url::Parts(parts) => *parts,
            Url::Raw(_) => panic!("expected url object"),
        }
    }

    #[test]
    fn builds_text_from_parts_without_raw() {
        let url = parts(
            r#"{"protocol":"https","host":["api","x","com"],"port":"8443","path":["v1","u"],
                "query":[{"key":"a","value":"1"},{"key":"b","disabled":true},{"key":"flag"}],"hash":"h"}"#,
        );
        assert_eq!(url.to_text(), "https://api.x.com:8443/v1/u?a=1&flag#h");
    }

    #[test]
    fn avoids_double_slash_for_leading_slash_path() {
        let url = parts(r#"{"host":"{{base}}","path":"/v1/u"}"#);
        assert_eq!(url.to_text(), "{{base}}/v1/u");
    }

    #[test]
    fn empty_parts_give_empty_text() {
        assert_eq!(parts("{}").to_text(), "");
    }
}
