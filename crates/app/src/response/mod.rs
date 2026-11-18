mod buffer;
mod comparison;
mod search;

pub use buffer::{ResponseBuffer, BYTES_PER_MEGABYTE};
pub use comparison::{ChangesUnavailable, ComparisonBase};

use comparison::comparable_text;
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
    previous: Option<ComparisonBase>,
    changes: Option<Result<LineDiff, ChangesUnavailable>>,
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

    pub fn with_previous(mut self, previous: Option<ComparisonBase>) -> Self {
        self.previous = previous;
        self
    }

    pub fn has_previous(&self) -> bool {
        self.previous.is_some() || self.changes.is_some()
    }

    pub fn into_comparison_base(self) -> Option<ComparisonBase> {
        let comparable = self.format != ResponseFormat::Binary && self.is_small_enough();
        comparable.then(|| ComparisonBase::new(self.body, self.format))
    }

    pub fn changes(&mut self) -> Result<&LineDiff, ChangesUnavailable> {
        if self.changes.is_none() {
            let previous = self.previous.take();
            self.changes = previous.map(|previous| self.compared_with(&previous));
        }
        match &self.changes {
            Some(Ok(diff)) => Ok(diff),
            Some(Err(reason)) => Err(*reason),
            None => Err(ChangesUnavailable::TooLarge),
        }
    }

    fn compared_with(&self, previous: &ComparisonBase) -> Result<LineDiff, ChangesUnavailable> {
        if self.format == ResponseFormat::Binary {
            return Err(ChangesUnavailable::Binary);
        }
        if !self.is_small_enough() {
            return Err(ChangesUnavailable::TooLarge);
        }
        let current = comparable_text(&self.body, self.format);
        LineDiff::between(&previous.text(), &current).ok_or(ChangesUnavailable::TooLarge)
    }

    fn is_small_enough(&self) -> bool {
        self.body.len() <= MAX_COMPARED_BYTES
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
