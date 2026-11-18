use reqwest::Url;

use super::loader::DocumentSource;
use super::MAX_DOCUMENT_BYTES;
use crate::prepared::PreparedRequest;
use crate::response::StatusClass;
use crate::settings::BYTES_PER_MEGABYTE;
use crate::transport::HttpTransport;

const WEB_SCHEMES: [&str; 2] = ["http", "https"];
const NOT_A_WEB_ADDRESS: &str = "only http and https addresses are downloaded";
const NOT_TEXT: &str = "the response is not UTF-8 text";

pub struct RemoteSource<'a> {
    transport: &'a dyn HttpTransport,
}

impl<'a> RemoteSource<'a> {
    pub fn new(transport: &'a dyn HttpTransport) -> Self {
        RemoteSource { transport }
    }
}

impl DocumentSource for RemoteSource<'_> {
    fn resolve(&self, base: &str, location: &str) -> String {
        Url::parse(base)
            .and_then(|base| base.join(location))
            .map_or_else(|_| location.to_string(), String::from)
    }

    fn read(&self, location: &str) -> Result<String, String> {
        let url = Url::parse(location).map_err(|error| error.to_string())?;
        if !WEB_SCHEMES.contains(&url.scheme()) {
            return Err(NOT_A_WEB_ADDRESS.to_string());
        }
        let response = self
            .transport
            .send(PreparedRequest::get(url.as_str(), Vec::new()))
            .map_err(|error| error.to_string())?;
        if StatusClass::of(response.status) != StatusClass::Success {
            return Err(format!("the server answered {}", response.status));
        }
        if response.truncated || response.body.len() > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "it is larger than {} MB",
                MAX_DOCUMENT_BYTES / BYTES_PER_MEGABYTE
            ));
        }
        String::from_utf8(response.body).map_err(|_| NOT_TEXT.to_string())
    }
}
