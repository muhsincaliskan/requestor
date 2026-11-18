use std::collections::VecDeque;

use super::{EventStreamParser, ServerEvent};

pub const MAX_KEPT_EVENTS: usize = 1000;

#[derive(Debug, Default)]
pub struct EventLog {
    parser: EventStreamParser,
    recent: VecDeque<ServerEvent>,
    total: usize,
}

impl EventLog {
    pub fn from_body(body: &[u8]) -> Self {
        let mut log = EventLog::default();
        log.receive(body);
        log
    }

    pub fn receive(&mut self, bytes: &[u8]) {
        for event in self.parser.push(bytes) {
            self.keep(event);
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn kept(&self) -> usize {
        self.recent.len()
    }

    pub fn dropped(&self) -> usize {
        self.total - self.recent.len()
    }

    pub fn number_of(&self, index: usize) -> usize {
        self.dropped() + index + 1
    }

    pub fn get(&self, index: usize) -> Option<&ServerEvent> {
        self.recent.get(index)
    }

    fn keep(&mut self, event: ServerEvent) {
        self.total += 1;
        if self.recent.len() == MAX_KEPT_EVENTS {
            self.recent.pop_front();
        }
        self.recent.push_back(event);
    }
}
