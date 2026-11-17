use crate::collection::Body;
use crate::header::Header;
use crate::secret::{is_secret_name, mask_pairs, Redactor, REDACTED};
use serde_json::Value;

const JSON_MARK: &str = "json";
const JSON_OPENERS: [char; 2] = ['{', '['];

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
        if self.masks() {
            Redactor::default().redact_url(url)
        } else {
            url.to_string()
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
