use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn repository_root() -> PathBuf {
    std::env::current_dir().expect("Cargo test working directory")
}

fn binary() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    test_executable
        .parent()
        .and_then(|deps| deps.parent())
        .expect("Cargo target profile directory")
        .join(format!("zixcel-microsoft{}", std::env::consts::EXE_SUFFIX))
}

fn fixture(name: &str) -> PathBuf {
    repository_root().join("examples").join(name)
}

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(arguments)
        .output()
        .expect("CLI must start")
}

#[test]
fn daily_report_plan_cli_is_deterministic() {
    let request = fixture("sharepoint-daily-report-request-v1.json");
    let request = request.to_str().expect("UTF-8 fixture path");
    let first = run(&["sharepoint-daily-report", "plan", request]);
    let second = run(&["sharepoint-daily-report", "plan", request]);
    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);

    let plan: Value = serde_json::from_slice(&first.stdout).expect("plan JSON");
    assert_eq!(
        plan["schema"],
        "zixcel://microsoft/sharepoint-daily-report-plan/v1"
    );
    assert_eq!(plan["safety"]["network_calls_performed"], false);
    assert_eq!(plan["safety"]["redirects_followed"], false);
    assert_eq!(plan["safety"]["credentials_resolved"], false);
}

#[test]
fn daily_report_accept_cli_emits_content_free_receipt() {
    let request = fixture("sharepoint-daily-report-request-v1.json");
    let artifact = fixture("department-daily-report-v1.json");
    let output = run(&[
        "sharepoint-daily-report",
        "accept",
        request.to_str().expect("UTF-8 request path"),
        artifact.to_str().expect("UTF-8 artifact path"),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: Value = serde_json::from_slice(&output.stdout).expect("receipt JSON");
    assert_eq!(
        receipt["schema"],
        "zixcel://microsoft/sharepoint-daily-report-receipt/v1"
    );
    assert_eq!(receipt["status"], "accepted");
    assert_eq!(
        receipt["artifact"]["digest_sha256"],
        "3f2a8c6df93a1ea1c70a614f11cc73b1d8527d572270d99004c9432839a2a003"
    );
    assert_eq!(receipt["safety"]["network_calls_performed"], false);
    assert_eq!(receipt["safety"]["redirects_followed"], false);
    assert_eq!(receipt["safety"]["credentials_resolved"], false);
    assert!(receipt.get("source").is_none());
    assert_eq!(
        receipt["artifact"]["headquarters_id"],
        "example-headquarters"
    );
    assert_eq!(receipt["artifact"]["reporting_team_id"], "example-team-01");
    let serialized = serde_json::to_string(&receipt).expect("serialized receipt");
    for forbidden in [
        "drive_root_relative_path",
        "department-reports/example-unit",
        "site:example-hub",
        "drive:shared-documents",
        "owner_account_id",
        "summary",
        "Offline accept boundary verification fixture.",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "receipt leaked forbidden value: {forbidden}"
        );
    }
}

#[test]
fn crate_has_no_local_cargo_path_dependency() {
    let cargo_toml =
        std::fs::read_to_string(repository_root().join("Cargo.toml")).expect("Cargo.toml");
    assert!(!cargo_toml.contains("path ="));
}
