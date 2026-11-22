use std::collections::BTreeMap;
use std::fmt;

use super::Workspace;
use crate::collection::{Collection, Item, Variable};
use crate::environment::Environment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableHome {
    Environment {
        id: String,
        name: String,
    },
    Collection {
        id: String,
        name: String,
    },
    Folder {
        collection_id: String,
        folder_id: String,
        name: String,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct VariableAssignment {
    pub home: VariableHome,
    pub name: String,
    pub value: String,
}

impl fmt::Debug for VariableAssignment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VariableAssignment")
            .field("home", &self.home)
            .field("name", &self.name)
            .field("value_bytes", &self.value.len())
            .finish()
    }
}

impl VariableHome {
    pub fn of_environment(id: &str, environment: &Environment) -> Self {
        VariableHome::Environment {
            id: id.to_string(),
            name: environment.name.clone(),
        }
    }

    pub fn of_collection(collection: &Collection) -> Self {
        VariableHome::Collection {
            id: collection.id().to_string(),
            name: collection.info.name.clone(),
        }
    }

    pub fn of_folder(collection: &Collection, folder: &Item) -> Self {
        VariableHome::Folder {
            collection_id: collection.id().to_string(),
            folder_id: folder.id.clone().unwrap_or_default(),
            name: folder.name.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableHomes {
    defined: BTreeMap<String, VariableHome>,
    fallback: Option<VariableHome>,
}

impl VariableHomes {
    pub fn home_of(&self, name: &str) -> Option<&VariableHome> {
        self.defined.get(name).or(self.fallback.as_ref())
    }

    fn claim<'a>(&mut self, names: impl Iterator<Item = &'a str>, home: &VariableHome) {
        for name in names {
            self.defined.insert(name.to_string(), home.clone());
        }
    }
}

impl Workspace {
    pub fn variable_homes(
        &self,
        collection_id: Option<&str>,
        item_id: Option<&str>,
    ) -> VariableHomes {
        let collection = collection_id.and_then(|id| self.collection(id));
        let environment = self.active_environment_entry();
        let mut homes = VariableHomes::default();
        if let Some(collection) = collection {
            let home = VariableHome::of_collection(collection);
            homes.claim(
                Variable::provided_keys(collection.variable.as_deref().unwrap_or_default()),
                &home,
            );
            for folder in item_id
                .and_then(|id| collection.ancestors_of(id))
                .unwrap_or_default()
            {
                let home = VariableHome::of_folder(collection, folder);
                homes.claim(
                    Variable::provided_keys(folder.variable.as_deref().unwrap_or_default()),
                    &home,
                );
            }
            homes.fallback = Some(home);
        }
        if let Some((id, environment)) = environment {
            let home = VariableHome::of_environment(id, environment);
            homes.claim(environment.provided_keys(), &home);
            homes.fallback = Some(home);
        }
        homes
    }

    pub fn assign_variable(&mut self, assignment: &VariableAssignment) -> bool {
        let VariableAssignment { home, name, value } = assignment;
        match home {
            VariableHome::Environment { id, .. } => self
                .environment_mut(id)
                .map(|environment| environment.assign(name, value))
                .is_some(),
            VariableHome::Collection { id, .. } => self
                .collection_mut(id)
                .map(|collection| Variable::assign_in(&mut collection.variable, name, value))
                .is_some(),
            VariableHome::Folder {
                collection_id,
                folder_id,
                ..
            } => self
                .collection_mut(collection_id)
                .and_then(|collection| collection.find_item_mut(folder_id))
                .map(|folder| Variable::assign_in(&mut folder.variable, name, value))
                .is_some(),
        }
    }

    fn active_environment_entry(&self) -> Option<(&str, &Environment)> {
        let id = self.session.active_environment.as_deref()?;
        Some((id, self.environment(id)?))
    }
}
