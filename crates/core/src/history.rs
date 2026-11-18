mod age;

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub use age::Age;

use crate::collection::{Request, Url};
use crate::method::HttpMethod;

pub const HISTORY_LIMIT: usize = 100;
pub const MAX_KEPT_BODY_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub name: String,
    pub request: Request,
    pub sent_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub body_left_out: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct History {
    entries: Vec<HistoryEntry>,
}

impl HistoryEntry {
    pub fn new(name: impl Into<String>, mut request: Request) -> Self {
        let body_left_out = request
            .body
            .as_ref()
            .and_then(|body| body.raw.as_ref())
            .is_some_and(|raw| raw.len() > MAX_KEPT_BODY_BYTES);
        if body_left_out {
            request.body = None;
        }
        HistoryEntry {
            name: name.into(),
            request,
            sent_at: unix_seconds(),
            status: None,
            body_left_out,
        }
    }

    pub fn finished(self, status: Option<u16>) -> Self {
        HistoryEntry { status, ..self }
    }

    pub fn age(&self) -> Age {
        Age::between(self.sent_at, unix_seconds())
    }

    pub fn method(&self) -> HttpMethod {
        self.request.http_method()
    }

    pub fn url_text(&self) -> String {
        self.request
            .url
            .as_ref()
            .map(|url| url.to_text())
            .unwrap_or_default()
    }

    pub fn url_without_scheme(&self) -> String {
        let text = self.url_text();
        match Url::split_scheme(&text) {
            Some((_, rest)) => rest.to_string(),
            None => text,
        }
    }

    pub fn matches_filter(&self, filter: &str) -> bool {
        let needle = filter.to_lowercase();
        needle.is_empty()
            || self.name.to_lowercase().contains(&needle)
            || self.url_text().to_lowercase().contains(&needle)
    }

    fn repeats(&self, other: &HistoryEntry) -> bool {
        self.name == other.name && self.request == other.request
    }
}

impl History {
    pub fn record(&mut self, entry: HistoryEntry) {
        match self.entries.first_mut() {
            Some(latest) if latest.repeats(&entry) => *latest = entry,
            _ => self.entries.insert(0, entry),
        }
        self.entries.truncate(HISTORY_LIMIT);
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
