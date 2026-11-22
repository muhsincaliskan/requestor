use std::collections::BTreeMap;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableAssignment {
    pub home: VariableHome,
    pub name: String,
    pub value: String,
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

    fn claim(&mut self, names: impl Iterator<Item = String>, home: &VariableHome) {
        for name in names {
            self.defined.insert(name, home.clone());
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
            let home = collection_home(collection);
            homes.claim(enabled_keys(collection.variable.as_ref()), &home);
            for folder in item_id
                .and_then(|id| collection.ancestors_of(id))
                .unwrap_or_default()
            {
                let home = folder_home(collection, folder);
                homes.claim(enabled_keys(folder.variable.as_ref()), &home);
            }
            homes.fallback = Some(home);
        }
        if let Some((id, environment)) = environment {
            let home = VariableHome::Environment {
                id: id.to_string(),
                name: environment.name.clone(),
            };
            homes.claim(environment_keys(environment), &home);
            homes.fallback = Some(home);
        }
        homes
    }

    pub fn assign_variable(&mut self, assignment: &VariableAssignment) -> bool {
        let VariableAssignment { home, name, value } = assignment;
        match home {
            VariableHome::Environment { id, .. } => self
                .environment_mut(id)
                .map(|environment| environment.set_value(name, value))
                .is_some(),
            VariableHome::Collection { id, .. } => self
                .collection_mut(id)
                .map(|collection| set_in(&mut collection.variable, name, value))
                .is_some(),
            VariableHome::Folder {
                collection_id,
                folder_id,
                ..
            } => self
                .collection_mut(collection_id)
                .and_then(|collection| collection.find_item_mut(folder_id))
                .map(|folder| set_in(&mut folder.variable, name, value))
                .is_some(),
        }
    }

    fn active_environment_entry(&self) -> Option<(&str, &Environment)> {
        let id = self.session.active_environment.as_deref()?;
        Some((id, self.environment(id)?))
    }
}

fn collection_home(collection: &Collection) -> VariableHome {
    VariableHome::Collection {
        id: collection.id().to_string(),
        name: collection.info.name.clone(),
    }
}

fn folder_home(collection: &Collection, folder: &Item) -> VariableHome {
    VariableHome::Folder {
        collection_id: collection.id().to_string(),
        folder_id: folder.id.clone().unwrap_or_default(),
        name: folder.name.clone().unwrap_or_default(),
    }
}

fn enabled_keys(variables: Option<&Vec<Variable>>) -> impl Iterator<Item = String> + '_ {
    variables
        .into_iter()
        .flatten()
        .filter(|variable| variable.is_enabled())
        .filter_map(|variable| variable.key.clone())
}

fn environment_keys(environment: &Environment) -> impl Iterator<Item = String> + '_ {
    environment
        .values
        .iter()
        .filter(|entry| entry.is_enabled())
        .map(|entry| entry.key.clone())
}

fn set_in(variables: &mut Option<Vec<Variable>>, name: &str, value: &str) {
    Variable::set_in(variables.get_or_insert_with(Vec::new), name, value);
}
