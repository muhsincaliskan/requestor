mod buffer;
mod search;

pub use buffer::{ResponseBuffer, BYTES_PER_MEGABYTE};
pub use search::BodySearch;

use std::time::Duration;

use requestor_core::checks::RulesReport;
use requestor_core::header::Header;
use requestor_core::response::{HttpResponse, ResponseFormat, StatusClass};
use requestor_core::text_diff::LineDiff;

const BINARY_NOTICE: &str = "Binary response body";
const MAX_COMPARED_BYTES: usize = 2 * BYTES_PER_MEGABYTE;
const SET_COOKIE: &str = "set-cookie";

pub struct ResponseView {
    pub status: u16,
    pub reason: String,
    pub version: String,
    pub elapsed: Duration,
    pub headers: Vec<Header>,
    pub format: ResponseFormat,
    pub truncated: bool,
    pub report: Option<RulesReport>,
    pretty: bool,
    body: Vec<u8>,
    display: Option<ResponseBuffer>,
    search: BodySearch,
    previous: Option<String>,
    changes: Option<Option<LineDiff>>,
}

impl ResponseView {
    pub fn from_response(response: HttpResponse, detect_format: bool) -> Self {
        let format = if detect_format {
            response.format()
        } else {
            ResponseFormat::Text
        };
        ResponseView {
            status: response.status,
            reason: response.reason,
            version: response.version,
            elapsed: response.elapsed,
            headers: response.headers,
            format,
            truncated: response.truncated,
            report: None,
            pretty: true,
            body: response.body,
            display: None,
            search: BodySearch::default(),
            previous: None,
            changes: None,
        }
    }

    pub fn with_previous(mut self, previous: Option<String>) -> Self {
        self.previous = previous;
        self
    }

    pub fn has_previous(&self) -> bool {
        self.previous.is_some()
    }

    pub fn comparison_text(&self) -> Option<String> {
        if self.format == ResponseFormat::Binary || self.body.len() > MAX_COMPARED_BYTES {
            return None;
        }
        let text = String::from_utf8_lossy(&self.body);
        Some(if self.format.has_layout() {
            self.format.laid_out(&text)
        } else {
            text.into_owned()
        })
    }

    pub fn changes(&mut self) -> Option<&LineDiff> {
        if self.changes.is_none() {
            let current = self.comparison_text();
            let diff = self
                .previous
                .as_deref()
                .zip(current.as_deref())
                .and_then(|(before, after)| LineDiff::between(before, after));
            self.changes = Some(diff);
        }
        self.changes.as_ref().and_then(Option::as_ref)
    }

    pub fn with_report(mut self, report: Option<RulesReport>) -> Self {
        self.report = report;
        self
    }

    pub fn has_failed_checks(&self) -> bool {
        self.report
            .as_ref()
            .is_some_and(|report| !report.all_passed())
    }

    pub fn status_class(&self) -> StatusClass {
        StatusClass::of(self.status)
    }

    #[cfg(test)]
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    pub fn size_bytes(&self) -> usize {
        self.body.len()
    }

    pub fn is_pretty(&self) -> bool {
        self.pretty
    }

    pub fn set_pretty(&mut self, pretty: bool) {
        if self.pretty != pretty {
            self.pretty = pretty;
            self.display = None;
            self.search.invalidate();
        }
    }

    pub fn display(&mut self) -> &ResponseBuffer {
        if self.display.is_none() {
            self.display = Some(ResponseBuffer::from_text(self.display_text()));
        }
        self.display
            .get_or_insert_with(|| ResponseBuffer::from_text(String::new()))
    }

    pub fn release_display(&mut self) {
        self.display = None;
        self.search.invalidate();
    }

    pub fn searched_display(&mut self) -> (&ResponseBuffer, &mut BodySearch) {
        self.display();
        let buffer = self.display.as_ref().expect("display was just built");
        self.search.refresh(buffer);
        (buffer, &mut self.search)
    }

    #[cfg(test)]
    pub fn holds_display(&self) -> bool {
        self.display.is_some()
    }

    pub fn cookies(&self) -> impl Iterator<Item = &str> {
        self.headers
            .iter()
            .filter(|header| header.is_named(SET_COOKIE))
            .map(|header| header.value.as_str())
    }

    fn display_text(&self) -> String {
        match self.format {
            ResponseFormat::Binary => BINARY_NOTICE.to_string(),
            format if self.pretty && format.has_layout() => {
                format.laid_out(&String::from_utf8_lossy(&self.body))
            }
            _ => String::from_utf8_lossy(&self.body).into_owned(),
        }
    }
}

#[cfg(test)]
mod tests;
