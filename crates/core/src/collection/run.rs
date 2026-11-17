use super::{Auth, Collection, Item, Request, Variable};

const PATH_SEPARATOR: &str = "/";

#[derive(Debug, Clone, PartialEq)]
pub struct RunnableRequest {
    pub path: Vec<String>,
    pub name: String,
    pub request: Request,
    pub variables: Vec<Variable>,
}

impl RunnableRequest {
    pub fn qualified_name(&self) -> String {
        let mut parts: Vec<&str> = self.path.iter().map(String::as_str).collect();
        parts.push(&self.name);
        parts.join(PATH_SEPARATOR)
    }
}

impl Collection {
    pub fn runnable_requests(&self) -> Vec<RunnableRequest> {
        let mut found = Vec::new();
        let root = Scope {
            path: Vec::new(),
            auth: self.auth.clone(),
            variables: Vec::new(),
        };
        root.collect(&self.item, &mut found);
        found
    }

    pub fn has_folder_named(&self, name: &str) -> bool {
        folder_named(&self.item, name)
    }
}

fn folder_named(items: &[Item], name: &str) -> bool {
    items
        .iter()
        .filter_map(|item| item.item.as_deref().map(|children| (item, children)))
        .any(|(folder, children)| {
            folder.name.as_deref() == Some(name) || folder_named(children, name)
        })
}

struct Scope {
    path: Vec<String>,
    auth: Option<Auth>,
    variables: Vec<Variable>,
}

impl Scope {
    fn collect(&self, items: &[Item], found: &mut Vec<RunnableRequest>) {
        for item in items {
            if let Some(children) = &item.item {
                self.enter(item).collect(children, found);
            } else if let Some(spec) = &item.request {
                found.push(self.runnable(item, spec.to_request()));
            }
        }
    }

    fn enter(&self, folder: &Item) -> Scope {
        let mut path = self.path.clone();
        path.push(folder.name.clone().unwrap_or_default());
        let mut variables = self.variables.clone();
        variables.extend(folder.variable.iter().flatten().cloned());
        Scope {
            path,
            auth: folder.auth.clone().or_else(|| self.auth.clone()),
            variables,
        }
    }

    fn runnable(&self, item: &Item, mut request: Request) -> RunnableRequest {
        if request.auth.is_none() {
            request.auth = item.auth.clone().or_else(|| self.auth.clone());
        }
        RunnableRequest {
            path: self.path.clone(),
            name: item.name.clone().unwrap_or_default(),
            request,
            variables: self.variables.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENT: &str = r#"{
        "info": {"name": "c"},
        "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "root"}]},
        "item": [
            {"name": "Top", "request": "http://h/top"},
            {"name": "Users", "variable": [{"key": "scope", "value": "users"}],
             "auth": {"type": "basic", "basic": [{"key": "username", "value": "u"}]},
             "item": [
                {"name": "List", "request": {"method": "GET", "url": "http://h/u"}},
                {"name": "Open", "request": {"method": "GET", "url": "http://h/o",
                    "auth": {"type": "noauth"}}},
                {"name": "Admin", "variable": [{"key": "scope", "value": "admin"}],
                 "item": [{"name": "Drop", "request": "http://h/a"}]},
                {"name": "Anon", "auth": {"type": "noauth"},
                 "item": [{"name": "Ping", "request": "http://h/p"}]}
            ]}
        ]
    }"#;

    fn runnables() -> Vec<RunnableRequest> {
        Collection::from_json(DOCUMENT).unwrap().runnable_requests()
    }

    fn find(name: &str) -> RunnableRequest {
        runnables()
            .into_iter()
            .find(|entry| entry.name == name)
            .unwrap()
    }

    #[test]
    fn requests_come_in_document_order_with_their_folder_path() {
        let names: Vec<String> = runnables()
            .iter()
            .map(RunnableRequest::qualified_name)
            .collect();
        assert_eq!(
            names,
            [
                "Top",
                "Users/List",
                "Users/Open",
                "Users/Admin/Drop",
                "Users/Anon/Ping"
            ]
        );
    }

    #[test]
    fn auth_is_inherited_from_nearest_folder_then_collection() {
        assert_eq!(find("Top").request.auth.unwrap().kind, "bearer");
        assert_eq!(find("List").request.auth.unwrap().kind, "basic");
        assert_eq!(find("Drop").request.auth.unwrap().kind, "basic");
    }

    #[test]
    fn explicit_no_auth_is_respected_on_requests_and_folders() {
        assert!(find("Open").request.auth.unwrap().is_none());
        assert!(find("Ping").request.auth.unwrap().is_none());
    }

    #[test]
    fn folders_are_found_by_name_even_when_empty_or_nested() {
        let collection = Collection::from_json(
            r#"{"info":{"name":"c"},"item":[
                {"name":"Outer","item":[{"name":"Inner","item":[]}]},
                {"name":"Leaf","request":"http://h"}]}"#,
        )
        .unwrap();
        assert!(collection.has_folder_named("Outer"));
        assert!(collection.has_folder_named("Inner"));
        assert!(!collection.has_folder_named("Leaf"));
        assert!(!collection.has_folder_named("Missing"));
    }

    #[test]
    fn folder_variables_are_collected_outermost_first() {
        assert!(find("Top").variables.is_empty());
        let nested = find("Drop").variables;
        let values: Vec<_> = nested.iter().filter_map(Variable::text_value).collect();
        assert_eq!(values, ["users", "admin"]);
    }
}
