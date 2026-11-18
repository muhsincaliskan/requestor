use std::borrow::Cow;
use std::time::Duration;

use crate::header::{find_header, Header, CONTENT_TYPE};
use crate::{json_layout, media_type, xml_layout};

const ERROR_STATUS_FLOOR: u16 = 400;

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

    pub fn has_layout(self) -> bool {
        matches!(self, Self::Json | Self::Xml)
    }

    pub fn laid_out(self, text: &str) -> String {
        match self {
            Self::Json => json_layout::reindent(text),
            Self::Xml => xml_layout::reindent(text),
            _ => text.to_string(),
        }
    }

    fn from_declared(content_type: &str) -> Option<Self> {
        match content_type {
            declared if declared.contains("json") => Some(Self::Json),
            declared if declared.contains("html") => Some(Self::Html),
            declared if declared.contains("xml") => Some(Self::Xml),
            declared if declared.starts_with("text/") => Some(Self::Text),
            declared if declared.starts_with(media_type::OCTET_STREAM) => Some(Self::Binary),
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
        if std::str::from_utf8(body).is_err_and(|error| error.error_len().is_some()) {
            return Self::Binary;
        }
        match body.iter().find(|byte| !byte.is_ascii_whitespace()) {
            Some(b'{' | b'[') => Self::Json,
            Some(b'<') => Self::Xml,
            _ => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    Informational,
    Success,
    Redirect,
    ClientError,
    ServerError,
    Unknown,
}

impl StatusClass {
    pub fn of(status: u16) -> Self {
        match status {
            100..=199 => Self::Informational,
            200..=299 => Self::Success,
            300..=399 => Self::Redirect,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::Unknown,
        }
    }

    pub fn is_error(self) -> bool {
        matches!(self, Self::ClientError | Self::ServerError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseTiming {
    pub dns: Option<Duration>,
    pub connect: Option<Duration>,
    pub first_byte: Duration,
    pub download: Duration,
}

impl ResponseTiming {
    pub fn completed_in(mut self, elapsed: Duration) -> Self {
        self.download = elapsed.saturating_sub(self.first_byte);
        self
    }
}

#[derive(Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub truncated: bool,
    pub elapsed: Duration,
    pub timing: Option<ResponseTiming>,
}

impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("headers", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name).map(|header| header.value.as_str())
    }

    pub fn format(&self) -> ResponseFormat {
        ResponseFormat::detect(self.header(CONTENT_TYPE), &self.body)
    }

    pub fn status_class(&self) -> StatusClass {
        StatusClass::of(self.status)
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_client_or_server_error(&self) -> bool {
        self.status >= ERROR_STATUS_FLOOR
    }

    pub fn body_text(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}

#[cfg(test)]
mod tests;
