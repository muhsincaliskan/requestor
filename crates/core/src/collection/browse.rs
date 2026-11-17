use super::{Collection, Item, RequestSpec};

const DEFAULT_METHOD: &str = "GET";

impl Item {
    pub fn matches_filter(&self, filter: &str) -> bool {
        self.matches_lowercase(&filter.to_lowercase())
    }

    pub fn method_name(&self) -> Option<&str> {
        match self.request.as_ref()? {
            RequestSpec::Full(request) => Some(request.method.as_deref().unwrap_or(DEFAULT_METHOD)),
            RequestSpec::Url(_) => Some(DEFAULT_METHOD),
        }
    }

    fn matches_lowercase(&self, needle: &str) -> bool {
        needle.is_empty()
            || self.display_name().to_lowercase().contains(needle)
            || self
                .children()
                .iter()
                .any(|child| child.matches_lowercase(needle))
    }
}

impl Collection {
    pub fn matches_filter(&self, filter: &str) -> bool {
        let needle = filter.to_lowercase();
        needle.is_empty()
            || self.info.name.to_lowercase().contains(&needle)
            || self.item.iter().any(|item| item.matches_lowercase(&needle))
    }

    pub fn position_of(&self, id: &str) -> Option<(Option<String>, usize)> {
        let trail = self.ancestors_of(id)?;
        let parent = trail.last();
        let siblings = parent.map_or(self.item.as_slice(), |folder| folder.children());
        let index = siblings.iter().position(|item| item.has_id(id))?;
        Some((parent.and_then(|folder| folder.id.clone()), index))
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::{Collection, Item, Request, Url};

    fn request_item(name: &str, method: &str) -> Item {
        Item::for_request(
            name.to_string(),
            Request::new(method, Url::Raw("https://x.test".to_string())),
        )
    }

    fn sample() -> (Collection, String, String) {
        let mut collection = Collection::new("Shop");
        let folder = collection
            .add_item(None, Item::folder("Orders".to_string(), Vec::new()))
            .unwrap();
        collection.add_item(None, request_item("Ping", "GET"));
        let nested = collection
            .add_item(Some(&folder), request_item("Create Order", "POST"))
            .unwrap();
        (collection, folder, nested)
    }

    #[test]
    fn filter_matches_names_case_insensitively_through_descendants() {
        let (collection, folder, _) = sample();
        let folder = collection.find_item(&folder).unwrap();
        assert!(folder.matches_filter("create"));
        assert!(folder.matches_filter("ORDERS"));
        assert!(!folder.matches_filter("ping"));
        assert!(folder.matches_filter(""));
    }

    #[test]
    fn collection_matches_by_own_name_or_any_item() {
        let (collection, _, _) = sample();
        assert!(collection.matches_filter("shop"));
        assert!(collection.matches_filter("PING"));
        assert!(!collection.matches_filter("missing"));
    }

    #[test]
    fn method_name_defaults_to_get_and_is_absent_for_folders() {
        let (collection, folder, nested) = sample();
        assert_eq!(
            collection.find_item(&nested).unwrap().method_name(),
            Some("POST")
        );
        assert_eq!(collection.find_item(&folder).unwrap().method_name(), None);
        let mut bare = request_item("bare", "GET");
        bare.replace_request(Request {
            method: None,
            ..Request::new("GET", Url::Raw(String::new()))
        });
        assert_eq!(bare.method_name(), Some("GET"));
    }

    #[test]
    fn position_of_reports_parent_and_index_among_siblings() {
        let (collection, folder, nested) = sample();
        assert_eq!(collection.position_of(&folder), Some((None, 0)));
        assert_eq!(collection.position_of(&nested), Some((Some(folder), 0)));
        assert_eq!(collection.position_of("missing"), None);
    }
}
