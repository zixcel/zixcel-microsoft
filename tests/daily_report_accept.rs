use serde_json::{Value, json};
use zixcel_microsoft::sharepoint_daily_report::{
    accept_daily_report_artifact, parse_daily_report_request,
};

const REQUEST: &str = include_str!("../examples/sharepoint-daily-report-request-v1.json");
const ARTIFACT: &[u8] = include_bytes!("../examples/department-daily-report-v1.json");

#[test]
fn exact_bytes_produce_a_stable_content_free_receipt() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let receipt = accept_daily_report_artifact(&request, ARTIFACT).expect("receipt");
    let repeated = accept_daily_report_artifact(&request, ARTIFACT).expect("receipt");
    assert_eq!(receipt, repeated);
    assert_eq!(receipt.artifact.observed_schema, request.expected_schema);
    assert_eq!(receipt.artifact.byte_count, ARTIFACT.len() as u64);
    assert_eq!(receipt.artifact.digest_sha256.len(), 64);
    assert!(!receipt.safety.network_calls_performed);
    assert!(!receipt.safety.artifact_content_persisted);

    let serialized = serde_json::to_string(&receipt).expect("receipt JSON");
    for forbidden in [
        "drive_root_relative_path",
        "site_ref",
        "drive_ref",
        "owner_account_id",
        "summary",
        "Offline accept boundary verification fixture.",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn receipt_identity_changes_when_exact_bytes_change() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let baseline = accept_daily_report_artifact(&request, ARTIFACT).expect("receipt");
    let mut changed = ARTIFACT.to_vec();
    changed.push(b' ');
    let changed = accept_daily_report_artifact(&request, &changed).expect("receipt");
    assert_ne!(baseline.receipt_id, changed.receipt_id);
    assert_ne!(
        baseline.artifact.digest_sha256,
        changed.artifact.digest_sha256
    );
}

#[test]
fn every_expected_scope_field_is_bound() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let mismatches = [
        ("headquarters_id", json!("another-headquarters")),
        ("department_id", json!("another-department")),
        ("reporting_team_id", json!("another-team")),
        ("classification", json!("restricted-sensitive")),
        ("customer_data", json!(true)),
    ];
    for (field, replacement) in mismatches {
        let mut artifact: Value = serde_json::from_slice(ARTIFACT).expect("fixture");
        artifact[field] = replacement;
        let bytes = serde_json::to_vec(&artifact).expect("artifact");
        let error = accept_daily_report_artifact(&request, &bytes).expect_err("scope mismatch");
        assert_eq!(error.field, format!("artifact.{field}"));
    }
}

#[test]
fn closed_artifact_contract_rejects_bad_shape_date_and_duplicates() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let mut artifact: Value = serde_json::from_slice(ARTIFACT).expect("fixture");
    artifact["unexpected"] = Value::Bool(true);
    assert!(accept(&request, &artifact).is_err());
    artifact
        .as_object_mut()
        .expect("object")
        .remove("unexpected");
    artifact["reporting_date"] = json!("2026-02-30");
    assert!(accept(&request, &artifact).is_err());
    artifact["reporting_date"] = json!("2026-07-24");
    artifact["source_task_ids"] = json!(["duplicate", "duplicate"]);
    assert!(accept(&request, &artifact).is_err());
}

fn accept(
    request: &zixcel_microsoft::sharepoint_daily_report::SharePointDailyReportRequest,
    value: &Value,
) -> Result<
    zixcel_microsoft::sharepoint_daily_report::SharePointDailyReportReceipt,
    zixcel_microsoft::sharepoint_daily_report::DailyReportError,
> {
    accept_daily_report_artifact(request, &serde_json::to_vec(value).expect("JSON"))
}
