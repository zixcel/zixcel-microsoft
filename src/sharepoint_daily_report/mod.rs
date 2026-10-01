//! SharePoint daily-report acquisition planning and offline acceptance.
//!
//! Network retrieval belongs to a separately authorized host executor. This
//! module validates versioned requests and host-supplied local bytes only.

mod accept;
mod acquisition_plan;
mod artifact_contract;
mod artifact_validation;
mod digest;
mod error;
mod identity;
mod path_safety;
mod plan_types;
mod receipt;
mod request;
mod time;

pub use accept::accept_daily_report_artifact;
pub use acquisition_plan::build_daily_report_plan;
pub use artifact_contract::DepartmentDailyReportClassification;
pub use error::DailyReportError;
pub use plan_types::{
    DailyReportArtifactContract, DailyReportPlanSafety, DailyReportPlanStep,
    SharePointArtifactSource, SharePointDailyReportPlan,
};
pub use receipt::{
    AcceptedDailyReportArtifact, DailyReportReceiptSafety, SharePointDailyReportReceipt,
};
pub use request::{
    ExpectedDailyReportArtifact, SharePointDailyReportRequest, parse_daily_report_request,
};

/// Request schema for a single bounded SharePoint artifact.
pub const DAILY_REPORT_REQUEST_SCHEMA: &str =
    "zixcel://microsoft/sharepoint-daily-report-request/v1";
/// Deterministic acquisition-plan schema.
pub const DAILY_REPORT_PLAN_SCHEMA: &str = "zixcel://microsoft/sharepoint-daily-report-plan/v1";
/// Content-free offline acceptance receipt schema.
pub const DAILY_REPORT_RECEIPT_SCHEMA: &str =
    "zixcel://microsoft/sharepoint-daily-report-receipt/v1";
/// Only accepted artifact media type.
pub const JSON_MEDIA_TYPE: &str = "application/json";
/// Hard artifact byte boundary independent of request input.
pub const MAX_DAILY_REPORT_BYTES: u64 = 1_048_576;
/// Hard request-document byte boundary.
pub const MAX_DAILY_REPORT_REQUEST_BYTES: u64 = 65_536;
