use std::collections::VecDeque;

use super::SocketMessage;
use crate::settings::MAX_SOCKET_MESSAGE_BYTES;

pub const MAX_KEPT_MESSAGES: usize = 1000;
pub const MAX_KEPT_MESSAGE_BYTES: usize = MAX_SOCKET_MESSAGE_BYTES;

#[derive(Debug, Default)]
pub struct SocketLog {
    recent: VecDeque<SocketMessage>,
    kept_bytes: usize,
    total: usize,
}

impl SocketLog {
    pub fn push(&mut self, message: SocketMessage) {
        self.total += 1;
        self.kept_bytes += message.size();
        self.recent.push_back(message);
        while self.is_over_budget() {
            if let Some(oldest) = self.recent.pop_front() {
                self.kept_bytes -= oldest.size();
            }
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

    pub fn get(&self, index: usize) -> Option<&SocketMessage> {
        self.recent.get(index)
    }

    fn is_over_budget(&self) -> bool {
        self.recent.len() > 1
            && (self.recent.len() > MAX_KEPT_MESSAGES || self.kept_bytes > MAX_KEPT_MESSAGE_BYTES)
    }
}
