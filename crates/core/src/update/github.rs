mod latest_release;

use latest_release::LatestReleaseResponse;

use super::{Release, ReleaseClient, UpdateError};
use crate::header::{Header, ACCEPT};
use crate::prepared::PreparedRequest;
use crate::response::StatusClass;
use crate::transport::HttpTransport;

pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/muhsincaliskan/requestor/releases/latest";
pub(super) const GITHUB_JSON: &str = "application/vnd.github+json";
const ANY_MEDIA_TYPE: &str = "*/*";

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

impl<T: HttpTransport> GithubReleaseClient<T> {
    fn fetch(&self, url: &str, accept: &str) -> Result<Vec<u8>, UpdateError> {
        let request = PreparedRequest::get(url, vec![Header::new(ACCEPT, accept)]);
        let response = self
            .transport
            .send(request)
            .map_err(UpdateError::Unreachable)?;
        if StatusClass::of(response.status) != StatusClass::Success {
            return Err(UpdateError::Status(response.status));
        }
        if response.truncated {
            return Err(UpdateError::Incomplete);
        }
        Ok(response.body)
    }
}

impl<T: HttpTransport> ReleaseClient for GithubReleaseClient<T> {
    fn latest_release(&self) -> Result<Release, UpdateError> {
        let body = self.fetch(&self.url, GITHUB_JSON)?;
        serde_json::from_slice::<LatestReleaseResponse>(&body)
            .map_err(|_| UpdateError::InvalidResponse)?
            .to_domain()
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(url, ANY_MEDIA_TYPE)
    }
}
