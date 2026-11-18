use requestor_core::response::ResponseFormat;
use requestor_core::text_diff::LineDiff;

use super::{ResponseView, BYTES_PER_MEGABYTE};

const MAX_COMPARED_BYTES: usize = 2 * BYTES_PER_MEGABYTE;

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

impl ResponseView {
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
        self.size_bytes() <= MAX_COMPARED_BYTES
    }
}
