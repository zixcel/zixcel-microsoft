use zixcel_microsoft::sharepoint_daily_report::{
    build_daily_report_plan, parse_daily_report_request,
};

const REQUEST: &str = include_str!("../examples/sharepoint-daily-report-request-v1.json");

#[test]
fn acquisition_plan_is_deterministic_and_non_executing() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let first = build_daily_report_plan(&request).expect("plan");
    let second = build_daily_report_plan(&request).expect("plan");
    assert_eq!(first, second);
    assert_eq!(first.request_id, request.request_id);
    assert!(!first.safety.network_calls_performed);
    assert!(!first.safety.redirects_followed);
    assert!(!first.safety.credentials_resolved);
    assert_eq!(first.safety.host_redirect_policy, "reject");
    assert!(first.steps.iter().all(|step| !step.performed));
}

#[test]
fn plan_identity_binds_location_and_expected_scope() {
    let request = parse_daily_report_request(REQUEST).expect("request");
    let baseline = build_daily_report_plan(&request).expect("plan");
    let mut changed = request.clone();
    changed.drive_root_relative_path =
        "department-reports/finance/2026/07/24/report-finance-20260724.json".to_owned();
    assert_ne!(
        baseline.plan_id,
        build_daily_report_plan(&changed)
            .expect("changed plan")
            .plan_id
    );
    changed = request;
    changed.expected_artifact.department_id = "finance".to_owned();
    assert_ne!(
        baseline.plan_id,
        build_daily_report_plan(&changed)
            .expect("changed plan")
            .plan_id
    );
}

#[test]
fn request_rejects_extensions_urls_unsafe_paths_and_oversize() {
    let unknown = REQUEST.replace("\n}", ",\n  \"access_token\": \"no\"\n}");
    assert!(parse_daily_report_request(&unknown).is_err());
    let mut request = parse_daily_report_request(REQUEST).expect("request");
    request.site_ref = "https://graph.microsoft.com/sites/raw".to_owned();
    assert!(request.validate().is_err());
    for path in [
        "/report.json",
        "../report.json",
        r"reports\one.json",
        "one.txt",
    ] {
        request.drive_root_relative_path = path.to_owned();
        assert!(request.validate().is_err(), "accepted {path}");
    }
    assert!(parse_daily_report_request(&" ".repeat(65_537)).is_err());
}

#[test]
fn bundled_schemas_are_json_documents() {
    for schema in [
        include_str!("../schemas/sharepoint-daily-report-request-v1.schema.json"),
        include_str!("../schemas/sharepoint-daily-report-plan-v1.schema.json"),
        include_str!("../schemas/sharepoint-daily-report-receipt-v1.schema.json"),
        include_str!("../schemas/sharepoint-daily-report-request-validation-v1.schema.json"),
    ] {
        serde_json::from_str::<serde_json::Value>(schema).expect("JSON Schema");
    }
}
