use serde_json::{Map, Value};

use super::example::media_sample;
use super::reference::Resolver;
use super::values::value_text;
use crate::collection::{Body, Param, Request};
use crate::header::CONTENT_TYPE;
use crate::media_type;

const PREFERRED_CONTENT_TYPES: [&str; 3] = [
    media_type::JSON,
    media_type::FORM_URLENCODED,
    media_type::MULTIPART_FORM_DATA,
];
const PARAMETER_SEPARATOR: char = ';';
const BINARY_FORMAT: &str = "binary";

pub struct RequestBody {
    body: Body,
    content_type: String,
}

struct FormField {
    name: String,
    value: String,
    is_file: bool,
}

impl RequestBody {
    pub fn from_spec<'a>(resolver: &mut Resolver<'a>, spec: &'a Value) -> Option<Self> {
        let spec = resolver.resolve(spec)?;
        let (content_type, media) = select_media(spec.get("content")?.as_object()?)?;
        let sample = media_sample(resolver, media);
        let body = match base_type(content_type).to_ascii_lowercase().as_str() {
            media_type::FORM_URLENCODED => {
                let fields = form_fields(resolver, media, sample);
                Body::urlencoded(fields.into_iter().map(FormField::into_text_param).collect())
            }
            media_type::MULTIPART_FORM_DATA => {
                let fields = form_fields(resolver, media, sample);
                Body::formdata(fields.into_iter().map(FormField::into_param).collect())
            }
            _ => Body::raw_for_content_type(raw_text(sample), Some(content_type)),
        };
        Some(RequestBody {
            body,
            content_type: content_type.to_string(),
        })
    }

    pub fn apply_to(self, request: &mut Request) {
        if !request.has_header(CONTENT_TYPE) {
            request.push_header(CONTENT_TYPE, self.content_type);
        }
        request.body = Some(self.body);
    }
}

impl FormField {
    fn into_text_param(self) -> Param {
        Param::text(self.name, self.value)
    }

    fn into_param(self) -> Param {
        if self.is_file {
            Param::file(self.name, "")
        } else {
            self.into_text_param()
        }
    }
}

fn base_type(content_type: &str) -> &str {
    content_type
        .split(PARAMETER_SEPARATOR)
        .next()
        .unwrap_or(content_type)
        .trim()
}

fn select_media(content: &Map<String, Value>) -> Option<(&str, &Value)> {
    PREFERRED_CONTENT_TYPES
        .iter()
        .find_map(|preferred| {
            content
                .iter()
                .find(|(key, _)| base_type(key).eq_ignore_ascii_case(preferred))
        })
        .or_else(|| content.iter().next())
        .map(|(key, media)| (key.as_str(), media))
}

fn raw_text(sample: Option<Value>) -> String {
    match sample {
        None => String::new(),
        Some(Value::String(text)) => text,
        Some(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
    }
}

fn form_fields<'a>(
    resolver: &mut Resolver<'a>,
    media: &'a Value,
    sample: Option<Value>,
) -> Vec<FormField> {
    let Some(Value::Object(fields)) = sample else {
        return Vec::new();
    };
    let properties = media
        .get("schema")
        .and_then(|schema| resolver.resolve(schema))
        .and_then(|schema| schema.get("properties")?.as_object());
    fields
        .into_iter()
        .map(|(name, value)| FormField {
            is_file: is_binary(resolver, properties, &name),
            value: value_text(&value),
            name,
        })
        .collect()
}

fn is_binary<'a>(
    resolver: &mut Resolver<'a>,
    properties: Option<&'a Map<String, Value>>,
    name: &str,
) -> bool {
    properties
        .and_then(|properties| properties.get(name))
        .and_then(|property| resolver.resolve(property))
        .and_then(|property| property.get("format")?.as_str())
        == Some(BINARY_FORMAT)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{select_media, RequestBody};
    use crate::import::openapi::reference::Resolver;

    #[test]
    fn json_is_preferred_and_other_types_fall_back_to_the_first() {
        let both = json!({"text/plain": {}, "application/json; charset=utf-8": {}});
        assert_eq!(
            select_media(both.as_object().unwrap()).unwrap().0,
            "application/json; charset=utf-8"
        );
        let other = json!({"text/csv": {}});
        assert_eq!(
            select_media(other.as_object().unwrap()).unwrap().0,
            "text/csv"
        );
    }

    #[test]
    fn multipart_binary_fields_become_file_params() {
        let root = json!({"content": {"multipart/form-data": {"schema": {"type": "object",
            "properties": {"file": {"type": "string", "format": "binary"},
                           "note": {"type": "string", "example": "hi"}}}}}});
        let mut resolver = Resolver::new(&root);
        let body = RequestBody::from_spec(&mut resolver, &root).unwrap().body;
        let params = body.formdata.unwrap();
        assert_eq!(params[0].kind.as_deref(), Some("file"));
        assert_eq!(params[1].value.as_deref(), Some("hi"));
    }
}
