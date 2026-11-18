use requestor_core::response::ResponseFormat;

pub struct ComparisonBase {
    body: Vec<u8>,
    format: ResponseFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesUnavailable {
    Binary,
    TooLarge,
}

impl ComparisonBase {
    pub(super) fn new(body: Vec<u8>, format: ResponseFormat) -> Self {
        ComparisonBase { body, format }
    }

    pub(super) fn text(&self) -> String {
        comparable_text(&self.body, self.format)
    }
}

pub(super) fn comparable_text(body: &[u8], format: ResponseFormat) -> String {
    let text = String::from_utf8_lossy(body);
    if format.has_layout() {
        return format.laid_out(&text);
    }
    text.into_owned()
}
