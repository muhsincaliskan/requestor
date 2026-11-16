use std::time::Duration;

use crate::header::{find_header, Header};

const CONTENT_TYPE: &str = "content-type";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    Json,
    Xml,
    Html,
    Text,
    Binary,
}

impl ResponseFormat {
    pub fn detect(content_type: Option<&str>, body: &[u8]) -> Self {
        match content_type.map(str::to_ascii_lowercase) {
            Some(declared) => Self::from_declared(&declared).unwrap_or_else(|| Self::sniff(body)),
            None => Self::sniff(body),
        }
    }

    fn from_declared(content_type: &str) -> Option<Self> {
        match content_type {
            declared if declared.contains("json") => Some(Self::Json),
            declared if declared.contains("html") => Some(Self::Html),
            declared if declared.contains("xml") => Some(Self::Xml),
            declared if declared.starts_with("text/") => Some(Self::Text),
            declared if declared.starts_with("application/octet-stream") => Some(Self::Binary),
            declared
                if ["image/", "audio/", "video/"]
                    .iter()
                    .any(|kind| declared.starts_with(kind)) =>
            {
                Some(Self::Binary)
            }
            _ => None,
        }
    }

    fn sniff(body: &[u8]) -> Self {
        if std::str::from_utf8(body).is_err() {
            return Self::Binary;
        }
        match body.iter().find(|byte| !byte.is_ascii_whitespace()) {
            Some(b'{' | b'[') => Self::Json,
            Some(b'<') => Self::Xml,
            _ => Self::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub truncated: bool,
    pub elapsed: Duration,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name).map(|header| header.value.as_str())
    }

    pub fn format(&self) -> ResponseFormat {
        ResponseFormat::detect(self.header(CONTENT_TYPE), &self.body)
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseFormat;

    #[test]
    fn declared_content_type_wins() {
        assert_eq!(
            ResponseFormat::detect(Some("application/problem+json"), b"x"),
            ResponseFormat::Json
        );
        assert_eq!(
            ResponseFormat::detect(Some("text/html; charset=utf-8"), b"{}"),
            ResponseFormat::Html
        );
        assert_eq!(
            ResponseFormat::detect(Some("image/png"), b"abc"),
            ResponseFormat::Binary
        );
    }

    #[test]
    fn sniffs_body_when_type_is_missing_or_generic() {
        assert_eq!(
            ResponseFormat::detect(None, b"  {\"a\":1}"),
            ResponseFormat::Json
        );
        assert_eq!(
            ResponseFormat::detect(Some("application/x-unknown"), b"<a/>"),
            ResponseFormat::Xml
        );
        assert_eq!(
            ResponseFormat::detect(None, &[0xff, 0xfe, 0x00]),
            ResponseFormat::Binary
        );
        assert_eq!(ResponseFormat::detect(None, b"hello"), ResponseFormat::Text);
    }
}
