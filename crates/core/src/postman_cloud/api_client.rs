use std::thread;
use std::time::Duration;

use serde::de::DeserializeOwned;

use super::responses::{
    CollectionResponse, EnvironmentResponse, WorkspaceResponse, WorkspacesResponse,
};
use super::retry::{RetryWait, MAX_RATE_LIMIT_RETRIES, TOO_MANY_REQUESTS};
use super::{CloudWorkspace, PostmanCloudClient, PostmanCloudError, WorkspaceContents};
use crate::collection::Collection;
use crate::environment::Environment;
use crate::header::{Header, ACCEPT};
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::secret::Secret;
use crate::transport::HttpTransport;

pub const POSTMAN_API_URL: &str = "https://api.getpostman.com";
const API_KEY_HEADER: &str = "X-Api-Key";
const JSON: &str = "application/json";
const UNAUTHORIZED: u16 = 401;
const FORBIDDEN: u16 = 403;
const NOT_FOUND: u16 = 404;
const WORKSPACES_PATH: &str = "/workspaces";
const COLLECTIONS_PATH: &str = "/collections";
const ENVIRONMENTS_PATH: &str = "/environments";

type Pause = Box<dyn Fn(Duration) + Send + Sync>;

pub struct PostmanApiClient<T> {
    transport: T,
    base_url: String,
    key: Secret,
    pause: Pause,
}

impl<T: HttpTransport> PostmanApiClient<T> {
    pub fn new(transport: T, base_url: impl Into<String>, key: Secret) -> Self {
        PostmanApiClient {
            transport,
            base_url: base_url.into(),
            key,
            pause: Box::new(thread::sleep),
        }
    }

    pub fn pausing_with(mut self, pause: impl Fn(Duration) + Send + Sync + 'static) -> Self {
        self.pause = Box::new(pause);
        self
    }

    fn fetch<R: DeserializeOwned>(&self, path: &str) -> Result<R, PostmanCloudError> {
        let response = self.send_with_retries(&format!("{}{path}", self.base_url))?;
        match response.status {
            UNAUTHORIZED | FORBIDDEN => return Err(PostmanCloudError::KeyRejected),
            NOT_FOUND => return Err(PostmanCloudError::NotFound),
            TOO_MANY_REQUESTS => return Err(PostmanCloudError::RateLimited),
            _ if !response.is_success() => return Err(PostmanCloudError::Status(response.status)),
            _ => {}
        }
        serde_json::from_slice(&response.body)
            .map_err(|error| PostmanCloudError::InvalidResponse(error.to_string()))
    }

    fn send_with_retries(&self, url: &str) -> Result<HttpResponse, PostmanCloudError> {
        let mut retries = 0;
        loop {
            let response = self
                .transport
                .send(self.request(url))
                .map_err(PostmanCloudError::Unreachable)?;
            let wait = (response.status == TOO_MANY_REQUESTS && retries < MAX_RATE_LIMIT_RETRIES)
                .then(|| RetryWait::after(&response))
                .flatten();
            match wait {
                Some(wait) => {
                    retries += 1;
                    (self.pause)(wait);
                }
                None => return Ok(response),
            }
        }
    }

    fn request(&self, url: &str) -> PreparedRequest {
        PreparedRequest::get(
            url,
            vec![
                Header::new(API_KEY_HEADER, self.key.expose()),
                Header::new(ACCEPT, JSON),
            ],
        )
    }
}

impl<T: HttpTransport> PostmanCloudClient for PostmanApiClient<T> {
    fn workspaces(&self) -> Result<Vec<CloudWorkspace>, PostmanCloudError> {
        self.fetch::<WorkspacesResponse>(WORKSPACES_PATH)
            .map(WorkspacesResponse::into_domain)
    }

    fn workspace(&self, id: &str) -> Result<WorkspaceContents, PostmanCloudError> {
        let path = format!("{WORKSPACES_PATH}/{}", path_segment(id)?);
        self.fetch::<WorkspaceResponse>(&path)
            .map(WorkspaceResponse::into_domain)
    }

    fn collection(&self, uid: &str) -> Result<Collection, PostmanCloudError> {
        let path = format!("{COLLECTIONS_PATH}/{}", path_segment(uid)?);
        self.fetch::<CollectionResponse>(&path)?.into_domain()
    }

    fn environment(&self, uid: &str) -> Result<Environment, PostmanCloudError> {
        let path = format!("{ENVIRONMENTS_PATH}/{}", path_segment(uid)?);
        self.fetch::<EnvironmentResponse>(&path)?.into_domain()
    }
}

fn path_segment(id: &str) -> Result<&str, PostmanCloudError> {
    let safe = !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-');
    safe.then_some(id)
        .ok_or_else(|| PostmanCloudError::InvalidIdentifier(id.to_string()))
}
