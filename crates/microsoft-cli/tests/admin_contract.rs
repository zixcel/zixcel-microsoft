use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn binding() -> Value {
    let path = env!("CARGO_BIN_EXE_zix-microsoft");
    let r = Command::new(path)
        .arg("--manifest")
        .env_clear()
        .output()
        .unwrap();
    assert!(r.status.success());
    let m: Value = serde_json::from_slice(&r.stdout).unwrap();
    let bytes = std::fs::read(path).unwrap();
    serde_json::to_value(zix_operation_contracts::Binding::for_catalog(
        m["package"].as_str().unwrap(),
        m["version"].as_str().unwrap(),
        &m,
        &zix_cli::files::digest(&bytes),
    ))
    .unwrap()
}
fn invoke(op: &str, args: Value) -> Value {
    let mut c = Command::new(env!("CARGO_BIN_EXE_zix-microsoft"))
        .arg("--zix-provider-v1")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(serde_json::to_string(&json!({"schema":"zix-provider/v1","id":"a".repeat(32),"operation":op,"arguments":args,"binding":binding(),"limits":{"timeout_ms":40000,"maximum_response_bytes":65536}})).unwrap().as_bytes()).unwrap();
    let r = c.wait_with_output().unwrap();
    assert!(
        r.status.success(),
        "native protocol exited {:?}: {} (executable bytes: {})",
        r.status.code(),
        String::from_utf8_lossy(&r.stderr),
        std::fs::metadata(env!("CARGO_BIN_EXE_zix-microsoft"))
            .unwrap()
            .len()
    );
    serde_json::from_slice(&r.stdout).unwrap()
}
#[test]
fn manifest_separates_operator_writes_and_subtracts_credential_inheritance() {
    let m = zixcel_microsoft_cli::manifest().unwrap();
    assert_eq!(m.schema, "zix-extension/v2");
    let apply = m
        .commands
        .iter()
        .find(|c| c.id == "exchange.apply")
        .unwrap();
    assert_eq!(apply.effect, "operator-write");
    for name in ["plan-file", "approval-sha256", "journal-directory"] {
        assert!(apply.arguments.iter().any(|a| a.name == name && a.required));
    }
    let read = m.commands.iter().find(|c| c.id == "admin.domains").unwrap();
    assert_eq!(
        read.environment.as_ref().unwrap().credential_environment,
        vec!["ZIX_MICROSOFT_ACCESS_TOKEN"]
    );
}
#[test]
fn configuration_and_group_selectors_fail_closed_before_credentials() {
    let mut args = json!({"tenant":"11111111-1111-4111-8111-111111111111","subject":"22222222-2222-4222-8222-222222222222","resource":"folder_permissions","mailbox":"33333333-3333-4333-8333-333333333333","folder":"other@example.net:\\Inbox"});
    assert_eq!(
        invoke("exchange.snapshot", args.clone())["error"],
        "invalid_folder_path"
    );
    args["folder"] = json!("\\Inbox");
    assert_eq!(
        invoke("exchange.snapshot", args)["error"],
        "credential_missing"
    );
    assert_eq!(
        invoke(
            "admin.organization",
            json!({"tenant":"11111111-1111-4111-8111-111111111111","subject":"22222222-2222-4222-8222-222222222222","resource":"group_members","identity":"../escape"})
        )["error"],
        "invalid_uuid"
    );
}
#[test]
fn default_dispatcher_timeout_is_accepted_before_credential_boundary() {
    let r = invoke(
        "admin.domains",
        json!({"tenant":"11111111-1111-1111-1111-111111111111","subject":"22222222-2222-2222-2222-222222222222"}),
    );
    assert_eq!(r["error"], "credential_missing");
    let r = invoke(
        "users.read",
        json!({"user":"11111111-1111-1111-1111-111111111111"}),
    );
    assert_eq!(r["error"], "credential_missing");
}
#[test]
fn invalid_tenant_and_dns_host_fail_before_authentication() {
    let r = invoke(
        "admin.domains",
        json!({"tenant":"../host","subject":"22222222-2222-2222-2222-222222222222"}),
    );
    assert_eq!(r["error"], "invalid_uuid");
    let r = invoke(
        "admin.domain-dns",
        json!({"tenant":"11111111-1111-1111-1111-111111111111","subject":"22222222-2222-2222-2222-222222222222","domain":"https://example.org"}),
    );
    assert_eq!(r["error"], "invalid_domain");
}
#[test]
fn doctor_requires_no_account_or_tokens_and_performs_no_installation() {
    let r = invoke("admin.doctor", json!({}));
    assert_eq!(r["result"]["credentials_read"], false);
    assert_eq!(r["result"]["remote_requests"], false);
    assert_eq!(r["result"]["exchange_profile"], "not_configured");
}

#[test]
fn mail_administration_writes_have_exact_approval_inputs_and_auth_has_no_inherited_tokens() {
    let m = zixcel_microsoft_cli::manifest().unwrap();
    assert!(m.commands.len() <= 32);
    assert!(serde_json::to_vec(&m).unwrap().len() <= 16384);
    for group in ["domains", "users", "licenses", "dkim", "mail"] {
        let c = m
            .commands
            .iter()
            .find(|c| c.id == format!("management.{group}.apply"))
            .unwrap();
        assert_eq!(c.effect, "operator-write");
        for name in ["plan-file", "approval-sha256", "journal-directory"] {
            assert!(c.arguments.iter().any(|a| a.name == name && a.required));
        }
    }
    for c in m.commands.iter().filter(|c| c.id.starts_with("auth.")) {
        assert!(
            c.environment
                .as_ref()
                .unwrap()
                .credential_environment
                .is_empty()
        );
    }
    let c = m
        .commands
        .iter()
        .find(|c| c.id == "management.dkim.apply")
        .unwrap();
    assert_eq!(
        c.environment.as_ref().unwrap().credential_environment.len(),
        3
    );
    let c = m
        .commands
        .iter()
        .find(|c| c.id == "management.domains.apply")
        .unwrap();
    assert_eq!(
        c.environment.as_ref().unwrap().credential_environment,
        vec!["ZIX_MICROSOFT_ACCESS_TOKEN"]
    );
}
