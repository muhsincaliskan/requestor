use super::{CloudItem, PostmanCloudClient, PostmanCloudError};
use crate::migration::MigrationReport;

const SKIPPED: &str = "skipped because the import stopped:";

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
    while let Some((done, (item, kind))) = steps.next() {
        progress(MigrationProgress {
            done,
            total,
            current: item.name.clone(),
        });
        if let Err(error) = kind.fetch_into(client, item, &mut report) {
            report.fail(&item.name, error.to_string());
            if error.stops_migration() {
                for (_, (skipped, _)) in steps.by_ref() {
                    report.fail(&skipped.name, format!("{SKIPPED} {error}"));
                }
            }
        }
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
