use crate::collection::Collection;
use crate::environment::Environment;
use crate::import::{ImportError, Imported, ImportedRequest};

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationFailure {
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MigrationReport {
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub requests: Vec<ImportedRequest>,
    pub failures: Vec<MigrationFailure>,
    pub warnings: Vec<String>,
}

impl MigrationReport {
    pub fn record(&mut self, source: &str, imported: Result<Imported, ImportError>) {
        match imported {
            Ok(imported) => {
                self.warnings.extend(imported.warnings().iter().cloned());
                match imported {
                    Imported::Collection(imported) => self.collections.push(imported.collection),
                    Imported::Environment(environment) => self.environments.push(environment),
                    Imported::Request(request) => self.requests.push(request),
                }
            }
            Err(error) => self.fail(source, error.to_string()),
        }
    }

    pub fn add_collection(&mut self, collection: Collection) {
        self.collections.push(collection);
    }

    pub fn add_environment(&mut self, environment: Environment) {
        self.environments.push(environment);
    }

    pub fn fail(&mut self, source: &str, reason: impl Into<String>) {
        self.failures.push(MigrationFailure {
            source: source.to_string(),
            reason: reason.into(),
        });
    }

    pub fn imported_count(&self) -> usize {
        self.collections.len() + self.environments.len() + self.requests.len()
    }
}
