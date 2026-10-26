//! Explicit acceptance using separately installed official binaries; no account connection.
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};
use zixcel_microsoft_admin::exchange::{ExchangeProfile, inspect_runtime};
#[test]
#[ignore = "requires explicit paths to separately installed official runtime distributions"]
fn official_runtime_imports_under_enforced_containment_without_authentication() {
    let d = tempfile::tempdir().unwrap();
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let p = ExchangeProfile {
        pwsh: PathBuf::from(std::env::var("ZIX_TEST_OFFICIAL_PWSH").unwrap()),
        module_manifest: PathBuf::from(std::env::var("ZIX_TEST_OFFICIAL_EXO").unwrap()),
        working_directory: d.path().canonicalize().unwrap(),
        user_principal_name: "offline-probe@example.net".into(),
    };
    let v = inspect_runtime(&p, Duration::from_secs(30)).unwrap();
    assert_eq!(v["module_version"], "3.10.1");
    assert_eq!(v["authentication"], "not_attempted");
    assert_eq!(v["containment"], "enforced");
}
