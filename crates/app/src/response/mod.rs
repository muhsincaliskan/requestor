mod buffer;
mod comparison;
mod inspection;
mod search;

pub use buffer::{ResponseBuffer, BYTES_PER_MEGABYTE};
pub use comparison::{ChangesUnavailable, ComparisonBase};
pub use inspection::{BodyLayout, JsonInspection, JsonTreeView};

pub use search::BodySearch;

use std::time::Duration;

use requestor_core::checks::RulesReport;
use requestor_core::event_stream::EventLog;
use requestor_core::header::Header;
use requestor_core::response::{HttpResponse, ResponseFormat, StatusClass};
use requestor_core::text_diff::LineDiff;

const BINARY_NOTICE: &str = "Binary response body";
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
    layout: BodyLayout,
    inspection: JsonInspection,
    body: Vec<u8>,
    display: Option<ResponseBuffer>,
    search: BodySearch,
    previous: Option<ComparisonBase>,
    changes: Option<Result<LineDiff, ChangesUnavailable>>,
    events: Option<EventLog>,
}

impl ResponseView {
    pub fn from_response(response: HttpResponse, detect_format: bool) -> Self {
        let format = if detect_format {
            response.format()
        } else {
            ResponseFormat::Text
        };
        let events = response
            .is_event_stream()
            .then(|| EventLog::from_body(&response.body));
        ResponseView {
            status: response.status,
            reason: response.reason,
            version: response.version,
            elapsed: response.elapsed,
            headers: response.headers,
            format,
            truncated: response.truncated,
            report: None,
            layout: BodyLayout::Pretty,
            inspection: JsonInspection::default(),
            body: response.body,
            display: None,
            search: BodySearch::default(),
            previous: None,
            changes: None,
            events,
        }
    }

    pub fn is_storable_as_text(&self) -> bool {
        !self.truncated
            && self.format != ResponseFormat::Binary
            && std::str::from_utf8(&self.body).is_ok()
    }

    pub fn to_response(&self) -> HttpResponse {
        HttpResponse {
            status: self.status,
            reason: self.reason.clone(),
            version: self.version.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            truncated: self.truncated,
            elapsed: self.elapsed,
        }
    }

    pub fn events(&self) -> Option<&EventLog> {
        self.events.as_ref()
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

    pub fn layout(&self) -> BodyLayout {
        self.layout
    }

    pub fn set_layout(&mut self, layout: BodyLayout) {
        if self.layout.is_pretty() != layout.is_pretty() {
            self.forget_display();
        }
        self.layout = layout;
    }

    pub fn is_inspectable(&self) -> bool {
        self.format == ResponseFormat::Json
    }

    pub fn inspection(&self) -> &JsonInspection {
        &self.inspection
    }

    pub fn set_filter(&mut self, text: String) {
        if self.inspection.set_filter(text) {
            self.forget_display();
        }
    }

    pub fn filter_result_count(&mut self) -> Option<usize> {
        self.inspection.result_count(&self.body)
    }

    pub fn json_tree(&mut self) -> Option<JsonTreeView<'_>> {
        self.inspection.tree(&self.body)
    }

    fn forget_display(&mut self) {
        self.display = None;
        self.search.invalidate();
    }

    pub fn display(&mut self) -> &ResponseBuffer {
        if self.display.is_none() {
            let text = self.display_text();
            self.display = Some(ResponseBuffer::from_text(text));
        }
        self.display
            .get_or_insert_with(|| ResponseBuffer::from_text(String::new()))
    }

    pub fn release_display(&mut self) {
        self.forget_display();
        self.inspection.release();
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

    fn display_text(&mut self) -> String {
        let pretty = self.layout.is_pretty();
        if self.is_inspectable() && self.inspection.is_filtering() {
            if let Some(text) = self.inspection.filtered_text(&self.body, pretty) {
                return text;
            }
        }
        match self.format {
            ResponseFormat::Binary => BINARY_NOTICE.to_string(),
            format if pretty && format.has_layout() => {
                format.laid_out(&String::from_utf8_lossy(&self.body))
            }
            _ => String::from_utf8_lossy(&self.body).into_owned(),
        }
    }
}

#[cfg(test)]
mod tests;
