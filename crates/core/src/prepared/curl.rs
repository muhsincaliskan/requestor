use super::PreparedRequest;
use crate::method::HttpMethod;

const CONTINUATION: &str = " \\\n  ";
const SINGLE_QUOTE_ESCAPE: &str = "'\\''";

impl PreparedRequest {
    pub fn to_curl(&self) -> String {
        let mut parts = vec!["curl".to_string()];
        if self.needs_explicit_method() {
            parts.push(format!("-X {}", self.method.as_str()));
        }
        parts.push(shell_quote(&self.url));
        parts.extend(self.headers.iter().map(|header| {
            format!(
                "-H {}",
                shell_quote(&format!("{}: {}", header.name, header.value))
            )
        }));
        if let Some(body) = &self.body {
            parts.push(format!(
                "--data-raw {}",
                shell_quote(&String::from_utf8_lossy(body))
            ));
        }
        parts.join(CONTINUATION)
    }

    fn needs_explicit_method(&self) -> bool {
        let implied = if self.body.is_some() {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        };
        self.method != implied
    }
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', SINGLE_QUOTE_ESCAPE))
}
