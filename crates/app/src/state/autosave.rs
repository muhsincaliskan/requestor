use std::collections::BTreeSet;

pub const QUIET_SECONDS: f64 = 0.8;

#[derive(Debug, Default)]
pub struct UnsavedChanges {
    pub collections: BTreeSet<String>,
    pub environments: BTreeSet<String>,
    pub settings: bool,
    pub session: bool,
    changed_at: Option<f64>,
}

impl UnsavedChanges {
    pub fn touch_collection(&mut self, id: &str, now: f64) {
        self.collections.insert(id.to_string());
        self.changed_at = Some(now);
    }

    pub fn touch_environment(&mut self, id: &str, now: f64) {
        self.environments.insert(id.to_string());
        self.changed_at = Some(now);
    }

    pub fn touch_settings(&mut self, now: f64) {
        self.settings = true;
        self.changed_at = Some(now);
    }

    pub fn touch_session(&mut self, now: f64) {
        self.session = true;
        self.changed_at = Some(now);
    }

    pub fn forget_collection(&mut self, id: &str) {
        self.collections.remove(id);
    }

    pub fn forget_environment(&mut self, id: &str) {
        self.environments.remove(id);
    }

    pub fn is_empty(&self) -> bool {
        self.collections.is_empty()
            && self.environments.is_empty()
            && !self.settings
            && !self.session
    }

    pub fn is_due(&self, now: f64) -> bool {
        !self.is_empty() && self.seconds_until_due(now).is_some_and(|wait| wait <= 0.0)
    }

    pub fn seconds_until_due(&self, now: f64) -> Option<f64> {
        if self.is_empty() {
            return None;
        }
        self.changed_at
            .map(|changed| (QUIET_SECONDS - (now - changed)).max(0.0))
    }

    pub fn take(&mut self) -> UnsavedChanges {
        std::mem::take(self)
    }
}
