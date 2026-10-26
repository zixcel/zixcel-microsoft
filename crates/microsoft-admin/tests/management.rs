use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use zixcel_microsoft_admin::{
    Error, Result, Scope, digest,
    exchange::Journal,
    management::{self, Backend, Change},
};
struct Fake {
    state: Value,
    after: Value,
    writes: usize,
    fail: bool,
}
impl Backend for Fake {
    fn observe(&mut self, _: &Scope, _: &Change) -> Result<Value> {
        Ok(self.state.clone())
    }
    fn mutate(&mut self, _: &Scope, _: &Change) -> Result<()> {
        self.writes += 1;
        if self.fail {
            return Err(Error("synthetic_unknown"));
        }
        self.state = self.after.clone();
        Ok(())
    }
}
fn scope() -> Scope {
    Scope {
        tenant: "11111111-1111-4111-8111-111111111111".into(),
        subject: "22222222-2222-4222-8222-222222222222".into(),
    }
}
fn fake() -> Fake {
    Fake {
        state: json!({"target":null,"eligible":true}),
        after: json!({"target":{"id":"example.net","isVerified":false},"eligible":false}),
        writes: 0,
        fail: false,
    }
}
fn journal() -> (tempfile::TempDir, Journal) {
    let d = tempfile::tempdir().unwrap();
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let j = Journal::open(d.path()).unwrap();
    (d, j)
}
#[test]
fn approval_drift_expiry_and_replay_never_write() {
    let mut b = fake();
    let p = management::plan(
        &mut b,
        scope(),
        Change::AddDomain {
            domain: "example.net".into(),
        },
        100,
    )
    .unwrap();
    let hash = digest(&p).unwrap();
    let (_d, j) = journal();
    assert!(management::execute(&mut b, &j, &p, &"0".repeat(64), || 101).is_err());
    assert!(management::execute(&mut b, &j, &p, &hash, || 400).is_err());
    b.state["eligible"] = json!(false);
    assert!(management::execute(&mut b, &j, &p, &hash, || 101).is_err());
    assert_eq!(b.writes, 0);
    b.state = p.before.clone();
    let r = management::execute(&mut b, &j, &p, &hash, || 101).unwrap();
    assert_eq!(r["state"], "verified");
    assert_eq!(b.writes, 1);
    b.state = p.before.clone();
    assert!(management::execute(&mut b, &j, &p, &hash, || 101).is_err());
    assert_eq!(b.writes, 1);
}
#[test]
fn uncertain_write_keeps_tenant_lock_and_never_retries() {
    let mut b = fake();
    b.fail = true;
    let p = management::plan(
        &mut b,
        scope(),
        Change::AddDomain {
            domain: "example.net".into(),
        },
        100,
    )
    .unwrap();
    let (d, j) = journal();
    let hash = digest(&p).unwrap();
    let r = management::execute(&mut b, &j, &p, &hash, || 101).unwrap();
    assert_eq!(r["state"], "outcome_unknown");
    assert_eq!(r["replay_allowed"], false);
    assert!(d.path().join(format!("{}.active", scope().tenant)).exists());
    assert!(management::execute(&mut b, &j, &p, &hash, || 101).is_err());
    assert_eq!(b.writes, 1);
}
#[test]
fn account_change_requires_exact_preservation_and_probe_does_not_claim_delivery() {
    let mut b = Fake {
        state: json!({"target":{"id":scope().subject,"accountEnabled":false,"displayName":"Owner"}}),
        after: json!({"target":{"id":scope().subject,"accountEnabled":true,"displayName":"Changed unexpectedly"}}),
        writes: 0,
        fail: false,
    };
    let p = management::plan(
        &mut b,
        scope(),
        Change::SetUserEnabled {
            user: scope().subject,
            enabled: true,
        },
        100,
    )
    .unwrap();
    let (_d, j) = journal();
    assert_eq!(
        management::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap()["state"],
        "outcome_unknown"
    );
    let nonce = "33333333-3333-4333-8333-333333333333";
    let mut b = Fake {
        state: json!({"sender":scope().subject,"target":null}),
        after: json!({"sender":scope().subject,"target":{"id":"message-id","recipient":"recipient@example.net","correlation":nonce}}),
        writes: 0,
        fail: false,
    };
    let p = management::plan(
        &mut b,
        scope(),
        Change::SendProbe {
            recipient: "recipient@example.net".into(),
            correlation: nonce.into(),
        },
        100,
    )
    .unwrap();
    let (_d, j) = journal();
    assert_eq!(
        management::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap()["state"],
        "sent_copy_observed_delivery_unverified"
    );
}
