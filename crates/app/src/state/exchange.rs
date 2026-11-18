use requestor_core::history::HistoryEntry;

use super::socket::SocketSession;
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
    Socket(Box<SocketSession>),
}

impl Exchange {
    pub fn start_send(&mut self, job: u64, record: HistoryEntry) -> Option<u64> {
        let (replaced, previous) = match std::mem::replace(self, Exchange::Idle) {
            Exchange::Completed(view) => (None, view.into_comparison_base()),
            Exchange::Sending { job, previous, .. } => (Some(job), previous),
            Exchange::Socket(session) => (session.live_job(), None),
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

    pub fn start_socket(&mut self, session: SocketSession) -> Option<u64> {
        match std::mem::replace(self, Exchange::Socket(Box::new(session))) {
            Exchange::Sending { job, .. } => Some(job),
            Exchange::Socket(previous) => previous.live_job(),
            Exchange::Idle | Exchange::Completed(_) | Exchange::Failed(_) => None,
        }
    }

    pub fn socket_mut(&mut self, job: u64) -> Option<&mut SocketSession> {
        match self {
            Exchange::Socket(session) if session.job() == job => Some(session),
            _ => None,
        }
    }

    pub fn live_socket(&mut self) -> Option<&mut SocketSession> {
        match self {
            Exchange::Socket(session) if session.is_live() => Some(session),
            _ => None,
        }
    }

    pub fn open_socket(&self) -> Option<&SocketSession> {
        match self {
            Exchange::Socket(session) if session.is_open() => Some(session),
            _ => None,
        }
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

    pub fn stream(&mut self, update_job: u64, update: StreamUpdate) -> bool {
        let Exchange::Sending { job, live, .. } = self else {
            return false;
        };
        if *job != update_job {
            return false;
        }
        match update {
            StreamUpdate::Opened(head) => {
                *live = Some(Box::new(LiveStream::open(*head)));
                true
            }
            StreamUpdate::Received(bytes) => {
                if let Some(live) = live {
                    live.receive(&bytes);
                }
                false
            }
        }
    }

    pub fn take_live(&mut self, finished_job: u64) -> Option<LiveStream> {
        match self {
            Exchange::Sending { job, live, .. } if *job == finished_job => {
                live.take().map(|live| *live)
            }
            _ => None,
        }
    }

    pub fn stop_live(&mut self) -> Option<(u64, LiveStream)> {
        match self {
            Exchange::Sending { job, live, .. } => live.take().map(|live| (*job, *live)),
            _ => None,
        }
    }
}
