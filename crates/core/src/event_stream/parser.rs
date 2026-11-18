use super::ServerEvent;

const FIELD_SEPARATOR: char = ':';
const VALUE_PADDING: char = ' ';
const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const DATA_FIELD: &str = "data";
const EVENT_FIELD: &str = "event";
const ID_FIELD: &str = "id";
const DATA_JOINER: char = '\n';
const MAX_PENDING_BYTES: usize = 1024 * 1024;
const BYTE_ORDER_MARK: char = '\u{FEFF}';
const NULL: char = '\0';

#[derive(Debug, Default)]
pub struct EventStreamParser {
    pending: Vec<u8>,
    data: Option<String>,
    name: Option<String>,
    id: Option<String>,
    after_carriage_return: bool,
    past_first_line: bool,
}

impl EventStreamParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        for &byte in bytes {
            let skip = self.after_carriage_return && byte == LINE_FEED;
            self.after_carriage_return = byte == CARRIAGE_RETURN;
            if skip {
                continue;
            }
            if byte == LINE_FEED || byte == CARRIAGE_RETURN {
                let line = std::mem::take(&mut self.pending);
                events.extend(self.line(&String::from_utf8_lossy(&line)));
            } else if self.pending.len() < MAX_PENDING_BYTES {
                self.pending.push(byte);
            }
        }
        events
    }

    fn line(&mut self, line: &str) -> Option<ServerEvent> {
        let line = if self.past_first_line {
            line
        } else {
            self.past_first_line = true;
            line.strip_prefix(BYTE_ORDER_MARK).unwrap_or(line)
        };
        if line.is_empty() {
            return self.dispatch();
        }
        let (field, value) = match line.split_once(FIELD_SEPARATOR) {
            Some((field, value)) => (field, value.strip_prefix(VALUE_PADDING).unwrap_or(value)),
            None => (line, ""),
        };
        match field {
            DATA_FIELD => self.append_data(value),
            EVENT_FIELD => self.name = Some(value.to_string()),
            ID_FIELD if !value.contains(NULL) => self.id = Some(value.to_string()),
            _ => {}
        }
        None
    }

    fn append_data(&mut self, value: &str) {
        match &mut self.data {
            Some(data) => {
                data.push(DATA_JOINER);
                data.push_str(value);
            }
            None => self.data = Some(value.to_string()),
        }
    }

    fn dispatch(&mut self) -> Option<ServerEvent> {
        let name = self.name.take();
        let data = self.data.take()?;
        Some(ServerEvent {
            name: name.filter(|name| !name.is_empty()),
            data,
            id: self.id.clone(),
        })
    }
}
