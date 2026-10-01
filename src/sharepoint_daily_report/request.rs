use serde::{Deserialize, Serialize};

use super::artifact_contract::DepartmentDailyReportClassification;
use super::identity::{contract_reference, identifier, opaque_reference};
use super::path_safety::drive_relative_path;
use super::{
    DAILY_REPORT_REQUEST_SCHEMA, DailyReportError, JSON_MEDIA_TYPE, MAX_DAILY_REPORT_BYTES,
    MAX_DAILY_REPORT_REQUEST_BYTES,
};

/// Closed scope expected in the host-supplied daily report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDailyReportArtifact {
    pub headquarters_id: String,
    pub department_id: String,
    pub reporting_team_id: String,
    pub classification: DepartmentDailyReportClassification,
    pub customer_data: bool,
}

impl ExpectedDailyReportArtifact {
    pub(crate) fn validate(&self) -> Result<(), DailyReportError> {
        for (field, value) in [
            ("expected_artifact.headquarters_id", &self.headquarters_id),
            ("expected_artifact.department_id", &self.department_id),
            (
                "expected_artifact.reporting_team_id",
                &self.reporting_team_id,
            ),
        ] {
            identifier(field, value)?;
        }
        Ok(())
    }
}

/// Versioned request passed to planning and later offline acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharePointDailyReportRequest {
    pub schema: String,
    pub request_id: String,
    pub correlation_id: String,
    pub artifact_id: String,
    pub site_ref: String,
    pub drive_ref: String,
    pub drive_root_relative_path: String,
    pub expected_schema: String,
    pub expected_artifact: ExpectedDailyReportArtifact,
    pub media_type: String,
    pub max_bytes: u64,
}

impl SharePointDailyReportRequest {
    /// Validates transport identity, location safety, scope, and byte limits.
    pub fn validate(&self) -> Result<(), DailyReportError> {
        if self.schema != DAILY_REPORT_REQUEST_SCHEMA {
            return Err(DailyReportError::new(
                "schema",
                "expected zixcel://microsoft/sharepoint-daily-report-request/v1",
            ));
        }
        identifier("request_id", &self.request_id)?;
        identifier("correlation_id", &self.correlation_id)?;
        identifier("artifact_id", &self.artifact_id)?;
        opaque_reference("site_ref", &self.site_ref, "site:")?;
        opaque_reference("drive_ref", &self.drive_ref, "drive:")?;
        drive_relative_path(&self.drive_root_relative_path)?;
        contract_reference("expected_schema", &self.expected_schema)?;
        self.expected_artifact.validate()?;
        if self.media_type != JSON_MEDIA_TYPE {
            return Err(DailyReportError::new(
                "media_type",
                "only application/json is accepted",
            ));
        }
        if self.max_bytes == 0 || self.max_bytes > MAX_DAILY_REPORT_BYTES {
            return Err(DailyReportError::new(
                "max_bytes",
                "must be between 1 and 1048576 bytes",
            ));
        }
        Ok(())
    }
}

/// Parses a bounded JSON request with no extension fields.
pub fn parse_daily_report_request(
    source: &str,
) -> Result<SharePointDailyReportRequest, DailyReportError> {
    if u64::try_from(source.len()).unwrap_or(u64::MAX) > MAX_DAILY_REPORT_REQUEST_BYTES {
        return Err(DailyReportError::new(
            "request",
            "request document exceeds 65536 bytes",
        ));
    }
    let request: SharePointDailyReportRequest = serde_json::from_str(source).map_err(|_| {
        DailyReportError::new(
            "request",
            "invalid JSON or fields do not match the daily-report request v1 schema",
        )
    })?;
    request.validate()?;
    Ok(request)
}
