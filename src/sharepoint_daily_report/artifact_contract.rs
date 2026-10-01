use serde::{Deserialize, Serialize};

use super::DailyReportError;
use super::identity::identifier;

/// Information classification preserved in a content-free receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepartmentDailyReportClassification {
    Internal,
    InternalConfidential,
    PersonalConfidential,
    RestrictedSensitive,
    CustomerConfidential,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DepartmentDailyReportMode {
    SimulationSeed,
    Runtime,
    Imported,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DepartmentDailyReportMetrics {
    #[serde(rename = "completed_tasks")]
    _completed_tasks: u64,
    #[serde(rename = "open_tasks")]
    _open_tasks: u64,
    #[serde(rename = "blocked_tasks")]
    _blocked_tasks: u64,
    #[serde(rename = "reported_deliverables")]
    _reported_deliverables: u64,
    #[serde(rename = "open_escalations")]
    _open_escalations: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DepartmentDailyReportSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RequiredNullableIdentifier(Option<String>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DepartmentDailyReportException {
    case_id: String,
    #[serde(rename = "severity")]
    _severity: DepartmentDailyReportSeverity,
    status: String,
    decision_id: RequiredNullableIdentifier,
}

impl DepartmentDailyReportException {
    pub(crate) fn validate(&self) -> Result<(), DailyReportError> {
        identifier("artifact.exceptions.case_id", &self.case_id)?;
        let status_length = self.status.chars().count();
        if status_length == 0 || status_length > 64 {
            return Err(DailyReportError::new(
                "artifact.exceptions.status",
                "must contain between 1 and 64 characters",
            ));
        }
        if let Some(decision_id) = &self.decision_id.0 {
            identifier("artifact.exceptions.decision_id", decision_id)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DepartmentDailyReportV1 {
    pub schema: String,
    pub operational_report_id: String,
    #[serde(rename = "record_mode")]
    pub _record_mode: DepartmentDailyReportMode,
    pub reporting_date: String,
    pub headquarters_id: String,
    pub department_id: String,
    pub reporting_team_id: String,
    pub owner_account_id: String,
    pub window_started_at: String,
    pub window_ended_at: String,
    pub generated_at: String,
    pub source_case_ids: Vec<String>,
    pub source_event_ids: Vec<String>,
    pub source_task_ids: Vec<String>,
    pub source_report_ids: Vec<String>,
    pub source_artifact_ids: Vec<String>,
    pub escalated_decision_ids: Vec<String>,
    #[serde(rename = "metrics")]
    pub _metrics: DepartmentDailyReportMetrics,
    pub exceptions: Vec<DepartmentDailyReportException>,
    pub classification: DepartmentDailyReportClassification,
    pub customer_data: bool,
    pub summary: String,
}
