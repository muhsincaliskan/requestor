use super::{AppState, Effect};

impl AppState {
    pub(super) fn release_page(&self) -> Option<Effect> {
        let release = self.available_update.as_ref()?;
        Some(Effect::OpenUrl(release.page_url.clone()))
    }
}
