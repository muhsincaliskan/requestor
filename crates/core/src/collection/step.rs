use crate::prepared::PreparedRequest;
use crate::script::ScriptReport;
use crate::secret::Redactor;

pub const BEFORE_REQUEST_STOPPED: &str = "Before request script stopped: ";

pub struct PreparedStep {
    pub request: PreparedRequest,
    pub redactor: Redactor,
    pub report: ScriptReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepFailure {
    pub reason: String,
    pub report: ScriptReport,
}
