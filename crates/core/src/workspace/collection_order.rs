use super::Workspace;

impl Workspace {
    pub fn collection_order(&self) -> Vec<String> {
        self.collections
            .iter()
            .map(|collection| collection.id().to_string())
            .collect()
    }

    pub fn move_collection(&mut self, id: &str, index: usize) -> bool {
        let Some(position) = self
            .collections
            .iter()
            .position(|collection| collection.id() == id)
        else {
            return false;
        };
        let collection = self.collections.remove(position);
        let index = index.min(self.collections.len());
        self.collections.insert(index, collection);
        position != index
    }

    pub(super) fn restore_collection_order(&mut self) {
        let order = &self.session.collection_order;
        self.collections.sort_by_key(|collection| {
            order
                .iter()
                .position(|id| id == collection.id())
                .unwrap_or(usize::MAX)
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::Collection;
    use crate::workspace::Workspace;

    fn workspace(names: &[&str]) -> (Workspace, Vec<String>) {
        let mut workspace = Workspace::default();
        let ids = names
            .iter()
            .map(|name| workspace.add_collection(Collection::new(*name)))
            .collect();
        (workspace, ids)
    }

    fn names(workspace: &Workspace) -> Vec<&str> {
        workspace
            .collections
            .iter()
            .map(|collection| collection.info.name.as_str())
            .collect()
    }

    #[test]
    fn moving_a_collection_places_it_at_the_index() {
        let (mut workspace, ids) = workspace(&["A", "B", "C"]);
        assert!(workspace.move_collection(&ids[2], 0));
        assert_eq!(names(&workspace), ["C", "A", "B"]);
        assert!(workspace.move_collection(&ids[2], usize::MAX));
        assert_eq!(names(&workspace), ["A", "B", "C"]);
        assert!(!workspace.move_collection(&ids[0], 0));
        assert!(!workspace.move_collection("missing", 0));
    }

    #[test]
    fn the_saved_order_is_restored_and_new_collections_go_last() {
        let (mut workspace, ids) = workspace(&["A", "B", "C"]);
        workspace.session.collection_order = vec![ids[2].clone(), ids[0].clone()];
        workspace.normalize();
        assert_eq!(names(&workspace), ["C", "A", "B"]);
        assert_eq!(
            workspace.collection_order(),
            [&ids[2], &ids[0], &ids[1]].map(String::clone)
        );
    }
}
