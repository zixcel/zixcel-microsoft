mod support;
use serde_json::{Value, json};
use support::*;
use zixcel_microsoft_admin::{
    Result, Scope, digest,
    management::{Backend, Change, Plan, reconciliation},
};
struct ReadOnly {
    observation: Value,
}
impl Backend for ReadOnly {
    fn observe(&mut self, _: &Scope, _: &Change) -> Result<Value> {
        Ok(self.observation.clone())
    }
    fn mutate(&mut self, _: &Scope, _: &Change) -> Result<()> {
        panic!("resolution must never write remotely")
    }
}
fn original() -> Plan {
    Plan {
        schema: "zix-microsoft-management-plan/v1".into(),
        scope: scope(),
        change: Change::VerifyDomain {
            domain: "example.org".into(),
        },
        before: json!({"target":{"id":"example.org","isVerified":false}}),
        created_at: 100,
        expires_at: 400,
    }
}
fn fake() -> ReadOnly {
    ReadOnly {
        observation: json!({"target":{"id":"example.org","isVerified":false,"state":null}}),
    }
}
fn seed(d: &tempfile::TempDir, p: &Plan) -> (String, Vec<u8>) {
    use std::os::unix::fs::PermissionsExt;
    let hash = digest(p).unwrap();
    let marker = b"admitted; outcome unknown until independent readback\n".to_vec();
    for (name, bytes) in [
        (format!("{hash}.attempt"), marker.clone()),
        (format!("{TENANT}.active"), hash.as_bytes().to_vec()),
    ] {
        let path = d.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    (hash, marker)
}
#[test]
fn explicit_resolution_preserves_uncertainty_original_attempt_and_archived_lock() {
    let (d, j) = journal();
    let old = original();
    let (hash, bytes) = seed(&d, &old);
    let mut f = fake();
    let p = reconciliation::plan(&mut f, &j, &scope(), old, 1000).unwrap();
    assert!(d.path().join(format!("{TENANT}.active")).exists());
    let r = reconciliation::execute(&mut f, &j, &scope(), &p, || 1000).unwrap();
    assert_eq!(r["original_outcome"], "unknown");
    assert_eq!(r["original_replay_allowed"], false);
    assert_eq!(r["remote_write_attempted"], false);
    assert_eq!(
        std::fs::read(d.path().join(format!("{hash}.attempt"))).unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::read_to_string(d.path().join(format!("{hash}.resolved-lock"))).unwrap(),
        hash
    );
    assert!(!d.path().join(format!("{TENANT}.active")).exists());
    assert!(reconciliation::execute(&mut f, &j, &scope(), &p, || 1000).is_err());
    // A closed original plan cannot be admitted again even after lock resolution.
    f.observation = p.original.before.clone();
    assert!(
        zixcel_microsoft_admin::management::execute(&mut f, &j, &p.original, &hash, || 200)
            .is_err()
    );
}
#[test]
fn refuses_progress_missing_state_wrong_scope_too_early_and_unsupported_operations() {
    for mode in 0..7 {
        let (d, j) = journal();
        let mut old = original();
        let mut f = fake();
        let mut s = scope();
        let mut at = 1000;
        match mode {
            0 => f.observation["target"]["state"] = json!({"status":"InProgress"}),
            1 => {
                f.observation["target"]
                    .as_object_mut()
                    .unwrap()
                    .remove("state");
            }
            2 => s.subject = TRUSTEE.into(),
            3 => at = 699,
            4 => {
                old.change = Change::AddDomain {
                    domain: "example.org".into(),
                }
            }
            5 => f.observation["target"]["id"] = json!("different.org"),
            _ => f.observation["target"]["state"] = json!({"status":"Scheduled"}),
        }
        seed(&d, &old);
        assert!(reconciliation::plan(&mut f, &j, &s, old, at).is_err());
        assert!(d.path().join(format!("{TENANT}.active")).exists());
    }
}
#[test]
fn drift_expiry_or_marker_changes_leave_lock_intact() {
    for mode in 0..3 {
        let (d, j) = journal();
        let old = original();
        let (hash, _) = seed(&d, &old);
        let mut f = fake();
        let p = reconciliation::plan(&mut f, &j, &scope(), old, 1000).unwrap();
        if mode == 0 {
            f.observation["target"]["isVerified"] = json!(true);
        }
        if mode == 1 {
            std::fs::write(
                d.path().join(format!("{hash}.attempt")),
                b"admitted; outcome unknown until independent readback\nchanged",
            )
            .unwrap();
        }
        assert!(
            reconciliation::execute(&mut f, &j, &scope(), &p, || if mode == 2 {
                1300
            } else {
                1000
            })
            .is_err()
        );
        assert!(d.path().join(format!("{TENANT}.active")).exists());
    }
}
