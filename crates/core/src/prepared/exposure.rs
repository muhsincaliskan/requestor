use super::{FormField, FormValue};
use crate::collection::{Body, Param, JSON_OPENERS};
use crate::header::Header;
use crate::secret::{is_secret_name, mask_pairs, Redactor, REDACTED};
use serde_json::Value;

const JSON_MARK: &str = "json";

#[derive(Debug, Clone, Copy)]
pub(super) enum SecretExposure<'a> {
    Masked(&'a Redactor),
    Included,
}

impl SecretExposure<'_> {
    pub(super) fn masks(self) -> bool {
        matches!(self, SecretExposure::Masked(_))
    }

    pub(super) fn text(self, text: &str) -> String {
        match self {
            SecretExposure::Masked(redactor) => redactor.redact_text(text),
            SecretExposure::Included => text.to_string(),
        }
    }

    pub(super) fn header_value(self, header: &Header) -> String {
        match self {
            SecretExposure::Masked(redactor) => redactor.redact_text(header.shown_value()),
            SecretExposure::Included => header.value.clone(),
        }
    }

    pub(super) fn url(self, url: &str) -> String {
        match self {
            SecretExposure::Masked(redactor) => redactor.redact_url(url),
            SecretExposure::Included => url.to_string(),
        }
    }

    pub(super) fn form_field(self, field: &FormField) -> FormField {
        match &field.value {
            FormValue::Text { content_type, .. } if self.masks() && is_secret_name(&field.name) => {
                FormField::typed_text(self.text(&field.name), REDACTED, content_type.as_deref())
            }
            FormValue::Text { text, content_type } => FormField::typed_text(
                self.text(&field.name),
                self.text(text),
                content_type.as_deref(),
            ),
            FormValue::File { path, content_type } => FormField::file(
                self.text(&field.name),
                self.text(path),
                content_type.as_deref(),
            ),
        }
    }

    pub(super) fn form_pair(self, field: &Param) -> String {
        let key = field.key.as_deref().unwrap_or_default();
        let value = field.value.as_deref().unwrap_or_default();
        let shown = if self.masks() && is_secret_name(key) {
            REDACTED.to_string()
        } else {
            self.text(value)
        };
        format!("{}={shown}", self.text(key))
    }

    pub(super) fn inline_body(self, body: &str, content_type: Option<&str>) -> String {
        if !self.masks() {
            return body.to_string();
        }
        let masked = if content_type.is_some_and(Body::is_form_content_type) {
            mask_pairs(body)
        } else {
            mask_json_text(body, content_type)
        };
        self.text(&masked)
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
