use crate::{CONNECTOR, PROVIDER};

use super::digest::stable_id;
use super::plan_types::{
    DailyReportArtifactContract, DailyReportPlanSafety, DailyReportPlanSeed, DailyReportPlanStep,
    SharePointArtifactSource, SharePointDailyReportPlan,
};
use super::{DAILY_REPORT_PLAN_SCHEMA, DailyReportError, SharePointDailyReportRequest};

/// Describes acquisition and offline acceptance without performing either.
pub fn build_daily_report_plan(
    request: &SharePointDailyReportRequest,
) -> Result<SharePointDailyReportPlan, DailyReportError> {
    request.validate()?;
    let source = SharePointArtifactSource {
        site_ref: request.site_ref.clone(),
        drive_ref: request.drive_ref.clone(),
        drive_root_relative_path: request.drive_root_relative_path.clone(),
    };
    let contract = DailyReportArtifactContract {
        artifact_id: request.artifact_id.clone(),
        expected_schema: request.expected_schema.clone(),
        expected_artifact: request.expected_artifact.clone(),
        media_type: request.media_type.clone(),
        max_bytes: request.max_bytes,
    };
    let steps = plan_steps(request);
    let safety = DailyReportPlanSafety {
        scope: "zixcel-microsoft-process",
        network_calls_performed: false,
        redirects_followed: false,
        credentials_resolved: false,
        host_execution_required: true,
        host_redirect_policy: "reject",
        offline_accept_required: true,
    };
    let seed = DailyReportPlanSeed {
        schema: DAILY_REPORT_PLAN_SCHEMA,
        request_id: &request.request_id,
        correlation_id: &request.correlation_id,
        artifact_id: &request.artifact_id,
        provider: PROVIDER,
        connector: CONNECTOR,
        capability: "sharepoint-department-daily-report-read",
        mode: "observe",
        source: &source,
        artifact_contract: &contract,
        steps: &steps,
        safety: &safety,
    };
    Ok(SharePointDailyReportPlan {
        schema: DAILY_REPORT_PLAN_SCHEMA,
        plan_id: stable_id("plan-microsoft-sharepoint", &seed)?,
        request_id: request.request_id.clone(),
        correlation_id: request.correlation_id.clone(),
        artifact_id: request.artifact_id.clone(),
        provider: PROVIDER,
        connector: CONNECTOR,
        capability: "sharepoint-department-daily-report-read",
        mode: "observe",
        source,
        artifact_contract: contract,
        steps,
        safety,
    })
}

fn plan_steps(request: &SharePointDailyReportRequest) -> Vec<DailyReportPlanStep> {
    [
        (1, "validate-versioned-request", "zixcel-microsoft", false),
        (2, "host-fetch-sharepoint-drive-item", "host-executor", true),
        (3, "host-stage-local-json-artifact", "host-executor", false),
        (4, "offline-validate-and-receipt", "zixcel-microsoft", false),
    ]
    .into_iter()
    .map(
        |(sequence, action, execution_boundary, network_required)| DailyReportPlanStep {
            sequence,
            action,
            execution_boundary,
            target: if sequence == 1 {
                request.request_id.clone()
            } else {
                request.artifact_id.clone()
            },
            effect: if sequence == 2 { "observe" } else { "none" },
            network_required,
            performed: false,
        },
    )
    .collect()
}
