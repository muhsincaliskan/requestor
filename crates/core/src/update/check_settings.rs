use crate::settings::RequestSettings;

const CHECK_TIMEOUT_MS: u64 = 15_000;
const CHECK_CONNECT_TIMEOUT_MS: u64 = 10_000;
const DOWNLOAD_TIMEOUT_MS: u64 = 600_000;
const MAX_DOWNLOAD_BYTES: usize = 512 * 1024 * 1024;

impl RequestSettings {
    pub fn for_update_check(&self) -> Self {
        RequestSettings {
            verify_ssl: true,
            follow_redirects: false,
            request_timeout_ms: CHECK_TIMEOUT_MS,
            connect_timeout_ms: CHECK_CONNECT_TIMEOUT_MS,
            client_certificates: Vec::new(),
            ..self.clone()
        }
    }

    pub fn for_update_download(&self) -> Self {
        RequestSettings {
            request_timeout_ms: DOWNLOAD_TIMEOUT_MS,
            max_response_bytes: MAX_DOWNLOAD_BYTES,
            follow_redirects: true,
            ..self.for_update_check()
        }
    }
}
