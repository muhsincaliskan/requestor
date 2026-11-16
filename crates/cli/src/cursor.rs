use std::collections::VecDeque;

use crate::error::CliError;

pub struct Cursor {
    items: VecDeque<String>,
}

impl Cursor {
    pub fn new(items: Vec<String>) -> Self {
        Cursor {
            items: items.into(),
        }
    }

    pub fn next_item(&mut self) -> Option<String> {
        self.items.pop_front()
    }

    pub fn drain(&mut self) -> Vec<String> {
        self.items.drain(..).collect()
    }

    pub fn value(&mut self, flag: &str) -> Result<String, CliError> {
        self.next_item()
            .ok_or_else(|| CliError::usage(format!("{flag} needs a value")))
    }

    pub fn number(&mut self, flag: &str) -> Result<u64, CliError> {
        let text = self.value(flag)?;
        text.parse()
            .map_err(|_| CliError::usage(format!("{flag} needs a whole number, got {text:?}")))
    }
}
