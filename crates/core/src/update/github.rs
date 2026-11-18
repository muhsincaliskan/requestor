mod latest_release;

use latest_release::LatestReleaseResponse;

use super::{Release, ReleaseClient, UpdateError};
use crate::header::Header;
use crate::prepared::PreparedRequest;
use crate::response::StatusClass;
use crate::transport::HttpTransport;

pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/muhsincaliskan/requestor/releases/latest";
const ACCEPT: &str = "Accept";
const GITHUB_JSON: &str = "application/vnd.github+json";

pub struct GithubReleaseClient<T> {
    transport: T,
    url: String,
}

impl<T: HttpTransport> GithubReleaseClient<T> {
    pub fn new(transport: T, url: impl Into<String>) -> Self {
        GithubReleaseClient {
            transport,
            url: url.into(),
        }
    }
}

impl<T: HttpTransport> ReleaseClient for GithubReleaseClient<T> {
    fn latest_release(&self) -> Result<Release, UpdateError> {
        let request = PreparedRequest::get(&self.url, vec![Header::new(ACCEPT, GITHUB_JSON)]);
        let response = self
            .transport
            .send(request)
            .map_err(UpdateError::Unreachable)?;
        if StatusClass::of(response.status) != StatusClass::Success {
            return Err(UpdateError::Status(response.status));
        }
        serde_json::from_slice::<LatestReleaseResponse>(&response.body)
            .map_err(|_| UpdateError::InvalidResponse)?
            .to_domain()
    }
}
