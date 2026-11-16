use serde_json::Value;

use super::body::RequestBody;
use super::document::Document;
use super::parameter::Parameters;
use super::reference::Resolver;
use super::values::{description_of, non_empty_text};
use crate::collection::{Item, Request};

const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];
const NAME_KEYS: [&str; 2] = ["summary", "operationId"];

pub struct Operation<'a> {
    path: &'a str,
    method: &'static str,
    spec: &'a Value,
    path_item: &'a Value,
}

impl<'a> Operation<'a> {
    pub fn of_path_item(
        path: &'a str,
        path_item: &'a Value,
    ) -> impl Iterator<Item = Operation<'a>> {
        HTTP_METHODS.iter().filter_map(move |method| {
            Some(Operation {
                path,
                method,
                spec: path_item.get(method)?,
                path_item,
            })
        })
    }

    pub fn folder(&self) -> Option<&'a str> {
        let tag = self.spec.get("tags")?.as_array()?.first()?.as_str()?;
        Some(tag.trim()).filter(|tag| !tag.is_empty())
    }

    fn method_name(&self) -> String {
        self.method.to_ascii_uppercase()
    }

    fn name(&self) -> String {
        NAME_KEYS
            .iter()
            .find_map(|key| non_empty_text(self.spec, key))
            .map_or_else(
                || format!("{} {}", self.method_name(), self.path),
                str::to_string,
            )
    }

    pub fn to_item(&self, resolver: &mut Resolver<'a>, document: &Document<'a>) -> Item {
        let parameters = Parameters::collect(
            resolver,
            self.path_item.get("parameters"),
            self.spec.get("parameters"),
        );
        let mut request = Request::new(&self.method_name(), parameters.url(self.path));
        request.description = description_of(self.spec);
        request.header = (!parameters.headers.is_empty()).then_some(parameters.headers.clone());
        if let Some(body) = self
            .spec
            .get("requestBody")
            .and_then(|spec| RequestBody::from_spec(resolver, spec))
        {
            body.apply_to(&mut request);
        }
        request.auth = self
            .spec
            .get("security")
            .and_then(|requirements| document.auth_for(resolver, requirements));
        Item::for_request(self.name(), request)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Operation;

    #[test]
    fn names_fall_back_from_summary_to_operation_id_to_method_and_path() {
        let path_item = json!({
            "get": {"summary": " List pets ", "operationId": "listPets"},
            "post": {"summary": "", "operationId": "addPet"},
            "delete": {}
        });
        let names: Vec<String> = Operation::of_path_item("/pets", &path_item)
            .map(|operation| operation.name())
            .collect();
        assert_eq!(names, ["List pets", "addPet", "DELETE /pets"]);
    }
}
