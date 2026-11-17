use serde_json::Value;

use super::example::{declared_value, explicit_example};
use super::reference::Resolver;
use super::values::{description_of, value_text};
use super::BASE_URL_PLACEHOLDER;
use crate::collection::{Param, Url, UrlParts, Variable};
use crate::header::CONTENT_TYPE;
use crate::text::StringOrList;

const RESERVED_HEADERS: [&str; 3] = ["Accept", CONTENT_TYPE, "Authorization"];

const QUERY_LOCATION: &str = "query";
const HEADER_LOCATION: &str = "header";
const PATH_LOCATION: &str = "path";
const COOKIE_LOCATION: &str = "cookie";
const PATH_SEPARATOR: &str = "/";
const QUERY_MARK: &str = "?";
const QUERY_SEPARATOR: &str = "&";
const PATH_VARIABLE_PREFIX: &str = ":";

struct Candidate<'a> {
    name: &'a str,
    location: &'a str,
    spec: &'a Value,
}

#[derive(Default)]
pub struct Parameters {
    query: Vec<Param>,
    pub headers: Vec<Param>,
    path_values: Vec<(String, String)>,
}

impl<'a> Candidate<'a> {
    fn from_value(resolver: &mut Resolver<'a>, raw: &'a Value) -> Option<Self> {
        let spec = resolver.resolve(raw)?;
        Some(Candidate {
            name: spec.get("name")?.as_str()?,
            location: spec.get("in")?.as_str()?,
            spec,
        })
    }

    fn is_same_parameter(&self, other: &Candidate) -> bool {
        self.name == other.name && self.location == other.location
    }

    fn example_text(&self, resolver: &mut Resolver<'a>) -> String {
        let value = explicit_example(resolver, self.spec).or_else(|| {
            let schema = resolver.resolve(self.spec.get("schema")?)?;
            declared_value(schema)
        });
        value.as_ref().map(value_text).unwrap_or_default()
    }

    fn is_reserved_header(&self) -> bool {
        RESERVED_HEADERS
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(self.name))
    }

    fn is_required(&self) -> bool {
        self.spec.get("required").and_then(Value::as_bool) == Some(true)
    }

    fn to_param(&self, resolver: &mut Resolver<'a>) -> Param {
        Param {
            disabled: (!self.is_required()).then_some(true),
            description: description_of(self.spec),
            ..Param::text(self.name, self.example_text(resolver))
        }
    }
}

impl<'a> Parameters {
    pub fn collect(
        resolver: &mut Resolver<'a>,
        path_level: Option<&'a Value>,
        operation_level: Option<&'a Value>,
    ) -> Self {
        let mut merged: Vec<Candidate<'a>> = Vec::new();
        let lists = [path_level, operation_level].into_iter().flatten();
        for raw in lists.filter_map(Value::as_array).flatten() {
            let Some(candidate) = Candidate::from_value(resolver, raw) else {
                continue;
            };
            match merged
                .iter()
                .position(|kept| kept.is_same_parameter(&candidate))
            {
                Some(index) => merged[index] = candidate,
                None => merged.push(candidate),
            }
        }
        let mut parameters = Parameters::default();
        for candidate in &merged {
            parameters.accept(resolver, candidate);
        }
        parameters
    }

    fn accept(&mut self, resolver: &mut Resolver<'a>, candidate: &Candidate<'a>) {
        match candidate.location {
            QUERY_LOCATION => self.query.push(candidate.to_param(resolver)),
            HEADER_LOCATION if candidate.is_reserved_header() => {}
            HEADER_LOCATION => self.headers.push(candidate.to_param(resolver)),
            PATH_LOCATION => self
                .path_values
                .push((candidate.name.to_string(), candidate.example_text(resolver))),
            COOKIE_LOCATION => resolver.warn(format!(
                "cookie parameter {} is not supported and was skipped",
                candidate.name
            )),
            other => resolver.warn(format!("parameter location {other} is not supported")),
        }
    }

    pub fn url(&self, path: &str) -> Url {
        let segments: Vec<String> = path
            .split(PATH_SEPARATOR)
            .filter(|segment| !segment.is_empty())
            .map(colon_form)
            .collect();
        let variables = self.path_variables(&segments);
        let raw = format!(
            "{BASE_URL_PLACEHOLDER}{PATH_SEPARATOR}{}{}",
            segments.join(PATH_SEPARATOR),
            self.query_text()
        );
        Url::Parts(Box::new(UrlParts {
            raw: Some(raw),
            protocol: None,
            host: Some(StringOrList::List(vec![BASE_URL_PLACEHOLDER.to_string()])),
            path: Some(StringOrList::List(segments)),
            port: None,
            query: (!self.query.is_empty()).then(|| self.query.clone()),
            variable: (!variables.is_empty()).then_some(variables),
            hash: None,
            extra: serde_json::Map::new(),
        }))
    }

    fn path_variables(&self, segments: &[String]) -> Vec<Variable> {
        segments
            .iter()
            .filter_map(|segment| segment.strip_prefix(PATH_VARIABLE_PREFIX))
            .map(|name| Variable::text(name, self.path_value(name)))
            .collect()
    }

    fn path_value(&self, name: &str) -> &str {
        self.path_values
            .iter()
            .find(|(declared, _)| declared == name)
            .map_or("", |(_, value)| value)
    }

    fn query_text(&self) -> String {
        let pairs: Vec<String> = self
            .query
            .iter()
            .filter(|param| param.is_enabled())
            .map(Param::query_pair)
            .collect();
        if pairs.is_empty() {
            return String::new();
        }
        format!("{QUERY_MARK}{}", pairs.join(QUERY_SEPARATOR))
    }
}

fn colon_form(segment: &str) -> String {
    let name = segment
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .filter(|name| !name.contains(['{', '}']));
    match name {
        Some(name) => format!("{PATH_VARIABLE_PREFIX}{name}"),
        None => segment.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Parameters;
    use crate::import::openapi::reference::Resolver;

    #[test]
    fn operation_parameters_override_path_level_ones_and_optional_ones_are_disabled() {
        let root = json!({
            "path": [
                {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 10}},
                {"name": "id", "in": "path", "required": true, "example": 7}
            ],
            "operation": [
                {"name": "limit", "in": "query", "required": true, "example": 50},
                {"name": "X-Trace", "in": "header", "example": "abc"},
                {"name": "sid", "in": "cookie"}
            ]
        });
        let mut resolver = Resolver::new(&root);
        let parameters =
            Parameters::collect(&mut resolver, Some(&root["path"]), Some(&root["operation"]));
        let url = parameters.url("/pets/{id}");
        assert_eq!(url.to_text(), "{{baseUrl}}/pets/:id?limit=50");
        assert_eq!(parameters.headers[0].disabled, Some(true));
        assert_eq!(resolver.into_warnings().len(), 1);
    }

    #[test]
    fn reserved_headers_are_ignored_case_insensitively() {
        let root = json!([
            {"name": "accept", "in": "header"},
            {"name": "CONTENT-TYPE", "in": "header"},
            {"name": "Authorization", "in": "header"},
            {"name": "X-Keep", "in": "header"}
        ]);
        let mut resolver = Resolver::new(&root);
        let parameters = Parameters::collect(&mut resolver, None, Some(&root));
        assert_eq!(parameters.headers.len(), 1);
        assert_eq!(parameters.headers[0].key.as_deref(), Some("X-Keep"));
    }
}
