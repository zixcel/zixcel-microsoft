use serde::Serialize;

use super::ExpectedDailyReportArtifact;

/// Opaque SharePoint source locator retained only in the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharePointArtifactSource {
    pub site_ref: String,
    pub drive_ref: String,
    pub drive_root_relative_path: String,
}

/// Contract the host-supplied artifact must satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DailyReportArtifactContract {
    pub artifact_id: String,
    pub expected_schema: String,
    pub expected_artifact: ExpectedDailyReportArtifact,
    pub media_type: String,
    pub max_bytes: u64,
}

/// One stage across the planner, host executor, and offline accept boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DailyReportPlanStep {
    pub sequence: u32,
    pub action: &'static str,
    pub execution_boundary: &'static str,
    pub target: String,
    pub effect: &'static str,
    pub network_required: bool,
    pub performed: bool,
}

/// Explicit evidence about effects performed during planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DailyReportPlanSafety {
    pub scope: &'static str,
    pub network_calls_performed: bool,
    pub redirects_followed: bool,
    pub credentials_resolved: bool,
    pub host_execution_required: bool,
    pub host_redirect_policy: &'static str,
    pub offline_accept_required: bool,
}

/// Deterministic acquisition plan; it is not an execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharePointDailyReportPlan {
    pub schema: &'static str,
    pub plan_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub artifact_id: String,
    pub provider: &'static str,
    pub connector: &'static str,
    pub capability: &'static str,
    pub mode: &'static str,
    pub source: SharePointArtifactSource,
    pub artifact_contract: DailyReportArtifactContract,
    pub steps: Vec<DailyReportPlanStep>,
    pub safety: DailyReportPlanSafety,
}

#[derive(Serialize)]
pub(crate) struct DailyReportPlanSeed<'a> {
    pub schema: &'static str,
    pub request_id: &'a str,
    pub correlation_id: &'a str,
    pub artifact_id: &'a str,
    pub provider: &'static str,
    pub connector: &'static str,
    pub capability: &'static str,
    pub mode: &'static str,
    pub source: &'a SharePointArtifactSource,
    pub artifact_contract: &'a DailyReportArtifactContract,
    pub steps: &'a [DailyReportPlanStep],
    pub safety: &'a DailyReportPlanSafety,
}
