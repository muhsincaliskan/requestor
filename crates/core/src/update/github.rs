mod latest_redirect;

use latest_redirect::LatestReleaseRedirect;

use super::{Release, ReleaseClient, UpdateError};
use crate::header::{Header, ACCEPT};
use crate::prepared::PreparedRequest;
use crate::response::{HttpResponse, StatusClass};
use crate::transport::HttpTransport;

pub const LATEST_RELEASE_URL: &str = "https://github.com/muhsincaliskan/requestor/releases/latest";
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
    fn send(&self, url: &str) -> Result<HttpResponse, UpdateError> {
        let request = PreparedRequest::get(url, vec![Header::new(ACCEPT, ANY_MEDIA_TYPE)]);
        self.transport
            .send(request)
            .map_err(UpdateError::Unreachable)
    }

    fn fetch(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        let response = self.send(url)?;
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
        LatestReleaseRedirect::from_response(&self.send(&self.url)?)?.to_domain()
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        self.fetch(url)
    }
}
