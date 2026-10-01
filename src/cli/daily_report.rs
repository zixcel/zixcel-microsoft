use serde_json::json;
use zixcel_microsoft::sharepoint_daily_report::{
    MAX_DAILY_REPORT_REQUEST_BYTES, accept_daily_report_artifact, build_daily_report_plan,
    parse_daily_report_request,
};

use super::file_input::{read_bounded_file, read_bounded_utf8};
use super::output::print_json;
use super::{expect_no_more, one_path, usage};

pub(super) fn dispatch(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let command = arguments.next().ok_or_else(usage)?;
    match command.as_str() {
        "validate-request" => validate_request(arguments),
        "plan" => plan(arguments),
        "accept" => accept(arguments),
        _ => Err(usage()),
    }
}

fn validate_request(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let request = load_request(&one_path(arguments)?)?;
    print_json(&json!({
        "schema": "zixcel://microsoft/sharepoint-daily-report-request-validation/v1",
        "request_id": request.request_id,
        "correlation_id": request.correlation_id,
        "artifact_id": request.artifact_id,
        "valid": true,
        "network_calls_performed": false,
        "redirects_followed": false,
        "credentials_resolved": false
    }))
}

fn plan(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let request = load_request(&one_path(arguments)?)?;
    let plan = build_daily_report_plan(&request).map_err(|error| error.to_string())?;
    print_json(&plan)
}

fn accept(arguments: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let request_path = arguments.next().ok_or_else(usage)?;
    let artifact_path = arguments.next().ok_or_else(usage)?;
    expect_no_more(arguments)?;
    let request = load_request(&request_path)?;
    let artifact = read_bounded_file(&artifact_path, request.max_bytes, "artifact")?;
    let receipt =
        accept_daily_report_artifact(&request, &artifact).map_err(|error| error.to_string())?;
    print_json(&receipt)
}

fn load_request(
    path: &str,
) -> Result<zixcel_microsoft::sharepoint_daily_report::SharePointDailyReportRequest, String> {
    let source = read_bounded_utf8(path, MAX_DAILY_REPORT_REQUEST_BYTES, "daily-report request")?;
    parse_daily_report_request(&source).map_err(|error| error.to_string())
}
