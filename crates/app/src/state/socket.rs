use requestor_core::history::HistoryEntry;
use requestor_core::response::HttpResponse;
use requestor_core::transport::SocketEvent;
use requestor_core::websocket::SocketLog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketPhase {
    Connecting,
    Open,
    Closing,
    Closed(Option<String>),
    Failed(String),
}

pub struct SocketSession {
    job: u64,
    phase: SocketPhase,
    log: SocketLog,
    head: Option<HttpResponse>,
    record: Option<Box<HistoryEntry>>,
}

impl SocketSession {
    pub fn connecting(job: u64, record: HistoryEntry) -> Self {
        SocketSession {
            job,
            phase: SocketPhase::Connecting,
            log: SocketLog::default(),
            head: None,
            record: Some(Box::new(record)),
        }
    }

    pub fn job(&self) -> u64 {
        self.job
    }

    pub fn phase(&self) -> &SocketPhase {
        &self.phase
    }

    pub fn log(&self) -> &SocketLog {
        &self.log
    }

    pub fn head(&self) -> Option<&HttpResponse> {
        self.head.as_ref()
    }

    pub fn is_live(&self) -> bool {
        matches!(
            self.phase,
            SocketPhase::Connecting | SocketPhase::Open | SocketPhase::Closing
        )
    }

    pub fn is_open(&self) -> bool {
        self.phase == SocketPhase::Open
    }

    pub fn live_job(&self) -> Option<u64> {
        self.is_live().then_some(self.job)
    }

    pub fn apply(&mut self, event: SocketEvent) -> Option<HistoryEntry> {
        if !self.is_live() {
            return None;
        }
        match event {
            SocketEvent::Opened(head) => {
                let status = head.status;
                self.head = Some(head);
                if self.phase == SocketPhase::Connecting {
                    self.phase = SocketPhase::Open;
                }
                return self.take_record(Some(status));
            }
            SocketEvent::Message(message) => self.log.push(message),
            SocketEvent::Closed(reason) => self.phase = SocketPhase::Closed(reason),
        }
        None
    }

    pub fn end(&mut self, result: Result<(), String>) -> Option<HistoryEntry> {
        if self.is_live() {
            self.phase = match result {
                Ok(()) => SocketPhase::Closed(None),
                Err(reason) => SocketPhase::Failed(reason),
            };
        }
        self.take_record(None)
    }

    pub fn disconnect(&mut self) -> Option<u64> {
        self.phase = match self.phase {
            SocketPhase::Connecting => SocketPhase::Closed(None),
            SocketPhase::Open => SocketPhase::Closing,
            _ => return None,
        };
        Some(self.job)
    }

    fn take_record(&mut self, status: Option<u16>) -> Option<HistoryEntry> {
        self.record.take().map(|record| record.finished(status))
    }
}
