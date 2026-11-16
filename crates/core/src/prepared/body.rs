use serde_json::{json, Map, Value};

use super::multipart;
use crate::collection::{Body, BodyMode};
use crate::encoding::percent_encode;
use crate::error::CoreError;
use crate::variables::VariableScope;

const RAW_LANGUAGE_CONTENT_TYPES: [(&str, &str); 5] = [
    ("json", "application/json"),
    ("xml", "application/xml"),
    ("html", "text/html"),
    ("javascript", "application/javascript"),
    ("text", "text/plain"),
];
const DEFAULT_RAW_CONTENT_TYPE: &str = "text/plain";
const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";
const JSON_CONTENT_TYPE: &str = "application/json";
const BINARY_CONTENT_TYPE: &str = "application/octet-stream";

pub(super) struct EncodedBody {
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

impl EncodedBody {
    pub(super) fn from_body(body: &Body, scope: &VariableScope) -> Result<Option<Self>, CoreError> {
        match body.mode {
            Some(BodyMode::Raw) => Ok(Some(Self::raw(body, scope))),
            Some(BodyMode::Urlencoded) => Ok(Some(Self::urlencoded(body, scope))),
            Some(BodyMode::Formdata) => {
                multipart::encode(&body.enabled_form_params(), scope).map(Some)
            }
            Some(BodyMode::File) => Self::file(body).map(Some),
            Some(BodyMode::Graphql) => Ok(Self::graphql(body, scope)),
            None => Ok(None),
        }
    }

    fn raw(body: &Body, scope: &VariableScope) -> Self {
        let text = scope.resolve(body.raw.as_deref().unwrap_or_default()).text;
        EncodedBody {
            content_type: Some(raw_content_type(body).to_string()),
            data: text.into_bytes(),
        }
    }

    fn urlencoded(body: &Body, scope: &VariableScope) -> Self {
        let pairs: Vec<String> = body
            .enabled_form_params()
            .into_iter()
            .map(|param| {
                let key = scope.resolve(param.key.as_deref().unwrap_or_default()).text;
                let value = scope
                    .resolve(param.value.as_deref().unwrap_or_default())
                    .text;
                format!("{}={}", percent_encode(&key), percent_encode(&value))
            })
            .collect();
        EncodedBody {
            content_type: Some(FORM_CONTENT_TYPE.to_string()),
            data: pairs.join("&").into_bytes(),
        }
    }

    fn file(body: &Body) -> Result<Self, CoreError> {
        let path = body
            .file
            .as_ref()
            .and_then(|file| file.get("src"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let data = std::fs::read(path).map_err(|error| CoreError::UnreadableFile {
            path: path.to_string(),
            reason: error.to_string(),
        })?;
        Ok(EncodedBody {
            content_type: Some(BINARY_CONTENT_TYPE.to_string()),
            data,
        })
    }

    fn graphql(body: &Body, scope: &VariableScope) -> Option<Self> {
        let definition = body.graphql.as_ref()?;
        let query = scope.resolve(definition.get("query")?.as_str()?).text;
        let variables = graphql_variables(definition, scope);
        let payload = json!({ "query": query, "variables": variables });
        Some(EncodedBody {
            content_type: Some(JSON_CONTENT_TYPE.to_string()),
            data: payload.to_string().into_bytes(),
        })
    }
}

fn graphql_variables(definition: &Value, scope: &VariableScope) -> Value {
    definition
        .get("variables")
        .and_then(Value::as_str)
        .map(|text| scope.resolve(text).text)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| Value::Object(Map::new()))
}

fn raw_content_type(body: &Body) -> &'static str {
    let language = body
        .options
        .as_ref()
        .and_then(|options| options.get("raw")?.get("language")?.as_str());
    RAW_LANGUAGE_CONTENT_TYPES
        .iter()
        .find(|(name, _)| Some(*name) == language)
        .map_or(DEFAULT_RAW_CONTENT_TYPE, |(_, content_type)| content_type)
}
