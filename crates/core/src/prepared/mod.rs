mod auth;
mod body;
mod multipart;

use crate::collection::{Param, Request, Url};
use crate::error::CoreError;
use crate::header::{find_header, Header};
use crate::identity::random_uuid;
use crate::method::HttpMethod;
use crate::settings::RequestSettings;
use crate::variables::VariableScope;

use body::EncodedBody;

const CONTENT_TYPE: &str = "Content-Type";
const CACHE_CONTROL: &str = "Cache-Control";
const NO_CACHE: &str = "no-cache";
const POSTMAN_TOKEN: &str = "Postman-Token";

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<Vec<u8>>,
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
            headers: resolved_headers(request, scope),
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

    fn apply_body(&mut self, encoded: Option<EncodedBody>) {
        let Some(encoded) = encoded else { return };
        if let Some(content_type) = encoded.content_type {
            self.add_header_if_absent(CONTENT_TYPE, content_type);
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
}

fn resolved_headers(request: &Request, scope: &VariableScope) -> Vec<Header> {
    request
        .enabled_headers()
        .into_iter()
        .filter_map(|param| resolved_header(param, scope))
        .collect()
}

fn resolved_header(param: &Param, scope: &VariableScope) -> Option<Header> {
    let name = scope.resolve(param.key.as_deref()?).text;
    let value = scope
        .resolve(param.value.as_deref().unwrap_or_default())
        .text;
    Some(Header::new(name, value))
}
