const NOTICE_LIFETIME_SECONDS: f64 = 6.0;
const MAX_NOTICES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub level: NoticeLevel,
    pub text: String,
    pub raised_at: f64,
}

#[derive(Debug, Default)]
pub struct Notices {
    entries: Vec<Notice>,
}

impl Notices {
    pub fn raise(&mut self, level: NoticeLevel, text: impl Into<String>, now: f64) {
        self.entries.push(Notice {
            level,
            text: text.into(),
            raised_at: now,
        });
        let overflow = self.entries.len().saturating_sub(MAX_NOTICES);
        self.entries.drain(..overflow);
    }

    pub fn current(&self, now: f64) -> Option<&Notice> {
        self.entries
            .last()
            .filter(|notice| now - notice.raised_at < NOTICE_LIFETIME_SECONDS)
    }

    pub fn expires_in(&self, now: f64) -> Option<f64> {
        self.current(now)
            .map(|notice| NOTICE_LIFETIME_SECONDS - (now - notice.raised_at))
    }

    pub fn dismiss(&mut self) {
        self.entries.clear();
    }
}
