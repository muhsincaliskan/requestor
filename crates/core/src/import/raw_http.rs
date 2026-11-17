use super::{ImportError, ImportedRequest};
use crate::collection::{Body, Url, SCHEME_SEPARATOR};
use crate::header::CONTENT_TYPE;

const HTTP_VERSION_PREFIX: &str = "HTTP/";
const SECURE_PORT: &str = "443";
const HOST_HEADER: &str = "host";
const KNOWN_METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE", "CONNECT",
];

impl ImportedRequest {
    pub fn from_raw_http(text: &str) -> Result<Self, ImportError> {
        let normalized = text.replace("\r\n", "\n");
        let (head, body) = normalized
            .split_once("\n\n")
            .unwrap_or((normalized.as_str(), ""));
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or_default().trim();
        let (method, target) = parse_request_line(request_line)?;
        let headers: Vec<(&str, &str)> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim(), value.trim()))
            .collect();
        let host = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(HOST_HEADER))
            .map(|(_, value)| *value);
        let url = absolute_url(target, host)?;
        let mut imported = ImportedRequest::new(&method, &url);
        for (name, value) in headers
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case(HOST_HEADER))
        {
            imported.request.push_header(*name, *value);
        }
        if !body.trim().is_empty() {
            let content_type = imported.request.header_value(CONTENT_TYPE);
            imported.request.body = Some(Body::raw_for_content_type(body.trim_end(), content_type));
        }
        Ok(imported)
    }
}

fn parse_request_line(line: &str) -> Result<(String, &str), ImportError> {
    let mut parts = line.split_whitespace();
    let method = parts.next().map(str::to_ascii_uppercase);
    let target = parts.next();
    let version_ok = parts
        .next()
        .is_none_or(|version| version.starts_with(HTTP_VERSION_PREFIX));
    match (method, target) {
        (Some(method), Some(target)) if version_ok && KNOWN_METHODS.contains(&method.as_str()) => {
            Ok((method, target))
        }
        _ => Err(ImportError::InvalidRequestLine(line.to_string())),
    }
}

fn absolute_url(target: &str, host: Option<&str>) -> Result<String, ImportError> {
    if Url::split_scheme(target).is_some() {
        return Ok(target.to_string());
    }
    let host = host.ok_or(ImportError::MissingUrl)?;
    let scheme = if host.ends_with(&format!(":{SECURE_PORT}")) {
        "https"
    } else {
        "http"
    };
    Ok(format!("{scheme}{SCHEME_SEPARATOR}{host}{target}"))
}

#[cfg(test)]
mod tests {
    use crate::import::{ImportError, ImportedRequest};

    #[test]
    fn parses_request_line_headers_and_body() {
        let text = "POST /api/users?x=1 HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        let imported = ImportedRequest::from_raw_http(text).unwrap();
        let request = imported.request;
        assert_eq!(request.method.as_deref(), Some("POST"));
        assert_eq!(
            request.url.unwrap().to_text(),
            "http://example.com/api/users?x=1"
        );
        assert_eq!(request.header.as_ref().unwrap().len(), 1);
        assert_eq!(request.body.unwrap().raw.as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn port_443_means_https_and_absolute_targets_are_kept() {
        let secure =
            ImportedRequest::from_raw_http("GET / HTTP/1.1\nHost: example.com:443").unwrap();
        assert_eq!(
            secure.request.url.unwrap().to_text(),
            "https://example.com:443/"
        );
        let absolute = ImportedRequest::from_raw_http("GET http://a.test/p").unwrap();
        assert_eq!(absolute.request.url.unwrap().to_text(), "http://a.test/p");
    }

    #[test]
    fn rejects_non_requests() {
        assert!(matches!(
            ImportedRequest::from_raw_http("hello world"),
            Err(ImportError::InvalidRequestLine(_))
        ));
        assert_eq!(
            ImportedRequest::from_raw_http("GET /x HTTP/1.1"),
            Err(ImportError::MissingUrl)
        );
    }
}
