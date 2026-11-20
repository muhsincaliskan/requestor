#[cfg(test)]
mod tests;

use super::exposure::SecretExposure;
use super::{BodySource, PreparedRequest};
use crate::collection::{Body, Param};
use crate::curl_command::{shell_quote, COMMAND_NAME};
use crate::header::CONTENT_TYPE;
use crate::method::HttpMethod;
use crate::secret::Redactor;

const CONTINUATION: &str = " \\\n";
const FOLLOW_FLAG: &str = "--location";
const HEAD_FLAG: &str = "--head";
const METHOD_FLAG: &str = "--request";
const HEADER_FLAG: &str = "--header";
const RAW_DATA_FLAG: &str = "--data-raw";
const ENCODED_DATA_FLAG: &str = "--data-urlencode";
const FILE_MARK: char = '@';

impl PreparedRequest {
    pub fn to_curl(&self, redactor: &Redactor) -> String {
        self.curl_command(SecretExposure::Masked(redactor))
    }

    pub fn to_curl_with_secrets(&self) -> String {
        self.curl_command(SecretExposure::Included)
    }

    pub(super) fn curl_command(&self, exposure: SecretExposure) -> String {
        let mut command = vec![COMMAND_NAME.to_string(), FOLLOW_FLAG.to_string()];
        command.extend(self.method_flag());
        command.push(shell_quote(&exposure.url(&self.url)));
        let mut parts = vec![command.join(" ")];
        parts.extend(self.header_flags(exposure));
        parts.extend(self.body_flags(exposure));
        parts.join(CONTINUATION)
    }

    fn method_flag(&self) -> Option<String> {
        if self.method == HttpMethod::Head {
            return Some(HEAD_FLAG.to_string());
        }
        let implied = if self.body.is_some() {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        };
        (self.method != implied).then(|| format!("{METHOD_FLAG} {}", self.method.as_str()))
    }

    fn header_flags(&self, exposure: SecretExposure) -> Vec<String> {
        let multipart = matches!(self.body_source, BodySource::Form(_));
        self.headers
            .iter()
            .filter(|header| !(multipart && header.is_named(CONTENT_TYPE)))
            .map(|header| {
                let line = format!("{}: {}", header.name, exposure.header_value(header));
                format!("{HEADER_FLAG} {}", shell_quote(&line))
            })
            .collect()
    }

    fn body_flags(&self, exposure: SecretExposure) -> Vec<String> {
        match &self.body_source {
            BodySource::File(path) => {
                vec![format!(
                    "--data-binary {}",
                    shell_quote(&exposure.text(&format!("{FILE_MARK}{path}")))
                )]
            }
            BodySource::Form(fields) => fields
                .iter()
                .map(|field| {
                    let (flag, argument) = field.curl_argument(exposure.masks());
                    format!("{flag} {}", shell_quote(&exposure.text(&argument)))
                })
                .collect(),
            BodySource::Inline => self.inline_body_flags(exposure),
        }
    }

    fn inline_body_flags(&self, exposure: SecretExposure) -> Vec<String> {
        let Some(body) = &self.body else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(body);
        let content_type = self.header(CONTENT_TYPE);
        let fields = content_type
            .filter(|kind| Body::is_form_content_type(kind))
            .and_then(|_| Param::from_form_text(&text))
            .filter(|fields| fields.iter().all(Param::has_plain_form_key))
            .filter(|fields| Param::form_text(fields) == text);
        match fields {
            Some(fields) => fields
                .iter()
                .map(|field| {
                    let pair = exposure.form_pair(field);
                    format!("{ENCODED_DATA_FLAG} {}", shell_quote(&pair))
                })
                .collect(),
            None => {
                let shown = exposure.inline_body(&text, content_type);
                vec![format!("{RAW_DATA_FLAG} {}", shell_quote(&shown))]
            }
        }
    }
}
