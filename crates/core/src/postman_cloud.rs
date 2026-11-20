mod api_client;
mod error;
mod migration;
mod responses;
mod retry;
mod workspace;

pub use api_client::{PostmanApiClient, POSTMAN_API_URL};
pub use error::PostmanCloudError;
pub use migration::{migrate_workspace, MigrationProgress};
pub use workspace::{CloudItem, CloudWorkspace, WorkspaceContents};

use crate::collection::Collection;
use crate::environment::Environment;

pub trait PostmanCloudClient {
    fn workspaces(&self) -> Result<Vec<CloudWorkspace>, PostmanCloudError>;
    fn workspace(&self, id: &str) -> Result<WorkspaceContents, PostmanCloudError>;
    fn collection(&self, uid: &str) -> Result<Collection, PostmanCloudError>;
    fn environment(&self, uid: &str) -> Result<Environment, PostmanCloudError>;
}

#[cfg(test)]
mod tests;
