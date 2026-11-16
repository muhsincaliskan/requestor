use std::collections::BTreeMap;

use serde_json::Value;

use super::reference::Resolver;
use crate::collection::{Auth, AuthAttribute};

const NO_AUTH_KIND: &str = "noauth";
const API_KEY_KIND: &str = "apikey";
const BEARER_TOKEN: &str = "{{bearerToken}}";
const BASIC_USERNAME: &str = "{{username}}";
const BASIC_PASSWORD: &str = "{{password}}";
const API_KEY_VALUE: &str = "{{apiKey}}";
const API_KEY_LOCATIONS: [&str; 2] = ["header", "query"];

pub fn auth_for<'a>(
    resolver: &mut Resolver<'a>,
    schemes: Option<&'a Value>,
    requirements: &'a Value,
) -> Option<Auth> {
    let alternatives = requirements.as_array()?;
    if alternatives.is_empty() {
        return Some(no_auth());
    }
    alternatives
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|requirement| requirement.keys())
        .find_map(|name| scheme_auth(resolver, schemes, name))
}

fn scheme_auth<'a>(
    resolver: &mut Resolver<'a>,
    schemes: Option<&'a Value>,
    name: &str,
) -> Option<Auth> {
    let Some(raw) = schemes.and_then(|schemes| schemes.get(name)) else {
        resolver.warn(format!("security scheme {name} is not defined"));
        return None;
    };
    let scheme = resolver.resolve(raw)?;
    let kind = scheme
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let auth = match kind {
        "http" => http_auth(scheme),
        "apiKey" => api_key_auth(scheme),
        _ => None,
    };
    if auth.is_none() {
        resolver.warn(format!(
            "security scheme {name} ({kind}) is not supported and was skipped"
        ));
    }
    auth
}

fn http_auth(scheme: &Value) -> Option<Auth> {
    let name = scheme.get("scheme")?.as_str()?.to_ascii_lowercase();
    match name.as_str() {
        "bearer" => Some(Auth::bearer(BEARER_TOKEN)),
        "basic" => Some(Auth::basic(BASIC_USERNAME, BASIC_PASSWORD)),
        _ => None,
    }
}

fn api_key_auth(scheme: &Value) -> Option<Auth> {
    let location = scheme.get("in")?.as_str()?;
    if !API_KEY_LOCATIONS.contains(&location) {
        return None;
    }
    let attributes = vec![
        AuthAttribute::string("key", scheme.get("name")?.as_str()?),
        AuthAttribute::string("value", API_KEY_VALUE),
        AuthAttribute::string("in", location),
    ];
    Some(Auth {
        kind: API_KEY_KIND.to_string(),
        parameters: BTreeMap::from([(API_KEY_KIND.to_string(), Some(attributes))]),
    })
}

fn no_auth() -> Auth {
    Auth {
        kind: NO_AUTH_KIND.to_string(),
        parameters: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::auth_for;
    use crate::import::openapi::reference::Resolver;

    #[test]
    fn maps_supported_schemes_and_warns_for_oauth() {
        let root = json!({
            "schemes": {
                "token": {"type": "http", "scheme": "Bearer"},
                "key": {"type": "apiKey", "in": "header", "name": "X-Key"},
                "oauth": {"type": "oauth2"}
            },
            "oauth": [{"oauth": []}],
            "token": [{"token": []}],
            "key": [{"key": []}],
            "none": []
        });
        let mut resolver = Resolver::new(&root);
        let schemes = Some(&root["schemes"]);
        let bearer = auth_for(&mut resolver, schemes, &root["token"]).unwrap();
        assert_eq!(bearer.value_of("token"), Some("{{bearerToken}}"));
        let api_key = auth_for(&mut resolver, schemes, &root["key"]).unwrap();
        assert_eq!(api_key.value_of("key"), Some("X-Key"));
        assert!(auth_for(&mut resolver, schemes, &root["oauth"]).is_none());
        assert!(auth_for(&mut resolver, schemes, &root["none"])
            .unwrap()
            .is_none());
        assert_eq!(resolver.into_warnings().len(), 1);
    }
}
