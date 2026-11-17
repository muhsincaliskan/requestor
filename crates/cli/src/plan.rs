use requestor_core::collection::{Collection, RunnableRequest};

use crate::error::CliError;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub folder: Option<String>,
    pub request: Option<String>,
}

impl Selection {
    pub fn pick(&self, collection: &Collection) -> Result<Vec<RunnableRequest>, CliError> {
        let all = collection.runnable_requests();
        if let Some(folder) = self
            .folder
            .as_ref()
            .filter(|folder| !Self::has_folder(&all, folder))
        {
            return Err(CliError::usage(format!("no folder named {folder:?}")));
        }
        let chosen: Vec<RunnableRequest> = all
            .into_iter()
            .filter(|runnable| self.matches(runnable))
            .collect();
        if chosen.is_empty() {
            return Err(CliError::usage("no request matches the selection"));
        }
        Ok(chosen)
    }

    fn has_folder(all: &[RunnableRequest], folder: &str) -> bool {
        all.iter()
            .any(|runnable| runnable.path.iter().any(|name| name == folder))
    }

    fn matches(&self, runnable: &RunnableRequest) -> bool {
        let in_folder = self
            .folder
            .as_ref()
            .is_none_or(|folder| runnable.path.contains(folder));
        let named = self
            .request
            .as_ref()
            .is_none_or(|name| *name == runnable.name);
        in_folder && named
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLECTION: &str = r#"{
        "info": {"name": "c"},
        "item": [
            {"name": "Root", "request": "http://h/root"},
            {"name": "Users", "item": [
                {"name": "List", "request": {"method": "GET", "url": "http://h/u"}},
                {"name": "Admin", "item": [{"name": "List", "request": "http://h/a"}]}
            ]}
        ]
    }"#;

    fn names(selection: &Selection) -> Result<Vec<String>, CliError> {
        let collection = Collection::from_json(COLLECTION).unwrap();
        selection
            .pick(&collection)
            .map(|picked| picked.iter().map(RunnableRequest::qualified_name).collect())
    }

    fn select(folder: Option<&str>, request: Option<&str>) -> Selection {
        Selection {
            folder: folder.map(str::to_string),
            request: request.map(str::to_string),
        }
    }

    #[test]
    fn selects_everything_in_order_without_filters() {
        assert_eq!(
            names(&Selection::default()).unwrap(),
            ["Root", "Users/List", "Users/Admin/List"]
        );
    }

    #[test]
    fn folder_filter_includes_nested_folders() {
        assert_eq!(
            names(&select(Some("Users"), None)).unwrap(),
            ["Users/List", "Users/Admin/List"]
        );
        assert_eq!(
            names(&select(Some("Admin"), None)).unwrap(),
            ["Users/Admin/List"]
        );
    }

    #[test]
    fn request_filter_matches_by_name_and_combines_with_folder() {
        assert_eq!(names(&select(None, Some("Root"))).unwrap(), ["Root"]);
        assert_eq!(
            names(&select(Some("Admin"), Some("List"))).unwrap(),
            ["Users/Admin/List"]
        );
    }

    #[test]
    fn unknown_selection_is_a_usage_error() {
        assert!(names(&select(Some("Nope"), None)).unwrap_err().is_usage());
        assert!(names(&select(None, Some("Nope"))).unwrap_err().is_usage());
    }
}
