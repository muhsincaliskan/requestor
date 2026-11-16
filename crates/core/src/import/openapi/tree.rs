use serde_json::Map;

use crate::collection::{Item, Request, RequestSpec};

#[derive(Default)]
pub struct ItemTree {
    folders: Vec<(String, Vec<Item>)>,
    root: Vec<Item>,
}

impl ItemTree {
    pub fn push(&mut self, folder: Option<&str>, item: Item) {
        match folder {
            Some(name) => self.folder_items(name).push(item),
            None => self.root.push(item),
        }
    }

    fn folder_items(&mut self, name: &str) -> &mut Vec<Item> {
        let index = match self.folders.iter().position(|(known, _)| known == name) {
            Some(index) => index,
            None => {
                self.folders.push((name.to_string(), Vec::new()));
                self.folders.len() - 1
            }
        };
        &mut self.folders[index].1
    }

    pub fn into_items(self) -> Vec<Item> {
        let folders = self.folders.into_iter().map(|(name, items)| Item {
            item: Some(items),
            ..Item::named(name)
        });
        folders.chain(self.root).collect()
    }
}

impl Item {
    pub fn named(name: String) -> Self {
        Item {
            id: None,
            name: Some(name),
            description: None,
            variable: None,
            event: None,
            auth: None,
            item: None,
            request: None,
            response: None,
            extra: Map::new(),
        }
    }

    pub fn for_request(name: String, request: Request) -> Self {
        Item {
            request: Some(RequestSpec::Full(Box::new(request))),
            ..Item::named(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ItemTree;
    use crate::collection::Item;

    #[test]
    fn folders_come_first_and_reuse_names() {
        let mut tree = ItemTree::default();
        tree.push(None, Item::named("loose".to_string()));
        tree.push(Some("pets"), Item::named("a".to_string()));
        tree.push(Some("pets"), Item::named("b".to_string()));
        let items = tree.into_items();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_folder());
        assert_eq!(items[0].item.as_ref().unwrap().len(), 2);
    }
}
