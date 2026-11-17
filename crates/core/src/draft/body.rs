use serde_json::{json, Map, Value};

use super::row::{params_from_rows, rows_from_params, KeyValueRow};
use crate::collection::{Body, BodyMode};

const OPTIONS_RAW_KEY: &str = "raw";
const OPTIONS_LANGUAGE_KEY: &str = "language";
const FILE_SOURCE_KEY: &str = "src";
const GRAPHQL_QUERY_KEY: &str = "query";
const GRAPHQL_VARIABLES_KEY: &str = "variables";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    #[default]
    None,
    Raw,
    Urlencoded,
    FormData,
    File,
    GraphQl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawLanguage {
    #[default]
    Text,
    Json,
    Xml,
    Html,
    JavaScript,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BodyDraft {
    pub kind: BodyKind,
    pub raw: String,
    pub language: RawLanguage,
    pub urlencoded: Vec<KeyValueRow>,
    pub formdata: Vec<KeyValueRow>,
    pub file_path: String,
    pub graphql_query: String,
    pub graphql_variables: String,
    source: Option<Body>,
}

impl BodyKind {
    pub const ALL: [BodyKind; 6] = [
        BodyKind::None,
        BodyKind::Raw,
        BodyKind::Urlencoded,
        BodyKind::FormData,
        BodyKind::File,
        BodyKind::GraphQl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BodyKind::None => "none",
            BodyKind::Raw => "raw",
            BodyKind::Urlencoded => "x-www-form-urlencoded",
            BodyKind::FormData => "form-data",
            BodyKind::File => "binary",
            BodyKind::GraphQl => "GraphQL",
        }
    }

    fn from_body(body: &Body) -> Self {
        if !body.is_enabled() {
            return BodyKind::None;
        }
        match body.mode {
            Some(BodyMode::Raw) => BodyKind::Raw,
            Some(BodyMode::Urlencoded) => BodyKind::Urlencoded,
            Some(BodyMode::Formdata) => BodyKind::FormData,
            Some(BodyMode::File) => BodyKind::File,
            Some(BodyMode::Graphql) => BodyKind::GraphQl,
            None => BodyKind::None,
        }
    }

    fn mode(self) -> Option<BodyMode> {
        match self {
            BodyKind::None => None,
            BodyKind::Raw => Some(BodyMode::Raw),
            BodyKind::Urlencoded => Some(BodyMode::Urlencoded),
            BodyKind::FormData => Some(BodyMode::Formdata),
            BodyKind::File => Some(BodyMode::File),
            BodyKind::GraphQl => Some(BodyMode::Graphql),
        }
    }
}

impl RawLanguage {
    pub const ALL: [RawLanguage; 5] = [
        RawLanguage::Text,
        RawLanguage::Json,
        RawLanguage::Xml,
        RawLanguage::Html,
        RawLanguage::JavaScript,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RawLanguage::Text => "Text",
            RawLanguage::Json => "JSON",
            RawLanguage::Xml => "XML",
            RawLanguage::Html => "HTML",
            RawLanguage::JavaScript => "JavaScript",
        }
    }

    pub fn postman_name(self) -> &'static str {
        match self {
            RawLanguage::Text => "text",
            RawLanguage::Json => "json",
            RawLanguage::Xml => "xml",
            RawLanguage::Html => "html",
            RawLanguage::JavaScript => "javascript",
        }
    }

    fn from_postman_name(name: &str) -> Self {
        RawLanguage::ALL
            .into_iter()
            .find(|language| language.postman_name() == name)
            .unwrap_or_default()
    }

    fn of_body(body: &Body) -> Self {
        body.options
            .as_ref()
            .and_then(|options| {
                options
                    .get(OPTIONS_RAW_KEY)?
                    .get(OPTIONS_LANGUAGE_KEY)?
                    .as_str()
            })
            .map(RawLanguage::from_postman_name)
            .unwrap_or_default()
    }
}

