mod auth;
mod body;
mod paste;
mod row;
mod url_sync;

pub use auth::{AuthDraft, AuthKind};
pub use body::{BodyDraft, BodyKind, RawLanguage};
pub use row::KeyValueRow;

use crate::collection::{Request, Url, UrlParts};
use crate::method::HttpMethod;
use row::{params_from_rows, rows_from_params};

#[derive(Debug, Clone, PartialEq)]
pub struct RequestDraft {
    pub name: String,
    pub method: String,
    pub url: String,
    pub query: Vec<KeyValueRow>,
    pub path_variables: Vec<KeyValueRow>,
    pub headers: Vec<KeyValueRow>,
    pub body: BodyDraft,
    pub auth: AuthDraft,
    source: Request,
}

impl RequestDraft {
    pub fn blank(name: impl Into<String>) -> Self {
        let source = Request::new(HttpMethod::Get.as_str(), Url::Raw(String::new()));
        RequestDraft::from_request(name, &source)
    }

    pub fn from_request(name: impl Into<String>, request: &Request) -> Self {
        let url = request.url.as_ref().map(Url::to_text).unwrap_or_default();
        let mut draft = RequestDraft {
            name: name.into(),
            method: request.http_method().as_str().to_string(),
            url,
            query: Vec::new(),
            path_variables: Vec::new(),
            headers: rows_from_params(request.header.as_ref()),
            body: BodyDraft::from_body(request.body.as_ref()),
            auth: AuthDraft::from_auth(request.auth.as_ref()),
            source: request.clone(),
        };
        draft.load_url_tables(request.url.as_ref());
        draft
    }

    pub fn active_param_count(&self) -> usize {
        KeyValueRow::active_count(&self.query) + KeyValueRow::active_count(&self.path_variables)
    }

    pub fn active_header_count(&self) -> usize {
        KeyValueRow::active_count(&self.headers)
    }

    pub fn to_request(&self) -> Request {
        if self.is_unchanged() {
            return self.source.clone();
        }
        let mut request = self.source.clone();
        request.method = Some(self.method.trim().to_ascii_uppercase());
        request.url = self.url_model();
        let headers = params_from_rows(&self.headers);
        request.header = (!headers.is_empty()).then_some(headers);
        request.body = self.body.to_body();
        request.auth = self.auth.to_auth();
        request
    }

    pub fn commit(&mut self) -> Request {
        let request = self.to_request();
        self.source = request.clone();
        request
    }

    pub fn is_unchanged(&self) -> bool {
        RequestDraft::from_request(self.name.clone(), &self.source) == *self
    }

    fn url_model(&self) -> Option<Url> {
        if self.url.trim().is_empty() {
            return None;
        }
        let mut parts = UrlParts::parse(self.url.trim());
        if let Some(Url::Parts(source)) = &self.source.url {
            parts.extra = source.extra.clone();
        }
        let query = params_from_rows(&self.query);
        parts.query = (!query.is_empty()).then_some(query);
        let variables: Vec<_> = self
            .path_variables
            .iter()
            .map(KeyValueRow::to_variable)
            .collect();
        parts.variable = (!variables.is_empty()).then_some(variables);
        Some(Url::Parts(Box::new(parts)))
    }
}

#[cfg(test)]
mod tests;
