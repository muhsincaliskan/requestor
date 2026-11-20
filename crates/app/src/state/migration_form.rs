use requestor_core::migration::{MigrationFailure, MigrationReport};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MigrationForm {
    job: Option<u64>,
    pub outcome: Option<MigrationOutcome>,
    pub include_credentials: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationOutcome {
    pub summary: String,
    pub failures: Vec<MigrationFailure>,
    pub warnings: Vec<String>,
}

impl MigrationOutcome {
    pub fn from(report: &MigrationReport) -> Self {
        let mut summary = format!(
            "Imported {} and {}",
            counted(report.collections.len(), "collection"),
            counted(report.environments.len(), "environment")
        );
        if !report.requests.is_empty() {
            summary.push_str(&format!(
                ", and opened {}",
                counted(report.requests.len(), "request")
            ));
        }
        if !report.failures.is_empty() {
            summary.push_str(&format!(
                "; {} could not be imported",
                report.failures.len()
            ));
        }
        MigrationOutcome {
            summary,
            failures: report.failures.clone(),
            warnings: report.warnings.clone(),
        }
    }
}

impl MigrationForm {
    pub fn is_working(&self) -> bool {
        self.job.is_some()
    }

    pub fn runs(&self, job: u64) -> bool {
        self.job == Some(job)
    }

    pub fn start(&mut self, job: u64) {
        self.job = Some(job);
        self.outcome = None;
    }

    pub fn finish(&mut self, job: u64, report: &MigrationReport) {
        if self.job == Some(job) {
            self.job = None;
            self.outcome = Some(MigrationOutcome::from(report));
        }
    }
}

fn counted(count: usize, noun: &str) -> String {
    match count {
        1 => format!("1 {noun}"),
        _ => format!("{count} {noun}s"),
    }
}
