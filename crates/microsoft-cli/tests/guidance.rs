use serde_json::Value;
use std::process::Command;
#[test]
fn metadata_is_credential_free_complete_for_manifest_and_source_owned() {
    let manifest = zixcel_microsoft_cli::manifest().unwrap();
    assert_eq!(manifest.protocol, "zix-provider/v2");
    let catalog = zixcel_microsoft_cli::guidance().unwrap();
    assert!(catalog.validate());
    for c in &manifest.commands {
        assert!(
            catalog
                .entries
                .iter()
                .any(|g| g.operation == c.id && g.selectors.is_empty())
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_zix-microsoft"))
        .arg("--guidance")
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let exported: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(exported, serde_json::to_value(catalog).unwrap());
    assert!(output.stderr.is_empty());
}
#[test]
fn policy_scope_and_exchange_audience_are_explicit_without_blanket_permission_advice() {
    let c = zixcel_microsoft_cli::guidance().unwrap();
    let g = c
        .select(
            "admin.organization",
            &std::collections::BTreeMap::from([("resource".into(), "security_defaults".into())]),
        )
        .unwrap();
    let v = serde_json::to_value(g).unwrap();
    assert!(
        v["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "oauth_scope"
                && r["value"] == "Policy.Read.All"
                && r["subject"] == "graph_credential")
    );
    assert!(!v.to_string().contains("Directory.ReadWrite.All"));
    let g = c.select("exchange.snapshot", &Default::default()).unwrap();
    let v = serde_json::to_value(g).unwrap();
    assert!(
        v["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "audience"
                && r["subject"] == "exchange_credential"
                && r["value"] == "https://outlook.office365.com")
    );
    let g = c
        .select("admin.domain-verification", &Default::default())
        .unwrap();
    assert!(
        g.next_actions
            .iter()
            .any(|a| a.namespace == "domain" && a.operation == "dns.delegation")
    );
}
#[test]
fn relationship_context_stops_before_handler_or_credentials_and_names_missing_input() {
    use std::{io::Write, process::Stdio};
    let binary = env!("CARGO_BIN_EXE_zix-microsoft");
    let manifest = zixcel_microsoft_cli::manifest().unwrap();
    let binding = zix_operation_contracts::Binding::for_catalog(
        &manifest.package,
        &manifest.version,
        &serde_json::to_value(&manifest).unwrap(),
        &zix_cli::files::digest(&std::fs::read(binary).unwrap()),
    );
    let request = serde_json::json!({"schema":"zix-provider/v2","id":"a".repeat(32),"binding":binding,"operation":"admin.organization","arguments":{"tenant":"00000000-0000-0000-0000-000000000001","subject":"00000000-0000-0000-0000-000000000002","resource":"group_members"},"limits":{"timeout_ms":5000,"maximum_response_bytes":65536}});
    let mut child = Command::new(binary)
        .arg("--zix-provider-v2")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"], "operation_context_missing");
    assert_eq!(
        v["diagnostic"]["guide"]["required_arguments"],
        serde_json::json!(["identity"])
    );
    assert_eq!(v["diagnostic"]["state"], "NeedsResolution");
}
