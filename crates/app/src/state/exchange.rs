use requestor_core::history::HistoryEntry;

use super::stream::{LiveStream, StreamUpdate};
use crate::response::{ComparisonBase, ResponseView};

pub enum Exchange {
    Idle,
    Sending {
        job: u64,
        previous: Option<ComparisonBase>,
        record: Option<Box<HistoryEntry>>,
        live: Option<Box<LiveStream>>,
    },
    Completed(Box<ResponseView>),
    Failed(String),
}

impl Exchange {
    pub fn start_send(&mut self, job: u64, record: HistoryEntry) -> Option<u64> {
        let (replaced, previous) = match std::mem::replace(self, Exchange::Idle) {
            Exchange::Completed(view) => (None, view.into_comparison_base()),
            Exchange::Sending { job, previous, .. } => (Some(job), previous),
            Exchange::Idle | Exchange::Failed(_) => (None, None),
        };
        *self = Exchange::Sending {
            job,
            previous,
            record: Some(Box::new(record)),
            live: None,
        };
        replaced
    }

    pub fn take_previous(&mut self) -> Option<ComparisonBase> {
        match self {
            Exchange::Sending { previous, .. } => previous.take(),
            _ => None,
        }
    }

    pub fn take_record(&mut self) -> Option<HistoryEntry> {
        match self {
            Exchange::Sending { record, .. } => record.take().map(|record| *record),
            _ => None,
        }
    }

    pub fn stream(&mut self, update_job: u64, update: StreamUpdate) {
        let Exchange::Sending { job, live, .. } = self else {
            return;
        };
        if *job != update_job {
            return;
        }
        match update {
            StreamUpdate::Opened(head) => *live = Some(Box::new(LiveStream::open(*head))),
            StreamUpdate::Received(bytes) => {
                if let Some(live) = live {
                    live.receive(&bytes);
                }
            }
        }
    }

    pub fn take_live(&mut self) -> Option<(u64, LiveStream)> {
        match self {
            Exchange::Sending { job, live, .. } => live.take().map(|live| (*job, *live)),
            _ => None,
        }
    }
}