impl BodyDraft {
    pub fn from_body(body: Option<&Body>) -> Self {
        let Some(body) = body else {
            return BodyDraft::default();
        };
        let graphql_text = |key: &str| {
            body.graphql
                .as_ref()
                .and_then(|definition| definition.get(key)?.as_str())
                .unwrap_or_default()
                .to_string()
        };
        BodyDraft {
            kind: BodyKind::from_body(body),
            raw: body.raw.clone().unwrap_or_default(),
            language: RawLanguage::of_body(body),
            urlencoded: rows_from_params(body.urlencoded.as_ref()),
            formdata: rows_from_params(body.formdata.as_ref()),
            file_path: body.file_source().unwrap_or_default().to_string(),
            graphql_query: graphql_text(GRAPHQL_QUERY_KEY),
            graphql_variables: graphql_text(GRAPHQL_VARIABLES_KEY),
            source: Some(body.clone()),
        }
    }

    pub fn to_body(&self) -> Option<Body> {
        if self.kind == BodyKind::None && self.is_empty() {
            return None;
        }
        let mut body = self
            .source
            .clone()
            .unwrap_or_else(|| Body::raw(String::new(), None));
        body.mode = self.kind.mode().or(body.mode);
        body.disabled = (self.kind == BodyKind::None).then_some(true);
        body.raw = non_empty(&self.raw);
        body.options = self.options(body.options.take());
        body.urlencoded = non_empty_rows(&self.urlencoded);
        body.formdata = non_empty_rows(&self.formdata);
        body.file = self.file_value(body.file.take());
        body.graphql = self.graphql_value(body.graphql.take());
        Some(body)
    }

    pub fn active_count(&self) -> usize {
        match self.kind {
            BodyKind::None => 0,
            BodyKind::Raw => usize::from(!self.raw.is_empty()),
            BodyKind::Urlencoded => KeyValueRow::active_count(&self.urlencoded),
            BodyKind::FormData => KeyValueRow::active_count(&self.formdata),
            BodyKind::File => usize::from(!self.file_path.is_empty()),
            BodyKind::GraphQl => usize::from(!self.graphql_query.is_empty()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
            && self.urlencoded.iter().all(KeyValueRow::is_blank)
            && self.formdata.iter().all(KeyValueRow::is_blank)
            && self.file_path.is_empty()
            && self.graphql_query.is_empty()
            && self.graphql_variables.is_empty()
    }

    fn options(&self, existing: Option<Value>) -> Option<Value> {
        if self.raw.is_empty() && existing.is_none() {
            return None;
        }
        let mut options = object_or_empty(existing);
        let mut raw_options = object_or_empty(options.remove(OPTIONS_RAW_KEY));
        raw_options.insert(
            OPTIONS_LANGUAGE_KEY.to_string(),
            json!(self.language.postman_name()),
        );
        options.insert(OPTIONS_RAW_KEY.to_string(), Value::Object(raw_options));
        Some(Value::Object(options))
    }

    fn file_value(&self, existing: Option<Value>) -> Option<Value> {
        if self.file_path.is_empty() {
            return None;
        }
        let mut file = object_or_empty(existing);
        file.insert(FILE_SOURCE_KEY.to_string(), json!(self.file_path));
        Some(Value::Object(file))
    }

    fn graphql_value(&self, existing: Option<Value>) -> Option<Value> {
        if self.graphql_query.is_empty() && self.graphql_variables.is_empty() {
            return None;
        }
        let mut graphql = object_or_empty(existing);
        graphql.insert(GRAPHQL_QUERY_KEY.to_string(), json!(self.graphql_query));
        graphql.insert(
            GRAPHQL_VARIABLES_KEY.to_string(),
            json!(self.graphql_variables),
        );
        Some(Value::Object(graphql))
    }
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_string())
}

fn non_empty_rows(rows: &[KeyValueRow]) -> Option<Vec<crate::collection::Param>> {
    let params = params_from_rows(rows);
    (!params.is_empty()).then_some(params)
}

fn object_or_empty(value: Option<Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

mod format;
#[cfg(test)]
mod tests;
