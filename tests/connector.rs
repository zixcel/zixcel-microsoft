use zixcel_microsoft::{build_plan, doctor, parse_config};

const CONFIG: &str = r#"
schema = "zixcel://microsoft/config/v1"
config_id = "operations"
tenant_ref = "tenant:corporate"
secret_ref = "secret://microsoft/graph/operations"
cloud = "public"
workloads = ["teams", "share-point", "outlook-mail"]
"#;

#[test]
fn workload_order_is_canonical() {
    let alternate = CONFIG.replace(
        "[\"teams\", \"share-point\", \"outlook-mail\"]",
        "[\"outlook-mail\", \"teams\", \"share-point\"]",
    );
    let first = build_plan(&parse_config(CONFIG).expect("config")).expect("plan");
    let second = build_plan(&parse_config(&alternate).expect("config")).expect("plan");
    assert_eq!(first, second);
}

#[test]
fn generic_contract_is_closed_bounded_and_credential_free() {
    assert!(parse_config(&format!("{CONFIG}\nclient_secret = \"no\"\n")).is_err());
    assert!(parse_config(&format!("{CONFIG}\nfuture = true\n")).is_err());
    assert!(parse_config(&" ".repeat(1_048_577)).is_err());
}

#[test]
fn doctor_proves_external_actions_are_unavailable() {
    let report = doctor();
    assert!(!report.network_client_linked);
    assert!(!report.secret_resolution_enabled);
    assert!(!report.execution_enabled);
    assert!(!report.network_calls_performed);
    assert!(!report.redirects_followed);
    assert!(!report.credentials_resolved);
}
