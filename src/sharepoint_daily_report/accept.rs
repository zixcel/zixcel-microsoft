use super::artifact_contract::DepartmentDailyReportV1;
use super::digest::sha256_hex;
use super::receipt::build_receipt;
use super::{
    DailyReportError, MAX_DAILY_REPORT_BYTES, SharePointDailyReportReceipt,
    SharePointDailyReportRequest, build_daily_report_plan,
};

/// Validates host-supplied bytes offline and returns a content-free receipt.
pub fn accept_daily_report_artifact(
    request: &SharePointDailyReportRequest,
    artifact_bytes: &[u8],
) -> Result<SharePointDailyReportReceipt, DailyReportError> {
    request.validate()?;
    let byte_count = u64::try_from(artifact_bytes.len()).map_err(|_| {
        DailyReportError::new("artifact", "artifact length cannot be represented as bytes")
    })?;
    if byte_count == 0 {
        return Err(DailyReportError::new(
            "artifact",
            "artifact must not be empty",
        ));
    }
    if byte_count > request.max_bytes || byte_count > MAX_DAILY_REPORT_BYTES {
        return Err(DailyReportError::new(
            "artifact",
            "artifact exceeds the request max_bytes boundary",
        ));
    }
    let report: DepartmentDailyReportV1 = serde_json::from_slice(artifact_bytes).map_err(|_| {
        DailyReportError::new(
            "artifact",
            "artifact must satisfy the closed department daily-report v1 JSON shape",
        )
    })?;
    report.validate()?;
    match_expected_scope(request, &report)?;
    let plan = build_daily_report_plan(request)?;
    build_receipt(
        request,
        report,
        plan.plan_id,
        sha256_hex(artifact_bytes),
        byte_count,
    )
}

fn match_expected_scope(
    request: &SharePointDailyReportRequest,
    report: &DepartmentDailyReportV1,
) -> Result<(), DailyReportError> {
    if report.schema != request.expected_schema {
        return Err(mismatch("artifact.schema", "expected_schema"));
    }
    if report.operational_report_id != request.artifact_id {
        return Err(mismatch(
            "artifact.operational_report_id",
            "requested artifact_id",
        ));
    }
    if report.headquarters_id != request.expected_artifact.headquarters_id {
        return Err(mismatch("artifact.headquarters_id", "expected_artifact"));
    }
    if report.department_id != request.expected_artifact.department_id {
        return Err(mismatch("artifact.department_id", "expected_artifact"));
    }
    if report.reporting_team_id != request.expected_artifact.reporting_team_id {
        return Err(mismatch("artifact.reporting_team_id", "expected_artifact"));
    }
    if report.classification != request.expected_artifact.classification {
        return Err(mismatch("artifact.classification", "expected_artifact"));
    }
    if report.customer_data != request.expected_artifact.customer_data {
        return Err(mismatch("artifact.customer_data", "expected_artifact"));
    }
    Ok(())
}

fn mismatch(field: &'static str, expected: &'static str) -> DailyReportError {
    let message = match expected {
        "expected_schema" => "artifact schema does not match the requested expected_schema",
        "requested artifact_id" => "operational_report_id must equal the requested artifact_id",
        _ => "artifact scope does not match expected_artifact",
    };
    DailyReportError::new(field, message)
}
