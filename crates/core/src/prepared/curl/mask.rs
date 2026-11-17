use crate::collection::{Body, Url, SCHEME_SEPARATOR};
use crate::encoding::FORM_PAIR_SEPARATOR;
use crate::header::Header;
use crate::secret::{is_secret_name, REDACTED};
use serde_json::Value;

const QUERY_MARK: char = '?';
const JSON_MARK: &str = "json";
const JSON_OPENERS: [char; 2] = ['{', '['];
const VALUE_SEPARATOR: char = '=';
const PATH_START: char = '/';
const USERINFO_END: char = '@';
const PASSWORD_START: char = ':';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecretExposure {
    Masked,
    Included,
}

impl SecretExposure {
    pub(super) fn masks(self) -> bool {
        self == SecretExposure::Masked
    }

    pub(super) fn header_value(self, header: &Header) -> String {
        if self.masks() && header.is_sensitive() {
            REDACTED.to_string()
        } else {
            header.value.clone()
        }
    }

    pub(super) fn url(self, url: &str) -> String {
        if !self.masks() {
            return url.to_string();
        }
        match url.split_once(QUERY_MARK) {
            Some((base, query)) => {
                format!("{}{QUERY_MARK}{}", mask_userinfo(base), mask_pairs(query))
            }
            None => mask_userinfo(url),
        }
    }

    pub(super) fn inline_body(self, body: &str, content_type: Option<&str>) -> String {
        if !self.masks() {
            return body.to_string();
        }
        if content_type.is_some_and(Body::is_form_content_type) {
            return mask_pairs(body);
        }
        mask_json_text(body, content_type)
    }
}

fn mask_pairs(text: &str) -> String {
    text.split(FORM_PAIR_SEPARATOR)
        .map(mask_pair)
        .collect::<Vec<_>>()
        .join(FORM_PAIR_SEPARATOR)
}

fn mask_pair(pair: &str) -> String {
    match pair.split_once(VALUE_SEPARATOR) {
        Some((key, _)) if is_secret_name(key) => format!("{key}{VALUE_SEPARATOR}{REDACTED}"),
        _ => pair.to_string(),
    }
}

fn mask_userinfo(base: &str) -> String {
    let Some((scheme, rest)) = Url::split_scheme(base) else {
        return base.to_string();
    };
    let (authority, path) = rest.split_at(rest.find(PATH_START).unwrap_or(rest.len()));
    match authority.rsplit_once(USERINFO_END) {
        Some((userinfo, host)) if userinfo.contains(PASSWORD_START) => {
            let user = userinfo.split(PASSWORD_START).next().unwrap_or_default();
            format!("{scheme}{SCHEME_SEPARATOR}{user}{PASSWORD_START}{REDACTED}{USERINFO_END}{host}{path}")
        }
        _ => base.to_string(),
    }
}

fn mask_json_text(body: &str, content_type: Option<&str>) -> String {
    let declared_json =
        content_type.is_some_and(|kind| kind.to_ascii_lowercase().contains(JSON_MARK));
    if !declared_json && !body.trim_start().starts_with(JSON_OPENERS) {
        return body.to_string();
    }
    match serde_json::from_str::<Value>(body) {
        Ok(mut document) => {
            mask_json(&mut document);
            document.to_string()
        }
        Err(_) if declared_json => REDACTED.to_string(),
        Err(_) => body.to_string(),
    }
}

fn mask_json(value: &mut Value) {
    match value {
        Value::Object(members) => {
            for (key, member) in members.iter_mut() {
                if is_secret_name(key) {
                    *member = Value::String(REDACTED.to_string());
                } else {
                    mask_json(member);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(mask_json),
        _ => {}
    }
}
