use std::collections::HashSet;
use std::path::PathBuf;

use crate::collection::Collection;
use crate::environment::Environment;
use crate::identity::random_uuid;

pub(super) trait StoredDocument {
    fn stored_id(&self) -> Option<&str>;
    fn assign_fresh_id(&mut self);
}

impl StoredDocument for Collection {
    fn stored_id(&self) -> Option<&str> {
        self.info.postman_id.as_deref()
    }

    fn assign_fresh_id(&mut self) {
        self.info.postman_id = Some(random_uuid());
    }
}

impl StoredDocument for Environment {
    fn stored_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn assign_fresh_id(&mut self) {
        self.id = Some(random_uuid());
    }
}

pub(super) struct LoadedDocument<T> {
    pub path: PathBuf,
    pub document: T,
}

pub(super) struct Adoption<T> {
    pub settled: Vec<LoadedDocument<T>>,
    pub misplaced: Vec<LoadedDocument<T>>,
}

impl<T: StoredDocument> Adoption<T> {
    pub(super) fn of(loaded: Vec<LoadedDocument<T>>, home_of: impl Fn(&str) -> PathBuf) -> Self {
        let is_home = |entry: &LoadedDocument<T>| {
            entry
                .document
                .stored_id()
                .is_some_and(|id| home_of(id) == entry.path)
        };
        let (settled, mut misplaced): (Vec<_>, Vec<_>) = loaded.into_iter().partition(is_home);
        let mut taken: HashSet<PathBuf> = settled.iter().map(|entry| entry.path.clone()).collect();
        for entry in &mut misplaced {
            let claimed = entry.document.stored_id().map(&home_of);
            if !claimed.is_some_and(|home| taken.insert(home)) {
                entry.document.assign_fresh_id();
                taken.extend(entry.document.stored_id().map(&home_of));
            }
        }
        Adoption { settled, misplaced }
    }
}
