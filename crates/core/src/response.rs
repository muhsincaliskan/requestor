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

#[derive(Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub truncated: bool,
    pub elapsed: Duration,
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
mod tests {
    use std::time::Duration;

    use super::{HttpResponse, ResponseFormat, StatusClass};

    fn with_status(status: u16) -> HttpResponse {
        HttpResponse {
            status,
            reason: String::new(),
            version: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
            truncated: false,
            elapsed: Duration::ZERO,
        }
    }

    #[test]
    fn status_classes_are_distinguished() {
        let expectations = [
            (200, true, false),
            (204, true, false),
            (302, false, false),
            (399, false, false),
            (400, false, true),
            (503, false, true),
        ];
        for (status, success, error) in expectations {
            let response = with_status(status);
            assert_eq!(response.is_success(), success, "{status}");
            assert_eq!(response.is_client_or_server_error(), error, "{status}");
        }
    }

    #[test]
    fn status_codes_map_to_their_class() {
        let expectations = [
            (99, StatusClass::Unknown),
            (101, StatusClass::Informational),
            (204, StatusClass::Success),
            (301, StatusClass::Redirect),
            (404, StatusClass::ClientError),
            (500, StatusClass::ServerError),
            (600, StatusClass::Unknown),
        ];
        for (status, class) in expectations {
            assert_eq!(with_status(status).status_class(), class, "{status}");
        }
        assert!(StatusClass::ServerError.is_error());
        assert!(!StatusClass::Redirect.is_error());
    }

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
    fn json_and_xml_have_a_pretty_layout() {
        assert!(ResponseFormat::Xml.has_layout());
        assert_eq!(
            ResponseFormat::Xml.laid_out("<a><b/></a>"),
            "<a>\n  <b/>\n</a>"
        );
        assert!(!ResponseFormat::Html.has_layout());
        assert_eq!(ResponseFormat::Text.laid_out("<a><b/></a>"), "<a><b/></a>");
    }

    #[test]
    fn truncated_utf8_tail_is_still_text() {
        let mut body = "héllo".as_bytes().to_vec();
        body.pop();
        assert_eq!(ResponseFormat::detect(None, &body), ResponseFormat::Text);
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
