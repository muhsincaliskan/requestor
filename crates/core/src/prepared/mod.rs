mod auth;
mod body;
mod multipart;

use crate::collection::{Request, Url};
use crate::error::CoreError;
use crate::header::{find_header, Header, CONTENT_TYPE};
use crate::identity::random_uuid;
use crate::method::HttpMethod;
use crate::settings::RequestSettings;
use crate::variables::{placeholder_names, VariableScope};

use body::EncodedBody;

const CACHE_CONTROL: &str = "Cache-Control";
const NO_CACHE: &str = "no-cache";
const POSTMAN_TOKEN: &str = "Postman-Token";

#[derive(Clone, PartialEq)]
pub struct PreparedRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<Vec<u8>>,
}

impl std::fmt::Debug for PreparedRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedRequest")
            .field("method", &self.method)
            .field("headers", &self.headers.len())
            .field("body_bytes", &self.body.as_ref().map(Vec::len))
            .finish()
    }
}

impl PreparedRequest {
    pub fn from_request(
        request: &Request,
        scope: &VariableScope,
        settings: &RequestSettings,
    ) -> Result<Self, CoreError> {
        let url = request
            .url
            .as_ref()
            .map(Url::to_text)
            .filter(|text| !text.trim().is_empty())
            .ok_or(CoreError::MissingUrl)?;
        let mut prepared = PreparedRequest {
            method: request.http_method(),
            url: scope.resolve(&url).text,
            headers: request
                .enabled_headers()
                .into_iter()
                .filter_map(|param| Header::from_param(param, scope))
                .collect(),
            body: None,
        };
        if let Some(body) = request.body.as_ref().filter(|body| body.is_enabled()) {
            prepared.apply_body(EncodedBody::from_body(body, scope)?);
        }
        if let Some(authentication) = &request.auth {
            prepared.apply_auth(authentication, scope);
        }
        prepared.add_default_headers(settings);
        Ok(prepared)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name).map(|header| header.value.as_str())
    }

    pub fn unresolved_variables(&self) -> Vec<String> {
        let mut names = placeholder_names(&self.url);
        for header in &self.headers {
            names.extend(placeholder_names(&header.name));
            names.extend(placeholder_names(&header.value));
        }
        if let Some(body) = &self.body {
            names.extend(placeholder_names(&String::from_utf8_lossy(body)));
        }
        names.sort();
        names.dedup();
        names
    }

    fn apply_body(&mut self, encoded: Option<EncodedBody>) {
        let Some(encoded) = encoded else { return };
        match encoded.content_type.clone() {
            Some(content_type) if encoded.is_multipart() => {
                self.set_header(CONTENT_TYPE, content_type)
            }
            Some(content_type) => self.add_header_if_absent(CONTENT_TYPE, content_type),
            None => {}
        }
        self.body = Some(encoded.data);
    }

    fn add_default_headers(&mut self, settings: &RequestSettings) {
        if settings.send_no_cache_header {
            self.add_header_if_absent(CACHE_CONTROL, NO_CACHE.to_string());
        }
        if settings.send_postman_token_header {
            self.add_header_if_absent(POSTMAN_TOKEN, random_uuid());
        }
    }

    pub(crate) fn add_header_if_absent(&mut self, name: &str, value: String) {
        if find_header(&self.headers, name).is_none() {
            self.headers.push(Header::new(name, value));
        }
    }

    fn set_header(&mut self, name: &str, value: String) {
        self.headers.retain(|header| !header.is_named(name));
        self.headers.push(Header::new(name, value));
    }
}
