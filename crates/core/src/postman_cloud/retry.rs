use std::time::Duration;

use crate::response::HttpResponse;

pub(super) const TOO_MANY_REQUESTS: u16 = 429;
pub(super) const MAX_RATE_LIMIT_RETRIES: usize = 3;
const WAIT_HEADERS: [&str; 3] = ["Retry-After", "X-RateLimit-RetryAfter", "RateLimit-Reset"];
const DEFAULT_WAIT: Duration = Duration::from_secs(10);
const LONGEST_WAIT: Duration = Duration::from_secs(60);

pub(super) struct RetryWait;

impl RetryWait {
    pub(super) fn after(response: &HttpResponse) -> Option<Duration> {
        let announced = WAIT_HEADERS
            .iter()
            .find_map(|name| response.header(name)?.trim().parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_WAIT);
        (announced <= LONGEST_WAIT).then_some(announced)
    }
}
