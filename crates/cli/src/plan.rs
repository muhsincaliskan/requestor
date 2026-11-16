use requestor_core::collection::{Auth, Collection, Item, Request, RequestSpec};

use crate::error::CliError;

const PATH_SEPARATOR: &str = "/";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub folder: Option<String>,
    pub request: Option<String>,
}

impl Selection {
    fn folder_matches(&self, name: &str) -> bool {
        self.folder.as_deref() == Some(name)
    }

    fn request_matches(&self, name: &str) -> bool {
        self.request.as_deref().is_none_or(|wanted| wanted == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedRequest {
    pub name: String,
    pub request: Request,
}

impl PlannedRequest {
    pub fn select(collection: &Collection, selection: &Selection) -> Result<Vec<Self>, CliError> {
        let mut walker = Walker {
            selection,
            planned: Vec::new(),
            folder_seen: false,
        };
        let root = Frame {
            inside_folder: selection.folder.is_none(),
            path: Vec::new(),
            auth: collection.auth.clone(),
        };
        walker.visit(&collection.item, &root);
        walker.finish()
    }
}

struct Frame {
    inside_folder: bool,
    path: Vec<String>,
    auth: Option<Auth>,
}

impl Frame {
    fn path_to(&self, name: &str) -> Vec<String> {
        let mut path = self.path.clone();
        path.push(name.to_string());
        path
    }
}

struct Walker<'a> {
    selection: &'a Selection,
    planned: Vec<PlannedRequest>,
    folder_seen: bool,
}

impl Walker<'_> {
    fn visit(&mut self, items: &[Item], frame: &Frame) {
        for item in items {
            self.visit_item(item, frame);
        }
    }

    fn visit_item(&mut self, item: &Item, frame: &Frame) {
        let name = item.name.clone().unwrap_or_default();
        if let Some(children) = &item.item {
            let child = self.descend(item, &name, frame);
            self.visit(children, &child);
        } else if let Some(spec) = &item.request {
            self.accept(&name, spec, frame);
        }
    }

    fn descend(&mut self, item: &Item, name: &str, frame: &Frame) -> Frame {
        let matched = self.selection.folder_matches(name);
        self.folder_seen |= matched;
        Frame {
            inside_folder: frame.inside_folder || matched,
            path: frame.path_to(name),
            auth: item.auth.clone().or_else(|| frame.auth.clone()),
        }
    }

    fn accept(&mut self, name: &str, spec: &RequestSpec, frame: &Frame) {
        if !frame.inside_folder || !self.selection.request_matches(name) {
            return;
        }
        let mut request = spec.to_request();
        if request.auth.is_none() {
            request.auth = frame.auth.clone();
        }
        self.planned.push(PlannedRequest {
            name: frame.path_to(name).join(PATH_SEPARATOR),
            request,
        });
    }

    fn finish(self) -> Result<Vec<PlannedRequest>, CliError> {
        if let Some(folder) = self.selection.folder.as_ref().filter(|_| !self.folder_seen) {
            return Err(CliError::usage(format!("no folder named {folder:?}")));
        }
        if self.planned.is_empty() {
            return Err(CliError::usage("no request matches the selection"));
        }
        Ok(self.planned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLECTION: &str = r#"{
        "info": {"name": "c"},
        "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "t"}]},
        "item": [
            {"name": "Root", "request": "http://h/root"},
            {"name": "Users", "item": [
                {"name": "List", "request": {"method": "GET", "url": "http://h/u"}},
                {"name": "Admin", "item": [{"name": "List", "request": "http://h/a"}]}
            ]}
        ]
    }"#;

    fn names(collection: &Collection, selection: &Selection) -> Result<Vec<String>, CliError> {
        PlannedRequest::select(collection, selection)
            .map(|planned| planned.into_iter().map(|entry| entry.name).collect())
    }

    #[test]
    fn selects_everything_in_order_without_filters() {
        let collection = Collection::from_json(COLLECTION).unwrap();
        let all = names(&collection, &Selection::default()).unwrap();
        assert_eq!(all, ["Root", "Users/List", "Users/Admin/List"]);
    }

    #[test]
    fn folder_filter_includes_nested_folders() {
        let collection = Collection::from_json(COLLECTION).unwrap();
        let selection = Selection {
            folder: Some("Users".into()),
            request: None,
        };
        assert_eq!(
            names(&collection, &selection).unwrap(),
            ["Users/List", "Users/Admin/List"]
        );
    }

    #[test]
    fn request_filter_matches_by_name() {
        let collection = Collection::from_json(COLLECTION).unwrap();
        let selection = Selection {
            folder: None,
            request: Some("Root".into()),
        };
        assert_eq!(names(&collection, &selection).unwrap(), ["Root"]);
    }

    #[test]
    fn unknown_selection_is_a_usage_error() {
        let collection = Collection::from_json(COLLECTION).unwrap();
        let missing_folder = Selection {
            folder: Some("Nope".into()),
            request: None,
        };
        let missing_request = Selection {
            folder: None,
            request: Some("Nope".into()),
        };
        assert!(names(&collection, &missing_folder).unwrap_err().is_usage());
        assert!(names(&collection, &missing_request).unwrap_err().is_usage());
    }

    #[test]
    fn requests_inherit_collection_auth() {
        let collection = Collection::from_json(COLLECTION).unwrap();
        let planned = PlannedRequest::select(&collection, &Selection::default()).unwrap();
        assert!(planned.iter().all(|entry| entry.request.auth.is_some()));
    }
}
