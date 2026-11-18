use crate::settings::RequestSettings;

const CHECK_TIMEOUT_MS: u64 = 15_000;
const CHECK_CONNECT_TIMEOUT_MS: u64 = 10_000;

impl RequestSettings {
    pub fn for_update_check(&self) -> Self {
        RequestSettings {
            verify_ssl: true,
            follow_redirects: true,
            request_timeout_ms: CHECK_TIMEOUT_MS,
            connect_timeout_ms: CHECK_CONNECT_TIMEOUT_MS,
            ..self.clone()
        }
    }
}
