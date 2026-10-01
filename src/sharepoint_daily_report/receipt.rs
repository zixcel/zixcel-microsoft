use serde::Serialize;

use crate::{CONNECTOR, PROVIDER};

use super::artifact_contract::DepartmentDailyReportV1;
use super::digest::stable_id;
use super::{DAILY_REPORT_RECEIPT_SCHEMA, DailyReportError, SharePointDailyReportRequest};

/// Validated scope and digest; source locators and body content are excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedDailyReportArtifact {
    pub artifact_id: String,
    pub headquarters_id: String,
    pub department_id: String,
    pub reporting_team_id: String,
    pub expected_schema: String,
    pub observed_schema: String,
    pub media_type: String,
    pub byte_count: u64,
    pub digest_sha256: String,
    pub classification: super::DepartmentDailyReportClassification,
    pub customer_data: bool,
}

/// Evidence about effects performed while accepting local bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DailyReportReceiptSafety {
    pub scope: &'static str,
    pub source_mode: &'static str,
    pub offline_validation_performed: bool,
    pub network_calls_performed: bool,
    pub redirects_followed: bool,
    pub credentials_resolved: bool,
    pub artifact_content_persisted: bool,
}

/// Content-free receipt for a contract-valid host-supplied artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharePointDailyReportReceipt {
    pub schema: &'static str,
    pub receipt_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub artifact_id: String,
    pub plan_id: String,
    pub provider: &'static str,
    pub connector: &'static str,
    pub status: &'static str,
    pub artifact: AcceptedDailyReportArtifact,
    pub safety: DailyReportReceiptSafety,
}

#[derive(Serialize)]
struct DailyReportReceiptSeed<'a> {
    schema: &'static str,
    request_id: &'a str,
    correlation_id: &'a str,
    artifact_id: &'a str,
    plan_id: &'a str,
    digest_sha256: &'a str,
    byte_count: u64,
}

pub(crate) fn build_receipt(
    request: &SharePointDailyReportRequest,
    report: DepartmentDailyReportV1,
    plan_id: String,
    digest_sha256: String,
    byte_count: u64,
) -> Result<SharePointDailyReportReceipt, DailyReportError> {
    let seed = DailyReportReceiptSeed {
        schema: DAILY_REPORT_RECEIPT_SCHEMA,
        request_id: &request.request_id,
        correlation_id: &request.correlation_id,
        artifact_id: &request.artifact_id,
        plan_id: &plan_id,
        digest_sha256: &digest_sha256,
        byte_count,
    };
    Ok(SharePointDailyReportReceipt {
        schema: DAILY_REPORT_RECEIPT_SCHEMA,
        receipt_id: stable_id("receipt-microsoft-sharepoint", &seed)?,
        request_id: request.request_id.clone(),
        correlation_id: request.correlation_id.clone(),
        artifact_id: request.artifact_id.clone(),
        plan_id,
        provider: PROVIDER,
        connector: CONNECTOR,
        status: "accepted",
        artifact: AcceptedDailyReportArtifact {
            artifact_id: request.artifact_id.clone(),
            headquarters_id: request.expected_artifact.headquarters_id.clone(),
            department_id: request.expected_artifact.department_id.clone(),
            reporting_team_id: request.expected_artifact.reporting_team_id.clone(),
            expected_schema: request.expected_schema.clone(),
            observed_schema: report.schema,
            media_type: request.media_type.clone(),
            byte_count,
            digest_sha256,
            classification: request.expected_artifact.classification,
            customer_data: request.expected_artifact.customer_data,
        },
        safety: DailyReportReceiptSafety {
            scope: "zixcel-microsoft-process",
            source_mode: "host-supplied-local-artifact",
            offline_validation_performed: true,
            network_calls_performed: false,
            redirects_followed: false,
            credentials_resolved: false,
            artifact_content_persisted: false,
        },
    })
}
