mod latest_release;

use latest_release::LatestReleaseRedirect;

use super::{Release, ReleaseClient, UpdateError};
use crate::header::{Header, ACCEPT, LOCATION};
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
    fn get(&self, url: &str) -> Result<HttpResponse, UpdateError> {
        let request = PreparedRequest::get(url, vec![Header::new(ACCEPT, ANY_MEDIA_TYPE)]);
        self.transport
            .send(request)
            .map_err(UpdateError::Unreachable)
    }

    fn expect(response: &HttpResponse, class: StatusClass) -> Result<(), UpdateError> {
        if response.status_class() == class {
            Ok(())
        } else {
            Err(UpdateError::Status(response.status))
        }
    }
}

impl<T: HttpTransport> ReleaseClient for GithubReleaseClient<T> {
    fn latest_release(&self) -> Result<Release, UpdateError> {
        let response = self.get(&self.url)?;
        Self::expect(&response, StatusClass::Redirect)?;
        LatestReleaseRedirect::new(response.header(LOCATION)).to_domain()
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        let response = self.get(url)?;
        Self::expect(&response, StatusClass::Success)?;
        if response.truncated {
            return Err(UpdateError::Incomplete);
        }
        Ok(response.body)
    }
}
