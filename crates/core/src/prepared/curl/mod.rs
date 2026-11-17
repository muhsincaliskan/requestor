mod mask;
#[cfg(test)]
mod tests;

use super::{BodySource, PreparedRequest};
use crate::curl_command::COMMAND_NAME;
use crate::header::CONTENT_TYPE;
use crate::method::HttpMethod;
use mask::SecretExposure;

const CONTINUATION: &str = " \\\n  ";
const SINGLE_QUOTE_ESCAPE: &str = "'\\''";
const HEAD_FLAG: &str = "-I";
const FILE_MARK: char = '@';

impl PreparedRequest {
    pub fn to_curl(&self) -> String {
        self.curl_command(SecretExposure::Masked)
    }

    pub fn to_curl_with_secrets(&self) -> String {
        self.curl_command(SecretExposure::Included)
    }

    fn curl_command(&self, exposure: SecretExposure) -> String {
        let mut parts = vec![COMMAND_NAME.to_string()];
        parts.extend(self.method_flag());
        parts.push(shell_quote(&exposure.url(&self.url)));
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
        (self.method != implied).then(|| format!("-X {}", self.method.as_str()))
    }

    fn header_flags(&self, exposure: SecretExposure) -> Vec<String> {
        let multipart = matches!(self.body_source, BodySource::Form(_));
        self.headers
            .iter()
            .filter(|header| !(multipart && header.is_named(CONTENT_TYPE)))
            .map(|header| {
                let line = format!("{}: {}", header.name, exposure.header_value(header));
                format!("-H {}", shell_quote(&line))
            })
            .collect()
    }

    fn body_flags(&self, exposure: SecretExposure) -> Vec<String> {
        match &self.body_source {
            BodySource::File(path) => {
                vec![format!(
                    "--data-binary {}",
                    shell_quote(&format!("{FILE_MARK}{path}"))
                )]
            }
            BodySource::Form(fields) => fields
                .iter()
                .map(|field| {
                    let (flag, argument) = field.curl_argument(exposure.masks());
                    format!("{flag} {}", shell_quote(&argument))
                })
                .collect(),
            BodySource::Inline => self
                .body
                .iter()
                .map(|body| {
                    let text = String::from_utf8_lossy(body);
                    let shown = exposure.inline_body(&text, self.header(CONTENT_TYPE));
                    format!("--data-raw {}", shell_quote(&shown))
                })
                .collect(),
        }
    }
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', SINGLE_QUOTE_ESCAPE))
}
