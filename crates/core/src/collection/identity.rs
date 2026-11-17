use std::collections::HashSet;

use super::{Collection, Item};
use crate::identity::random_uuid;

impl Collection {
    pub fn ensure_ids(&mut self) -> bool {
        let named = self.info.postman_id.is_none();
        if named {
            self.info.postman_id = Some(random_uuid());
        }
        let mut seen = HashSet::new();
        self.item.iter_mut().fold(named, |changed, item| {
            item.assign_unique_ids(&mut seen) | changed
        })
    }

    pub fn claim_unique_id(&mut self, taken: &mut HashSet<String>) -> bool {
        let mut changed = self.ensure_ids();
        if !taken.insert(self.id().to_string()) {
            let fresh = random_uuid();
            taken.insert(fresh.clone());
            self.info.postman_id = Some(fresh);
            changed = true;
        }
        changed
    }

    pub(super) fn item_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();
        for item in &self.item {
            item.collect_ids(&mut ids);
        }
        ids
    }
}

impl Item {
    pub(super) fn assign_unique_ids(&mut self, seen: &mut HashSet<String>) -> bool {
        let unique = self.id.as_ref().is_some_and(|id| seen.insert(id.clone()));
        if !unique {
            let fresh = random_uuid();
            seen.insert(fresh.clone());
            self.id = Some(fresh);
        }
        self.item
            .iter_mut()
            .flatten()
            .fold(!unique, |changed, child| {
                child.assign_unique_ids(seen) | changed
            })
    }

    pub(super) fn collect_ids(&self, ids: &mut HashSet<String>) {
        ids.extend(self.id.clone());
        for child in self.children() {
            child.collect_ids(ids);
        }
    }
}

#[cfg(test)]
mod tests;
