use std::borrow::Cow;
use std::io::{self, Write};

use requestor_core::response::{HttpResponse, ResponseFormat};

pub struct ResponseView<'a> {
    head: String,
    body: Cow<'a, str>,
    pub summary: String,
    pub truncated: bool,
}

impl<'a> ResponseView<'a> {
    pub fn from(response: &'a HttpResponse) -> Self {
        let mut head = format!(
            "{} {} {}\n",
            response.version, response.status, response.reason
        );
        for header in &response.headers {
            head.push_str(&format!("{}: {}\n", header.name, header.shown_value()));
        }
        ResponseView {
            head,
            body: shown_body(response),
            summary: format!(
                "{} ms, {} B",
                response.elapsed.as_millis(),
                response.body.len()
            ),
            truncated: response.truncated,
        }
    }

    pub fn write_to(&self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "{}", self.head)?;
        out.write_all(self.body.as_bytes())
    }
}

fn shown_body(response: &HttpResponse) -> Cow<'_, str> {
    match response.format() {
        ResponseFormat::Binary => {
            Cow::Owned(format!("[binary body, {} bytes]", response.body.len()))
        }
        _ => response.body_text(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use requestor_core::header::Header;
    use requestor_core::secret::REDACTED;

    use super::*;

    fn written(view: &ResponseView) -> String {
        let mut out = Vec::new();
        view.write_to(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

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
        let response = response(b"hello", "text/plain");
        let view = ResponseView::from(&response);
        assert_eq!(
            written(&view),
            format!("HTTP/1.1 200 OK\nContent-Type: text/plain\nSet-Cookie: {REDACTED}\n\nhello")
        );
        assert_eq!(view.summary, "7 ms, 5 B");
    }

    #[test]
    fn binary_bodies_are_summarized() {
        let response = response(&[0, 1, 2], "image/png");
        let view = ResponseView::from(&response);
        assert!(written(&view).ends_with("[binary body, 3 bytes]"));
    }
}
