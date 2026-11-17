use std::collections::HashSet;

use super::{Auth, Collection, Item, Request, RequestSpec, Variable};
use crate::identity::random_uuid;

const UNNAMED_ITEM: &str = "Untitled";
const COPY_SUFFIX: &str = " Copy";

impl Collection {
    pub fn containing(item: Item) -> Self {
        let mut collection = Collection::new(item.display_name());
        collection.item.push(item);
        collection.ensure_ids();
        collection
    }

    pub fn id(&self) -> &str {
        self.info.postman_id.as_deref().unwrap_or_default()
    }

    pub fn ensure_ids(&mut self) {
        if self.info.postman_id.is_none() {
            self.info.postman_id = Some(random_uuid());
        }
        let mut seen = HashSet::new();
        for item in &mut self.item {
            item.assign_unique_ids(&mut seen);
        }
    }

    pub fn find_item(&self, id: &str) -> Option<&Item> {
        find_in(&self.item, id)
    }

    pub fn find_item_mut(&mut self, id: &str) -> Option<&mut Item> {
        find_in_mut(&mut self.item, id)
    }

    pub fn add_item(&mut self, parent: Option<&str>, mut item: Item) -> Option<String> {
        let mut seen = self.item_ids();
        item.assign_unique_ids(&mut seen);
        let id = item.id.clone();
        self.children_of_mut(parent)?.push(item);
        id
    }

    pub fn remove_item(&mut self, id: &str) -> Option<Item> {
        remove_from(&mut self.item, id)
    }

    pub fn move_item(&mut self, id: &str, parent: Option<&str>, index: usize) -> bool {
        if !self.can_move(id, parent) {
            return false;
        }
        let Some(item) = self.remove_item(id) else {
            return false;
        };
        match self.children_of_mut(parent) {
            Some(children) => {
                children.insert(index.min(children.len()), item);
                true
            }
            None => false,
        }
    }

    pub fn duplicate_item(&mut self, id: &str) -> Option<String> {
        let trail = self.ancestors_of(id)?;
        let parent = trail.last().and_then(|folder| folder.id.clone());
        let copy = self.find_item(id)?.duplicated();
        let copy_id = copy.id.clone();
        let siblings = self.children_of_mut(parent.as_deref())?;
        let position = siblings.iter().position(|item| item.has_id(id))?;
        siblings.insert(position + 1, copy);
        copy_id
    }

    pub fn ancestors_of(&self, id: &str) -> Option<Vec<&Item>> {
        let mut trail = Vec::new();
        trail_to(&self.item, id, &mut trail).then_some(trail)
    }

    pub fn inherited_auth(&self, id: &str) -> Option<&Auth> {
        let trail = self.ancestors_of(id)?;
        trail
            .iter()
            .rev()
            .find_map(|folder| folder.auth.as_ref())
            .or(self.auth.as_ref())
    }

    pub fn inherited_variables(&self, id: &str) -> Vec<&Variable> {
        let trail = self.ancestors_of(id).unwrap_or_default();
        self.variable
            .iter()
            .flatten()
            .chain(
                trail
                    .into_iter()
                    .flat_map(|folder| folder.variable.iter().flatten()),
            )
            .collect()
    }

    pub fn folder_paths(&self, separator: &str) -> Vec<(String, String)> {
        let mut paths = Vec::new();
        collect_folder_paths(&self.item, "", separator, &mut paths);
        paths
    }

    fn item_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();
        for item in &self.item {
            item.collect_ids(&mut ids);
        }
        ids
    }

    fn can_move(&self, id: &str, parent: Option<&str>) -> bool {
        let Some(item) = self.find_item(id) else {
            return false;
        };
        match parent {
            None => true,
            Some(parent_id) => {
                item.find(parent_id).is_none()
                    && self.find_item(parent_id).is_some_and(Item::is_folder)
            }
        }
    }

    fn children_of_mut(&mut self, parent: Option<&str>) -> Option<&mut Vec<Item>> {
        match parent {
            None => Some(&mut self.item),
            Some(id) => self.find_item_mut(id)?.item.as_mut(),
        }
    }
}

impl Item {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(UNNAMED_ITEM)
    }

    pub fn children(&self) -> &[Item] {
        self.item.as_deref().unwrap_or_default()
    }

    pub fn has_id(&self, id: &str) -> bool {
        self.id.as_deref() == Some(id)
    }

    pub fn full_request(&self) -> Option<Request> {
        self.request.as_ref().map(RequestSpec::to_request)
    }

    pub fn replace_request(&mut self, request: Request) {
        self.request = Some(RequestSpec::Full(Box::new(request)));
    }

    fn assign_unique_ids(&mut self, seen: &mut HashSet<String>) {
        let unique = self.id.as_ref().is_some_and(|id| seen.insert(id.clone()));
        if !unique {
            let fresh = random_uuid();
            seen.insert(fresh.clone());
            self.id = Some(fresh);
        }
        for child in self.item.iter_mut().flatten() {
            child.assign_unique_ids(seen);
        }
    }

    fn collect_ids(&self, ids: &mut HashSet<String>) {
        ids.extend(self.id.clone());
        for child in self.children() {
            child.collect_ids(ids);
        }
    }

    fn find(&self, id: &str) -> Option<&Item> {
        if self.has_id(id) {
            return Some(self);
        }
        find_in(self.children(), id)
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut Item> {
        if self.has_id(id) {
            return Some(self);
        }
        find_in_mut(self.item.as_deref_mut().unwrap_or_default(), id)
    }

    fn duplicated(&self) -> Item {
        let mut copy = self.clone();
        copy.name = Some(format!("{}{COPY_SUFFIX}", self.display_name()));
        copy.forget_ids();
        copy.assign_unique_ids(&mut HashSet::new());
        copy
    }

    fn forget_ids(&mut self) {
        self.id = None;
        self.item.iter_mut().flatten().for_each(Item::forget_ids);
    }
}

fn find_in<'a>(items: &'a [Item], id: &str) -> Option<&'a Item> {
    items.iter().find_map(|item| item.find(id))
}

fn find_in_mut<'a>(items: &'a mut [Item], id: &str) -> Option<&'a mut Item> {
    items.iter_mut().find_map(|item| item.find_mut(id))
}

fn remove_from(items: &mut Vec<Item>, id: &str) -> Option<Item> {
    if let Some(position) = items.iter().position(|item| item.has_id(id)) {
        return Some(items.remove(position));
    }
    items
        .iter_mut()
        .filter_map(|item| item.item.as_mut())
        .find_map(|children| remove_from(children, id))
}

fn trail_to<'a>(items: &'a [Item], id: &str, trail: &mut Vec<&'a Item>) -> bool {
    for item in items {
        if item.has_id(id) {
            return true;
        }
        trail.push(item);
        if trail_to(item.children(), id, trail) {
            return true;
        }
        trail.pop();
    }
    false
}

fn collect_folder_paths(
    items: &[Item],
    prefix: &str,
    separator: &str,
    paths: &mut Vec<(String, String)>,
) {
    for folder in items.iter().filter(|item| item.is_folder()) {
        let path = format!("{prefix}{}", folder.display_name());
        if let Some(id) = &folder.id {
            paths.push((id.clone(), path.clone()));
        }
        collect_folder_paths(
            folder.children(),
            &format!("{path}{separator}"),
            separator,
            paths,
        );
    }
}

#[cfg(test)]
mod tests;
