use super::{CloudItem, PostmanCloudClient, PostmanCloudError};
use crate::migration::MigrationReport;

const SKIPPED: &str = "skipped because Postman stopped answering";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationProgress {
    pub done: usize,
    pub total: usize,
    pub current: String,
}

pub fn migrate_workspace(
    client: &impl PostmanCloudClient,
    workspace_id: &str,
    progress: &mut dyn FnMut(MigrationProgress),
) -> Result<MigrationReport, PostmanCloudError> {
    let contents = client.workspace(workspace_id)?;
    let total = contents.item_count();
    let mut report = MigrationReport::default();
    let mut steps = contents
        .collections
        .iter()
        .map(|item| (item, Kind::Collection))
        .chain(
            contents
                .environments
                .iter()
                .map(|item| (item, Kind::Environment)),
        )
        .enumerate();
    for (done, (item, kind)) in steps.by_ref() {
        progress(MigrationProgress {
            done,
            total,
            current: item.name.clone(),
        });
        if let Err(error) = kind.fetch_into(client, item, &mut report) {
            let stops = error.stops_migration();
            report.fail(&item.name, error.to_string());
            if stops {
                break;
            }
        }
    }
    for (_, (item, _)) in steps {
        report.fail(&item.name, SKIPPED);
    }
    Ok(report)
}

enum Kind {
    Collection,
    Environment,
}

impl Kind {
    fn fetch_into(
        &self,
        client: &impl PostmanCloudClient,
        item: &CloudItem,
        report: &mut MigrationReport,
    ) -> Result<(), PostmanCloudError> {
        match self {
            Kind::Collection => report.add_collection(client.collection(&item.uid)?),
            Kind::Environment => report.add_environment(client.environment(&item.uid)?),
        }
        Ok(())
    }
}
