mod apply;
mod options;
#[cfg(test)]
mod tests;

use super::shell_words;
use super::{ImportError, ImportedRequest};
use crate::collection::{Auth, Body, Param};
use crate::curl_command::COMMAND_NAME;
use crate::header::CONTENT_TYPE;
use crate::media_type;

const ACCEPT: &str = "Accept";
const QUERY_SEPARATOR: &str = "&";
const DATA_CONFLICT_WARNING: &str = "data given together with a file body was ignored";

#[derive(Default)]
struct CurlArguments {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<(String, String)>,
    data: Vec<String>,
    json_data: Vec<String>,
    data_file: Option<String>,
    url_encoded: Vec<Param>,
    form: Vec<Param>,
    user: Option<String>,
    query_data: bool,
    head: bool,
    json: bool,
    warnings: Vec<String>,
}

impl ImportedRequest {
    pub fn from_curl(command: &str) -> Result<Self, ImportError> {
        let words = shell_words::split(command)?;
        let mut arguments = CurlArguments::default();
        let mut remaining = words
            .into_iter()
            .skip_while(|word| word == COMMAND_NAME)
            .peekable();
        while let Some(word) = remaining.next() {
            arguments.consume(&word, &mut remaining);
        }
        arguments.into_imported()
    }
}

impl CurlArguments {
    fn into_imported(mut self) -> Result<ImportedRequest, ImportError> {
        let url = self.url.clone().ok_or(ImportError::MissingUrl)?;
        if self.data_file.is_some() && !self.data_text().is_empty() {
            self.warnings.push(DATA_CONFLICT_WARNING.to_string());
        }
        let method = self.resolved_method();
        let mut imported = ImportedRequest::new(&method, &self.url_with_query_data(url));
        for (name, value) in &self.headers {
            imported.request.push_header(name.as_str(), value.as_str());
        }
        if self.json {
            self.add_json_headers(&mut imported);
        }
        imported.request.body = self.build_body(&imported);
        imported.request.auth = self.user.as_deref().map(Auth::from_credentials);
        imported.warnings = self.warnings;
        Ok(imported)
    }

    fn has_body(&self) -> bool {
        !self.data_text().is_empty()
            || self.data_file.is_some()
            || !self.url_encoded.is_empty()
            || !self.form.is_empty()
    }

    fn resolved_method(&self) -> String {
        if let Some(method) = &self.method {
            return method.clone();
        }
        if self.head {
            "HEAD".to_string()
        } else if self.has_body() && !self.query_data {
            "POST".to_string()
        } else {
            "GET".to_string()
        }
    }

    fn data_text(&self) -> String {
        let mut parts = Vec::new();
        if !self.data.is_empty() {
            parts.push(self.data.join(QUERY_SEPARATOR));
        }
        if !self.json_data.is_empty() {
            parts.push(self.json_data.concat());
        }
        parts.join(QUERY_SEPARATOR)
    }

    fn query_text(&self) -> String {
        let mut pairs = self.data.clone();
        pairs.extend(self.url_encoded.iter().map(Param::form_pair));
        pairs.join(QUERY_SEPARATOR)
    }

    fn url_with_query_data(&self, url: String) -> String {
        let query = self.query_text();
        if !self.query_data || query.is_empty() {
            return url;
        }
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}{query}")
    }

    fn add_json_headers(&self, imported: &mut ImportedRequest) {
        for (name, value) in [(CONTENT_TYPE, media_type::JSON), (ACCEPT, media_type::JSON)] {
            if !imported.request.has_header(name) {
                imported.request.push_header(name, value);
            }
        }
    }

    fn build_body(&self, imported: &ImportedRequest) -> Option<Body> {
        if self.query_data {
            return None;
        }
        if !self.form.is_empty() {
            return Some(Body::formdata(self.form.clone()));
        }
        if let Some(path) = &self.data_file {
            return Some(Body::file(path.clone()));
        }
        let data = self.data_text();
        if !self.url_encoded.is_empty() {
            let mut params = Param::from_form_text(&data).unwrap_or_default();
            params.extend(self.url_encoded.clone());
            return Some(Body::urlencoded(params));
        }
        let content_type = imported.request.header_value(CONTENT_TYPE);
        (!data.is_empty()).then(|| Body::from_data(&data, content_type))
    }
}
