use serde_json::json;

use crate::header::{Header, CONTENT_TYPE};
use crate::media_type;
use crate::method::HttpMethod;
use crate::prepared::{BodySource, PreparedRequest};

const QUERY_KEY: &str = "query";

pub const INTROSPECTION_QUERY: &str = "query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      kind
      name
      fields(includeDeprecated: false) {
        name
        description
        args { name type { ...TypeRef } }
        type { ...TypeRef }
      }
    }
  }
}

fragment TypeRef on __Type {
  kind name ofType { kind name ofType { kind name ofType { kind name ofType {
  kind name ofType { kind name ofType { kind name ofType { kind name } } } } } } }
}";

impl PreparedRequest {
    pub fn for_schema(&self) -> PreparedRequest {
        let mut headers: Vec<Header> = self
            .headers
            .iter()
            .filter(|header| !header.is_named(CONTENT_TYPE))
            .cloned()
            .collect();
        headers.push(Header::new(CONTENT_TYPE, media_type::JSON));
        let body = json!({ QUERY_KEY: INTROSPECTION_QUERY }).to_string();
        PreparedRequest {
            method: HttpMethod::Post,
            url: self.url.clone(),
            headers,
            body: Some(body.into_bytes()),
            body_source: BodySource::Inline,
        }
    }
}
