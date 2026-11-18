use super::super::exposure::SecretExposure;
use super::super::multipart::BOUNDARY_PREFIX;
use super::super::{BodySource, FormField, FormValue, PreparedRequest};
use super::form_part::FormPart;
use crate::header::{Header, CONTENT_TYPE};
use crate::media_type;
use crate::method::HttpMethod;

pub(super) struct Snippet {
    pub(super) method: HttpMethod,
    pub(super) url: String,
    pub(super) headers: Vec<Header>,
    pub(super) body: SnippetBody,
}

pub(super) enum SnippetBody {
    Empty,
    Text(String),
    File(String),
    Form {
        boundary: String,
        fields: Vec<FormField>,
    },
}

impl Snippet {
    pub(super) fn new(request: &PreparedRequest, exposure: SecretExposure) -> Self {
        Snippet {
            method: request.method.clone(),
            url: exposure.url(&request.url),
            headers: request
                .headers
                .iter()
                .map(|header| Header::new(header.name.clone(), exposure.header_value(header)))
                .collect(),
            body: SnippetBody::new(request, exposure),
        }
    }

    pub(super) fn method_name(&self) -> &str {
        self.method.as_str()
    }

    pub(super) fn headers_without(&self, unsupported: &[&str]) -> Vec<Header> {
        self.headers
            .iter()
            .filter(|header| !unsupported.iter().any(|name| header.is_named(name)))
            .cloned()
            .collect()
    }

    pub(super) fn library_headers(&self, unsupported: &[&str]) -> Vec<Header> {
        let mut merged: Vec<Header> = Vec::new();
        for header in self.headers_without(unsupported) {
            if self.is_form() && header.is_named(CONTENT_TYPE) {
                continue;
            }
            match merged.iter_mut().find(|known| known.is_named(&header.name)) {
                Some(known) => known.append_value(&header.value),
                None => merged.push(header),
            }
        }
        merged
    }

    pub(super) fn has_files(&self) -> bool {
        match &self.body {
            SnippetBody::File(_) => true,
            SnippetBody::Form { fields, .. } => fields
                .iter()
                .any(|field| matches!(field.value, FormValue::File { .. })),
            SnippetBody::Empty | SnippetBody::Text(_) => false,
        }
    }

    pub(super) fn form_parts(&self, line_break: &str) -> Vec<FormPart> {
        match &self.body {
            SnippetBody::Form { boundary, fields } => {
                FormPart::layout(fields, boundary, line_break)
            }
            _ => Vec::new(),
        }
    }

    fn is_form(&self) -> bool {
        matches!(self.body, SnippetBody::Form { .. })
    }
}

impl SnippetBody {
    fn new(request: &PreparedRequest, exposure: SecretExposure) -> Self {
        match (&request.body_source, &request.body) {
            (BodySource::File(path), _) => SnippetBody::File(exposure.text(path)),
            (BodySource::Form(fields), _) => SnippetBody::Form {
                boundary: request
                    .header(CONTENT_TYPE)
                    .and_then(|content_type| {
                        media_type::parameter(content_type, media_type::BOUNDARY_PARAMETER)
                    })
                    .unwrap_or(BOUNDARY_PREFIX)
                    .to_string(),
                fields: fields
                    .iter()
                    .map(|field| exposure.form_field(field))
                    .collect(),
            },
            (BodySource::Inline, Some(body)) if !body.is_empty() => SnippetBody::Text(
                exposure.inline_body(&String::from_utf8_lossy(body), request.header(CONTENT_TYPE)),
            ),
            (BodySource::Inline, _) => SnippetBody::Empty,
        }
    }
}
