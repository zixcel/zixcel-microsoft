use serde_json::{Value, json};
use zix_native_provider::Request;
fn sample() -> Value {
    serde_json::from_str(include_str!("observation.json")).unwrap()
}
fn request() -> Request {
    serde_json::from_value(json!({"schema":"zix-provider/v1","id":"0".repeat(32),
    "operation":"users.read","arguments":sample()["arguments"],"limits":{"timeout_ms":5000,"maximum_response_bytes":65536}})).unwrap()
}
#[test]
fn official_route_facts_prepare_exact_scoped_request_and_project_synthetic_data() {
    let provider = zixcel_microsoft_cli::provider().unwrap();
    let req = request();
    let s = sample();
    assert_eq!(
        provider.prepare(&req).unwrap().url,
        s["url"].as_str().unwrap()
    );
    let observed = provider
        .project(&req, &serde_json::to_vec(&s["raw"]).unwrap())
        .unwrap();
    assert_eq!(observed["completeness"], "single_response");
    assert_eq!(observed["permission_state"], "not_inferred");
    assert!(!observed.to_string().contains("unselected-private-field"));
}
#[test]
fn invalid_arguments_unknown_mutations_and_wrong_response_identity_are_rejected() {
    let provider = zixcel_microsoft_cli::provider().unwrap();
    let mut req = request();
    req.arguments
        .insert("url".into(), "https://other.example.invalid".into());
    assert!(provider.prepare(&req).is_err());
    let mut req = request();
    req.operation = "unreviewed.delete".into();
    assert!(provider.prepare(&req).is_err());
    assert!(provider.project(&request(), b"{}").is_err());
}
#[test]
fn actual_native_manifest_and_missing_credential_protocol_need_no_node_or_account() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let output = Command::new(env!("CARGO_BIN_EXE_zix-microsoft"))
        .env_clear()
        .arg("--manifest")
        .output()
        .unwrap();
    assert!(output.status.success());
    let m: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(m["namespace"], "microsoft");
    assert!(
        m["commands"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["effect"] == "read" || c["effect"] == "operator-write")
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_zix-microsoft"))
        .env_clear()
        .arg("--zix-provider-v2")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            &serde_json::to_vec(&{
                let mut value = serde_json::to_value(request()).unwrap();
                value["schema"] = m["protocol"].clone();
                let bytes = std::fs::read(env!("CARGO_BIN_EXE_zix-microsoft")).unwrap();
                value["binding"] =
                    serde_json::to_value(zix_operation_contracts::Binding::for_catalog(
                        m["package"].as_str().unwrap(),
                        m["version"].as_str().unwrap(),
                        &m,
                        &zix_cli::files::digest(&bytes),
                    ))
                    .unwrap();
                value
            })
            .unwrap(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["error"], "credential_missing");
    assert!(response.get("result").is_none());
}
