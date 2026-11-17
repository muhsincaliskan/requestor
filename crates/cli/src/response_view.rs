use requestor_core::header::Header;
use requestor_core::response::{HttpResponse, ResponseFormat};
use requestor_core::secret::REDACTED;

pub struct ResponseView {
    head: String,
    body: String,
    pub summary: String,
    pub truncated: bool,
}

impl ResponseView {
    pub fn from(response: &HttpResponse) -> Self {
        let mut head = format!(
            "{} {} {}\n",
            response.version, response.status, response.reason
        );
        for header in &response.headers {
            head.push_str(&format!("{}: {}\n", header.name, display_value(header)));
        }
        ResponseView {
            head,
            body: display_body(response),
            summary: format!(
                "{} ms, {} B",
                response.elapsed.as_millis(),
                response.body.len()
            ),
            truncated: response.truncated,
        }
    }

    pub fn to_text(&self) -> String {
        format!("{}\n{}", self.head, self.body)
    }
}

fn display_value(header: &Header) -> &str {
    if header.is_sensitive() {
        REDACTED
    } else {
        &header.value
    }
}

fn display_body(response: &HttpResponse) -> String {
    match response.format() {
        ResponseFormat::Binary => format!("[binary body, {} bytes]", response.body.len()),
        _ => response.body_text(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn response(body: &[u8], content_type: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            reason: "OK".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: vec![
                Header::new("Content-Type", content_type),
                Header::new("Set-Cookie", "session=abc"),
            ],
            body: body.to_vec(),
            truncated: false,
            elapsed: Duration::from_millis(7),
        }
    }

    #[test]
    fn renders_status_headers_and_body() {
        let view = ResponseView::from(&response(b"hello", "text/plain"));
        assert_eq!(
            view.to_text(),
            format!("HTTP/1.1 200 OK\nContent-Type: text/plain\nSet-Cookie: {REDACTED}\n\nhello")
        );
        assert_eq!(view.summary, "7 ms, 5 B");
    }

    #[test]
    fn binary_bodies_are_summarized() {
        let view = ResponseView::from(&response(&[0, 1, 2], "image/png"));
        assert!(view.to_text().ends_with("[binary body, 3 bytes]"));
    }
}
