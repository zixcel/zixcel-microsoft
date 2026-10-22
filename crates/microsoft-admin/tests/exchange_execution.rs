mod support;
use support::*;
use zixcel_microsoft_admin::{
    digest,
    exchange::{Change, Journal, execute, plan},
};
#[test]
fn exact_approval_fresh_state_and_post_read_complete_one_write() {
    let (_d, j) = journal();
    let mut f = Fake::alias();
    let p = plan(snapshot(), alias(), 100).unwrap();
    let r = execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
    assert_eq!(r.state, "verified");
    assert_eq!(f.writes, 1);
    assert_eq!(f.calls, 2);
    assert!(!r.replay_allowed);
}
#[test]
fn approval_expiry_and_scope_drift_stop_before_effect() {
    for mode in 0..4 {
        let (_d, j) = journal();
        let mut f = Fake::alias();
        let p = plan(snapshot(), alias(), 100).unwrap();
        let hash = if mode == 0 {
            "a".repeat(64)
        } else {
            digest(&p).unwrap()
        };
        if mode == 2 {
            f.before.scope.subject = TRUSTEE.into();
        }
        if mode == 3 {
            f.before.recipients[0].primary = "changed@example.org".into();
        }
        assert!(execute(&mut f, &j, &p, &hash, || if mode == 1 { 221 } else { 100 }).is_err());
        assert_eq!(f.writes, 0);
    }
}
#[test]
fn restart_rejects_same_attempt_even_after_success() {
    let (d, j) = journal();
    let p = plan(snapshot(), alias(), 100).unwrap();
    let hash = digest(&p).unwrap();
    execute(&mut Fake::alias(), &j, &p, &hash, || 100).unwrap();
    drop(j);
    let j = Journal::open(d.path()).unwrap();
    let mut f = Fake::alias();
    assert!(execute(&mut f, &j, &p, &hash, || 100).is_err());
    assert_eq!(f.writes, 0);
}
#[test]
fn write_disconnect_and_readback_failure_are_unknown_not_retryable() {
    for write_failure in [true, false] {
        let (d, j) = journal();
        let p = plan(snapshot(), alias(), 100).unwrap();
        let mut f = Fake::alias();
        f.fail_write = write_failure;
        f.fail_post = !write_failure;
        let r = execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
        assert_eq!(r.state, "outcome_unknown");
        assert!(!r.replay_allowed);
        assert!(d.path().join(format!("{TENANT}.active")).exists());
        assert_eq!(f.writes, 1);
    }
}
#[test]
fn unrelated_recipient_change_blocks_success_despite_requested_alias() {
    let (_d, j) = journal();
    let p = plan(snapshot(), alias(), 100).unwrap();
    let mut f = Fake::alias();
    f.after.recipients[0].send_as = true;
    assert_eq!(
        execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100)
            .unwrap()
            .state,
        "outcome_unknown"
    );
}
#[test]
fn same_tenant_unknown_attempt_blocks_different_plan() {
    let (_d, j) = journal();
    let p = plan(snapshot(), alias(), 100).unwrap();
    let mut f = Fake::alias();
    f.fail_write = true;
    execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
    let other = plan(
        snapshot(),
        Change::AddAlias {
            mailbox: MAILBOX.into(),
            address: "different@example.org".into(),
        },
        101,
    )
    .unwrap();
    let mut f = Fake::alias();
    assert!(execute(&mut f, &j, &other, &digest(&other).unwrap(), || 101).is_err());
    assert_eq!(f.writes, 0);
}
#[test]
fn public_or_symlink_journal_is_rejected() {
    let (d, _) = journal();
    use std::os::unix::fs::{PermissionsExt, symlink};
    let link = d.path().join("link");
    symlink(d.path(), &link).unwrap();
    assert!(Journal::open(&link).is_err());
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Journal::open(d.path()).is_err());
}
#[test]
fn shared_creation_requires_independent_blocked_sign_in_evidence() {
    for blocked in [None, Some(false), Some(true)] {
        let (_d, j) = journal();
        let c = Change::CreateShared {
            address: "security@example.org".into(),
            display_name: "Security".into(),
        };
        let p = plan(snapshot(), c, 100).unwrap();
        let mut f = Fake::alias();
        f.after = snapshot();
        let mut r = f.after.recipients[0].clone();
        r.id = TRUSTEE.into();
        r.primary = "security@example.org".into();
        r.aliases = vec![r.primary.clone()];
        r.display_name = "Security".into();
        r.sign_in_blocked = blocked;
        f.after.recipients.push(r);
        let r = execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
        assert_eq!(r.state == "verified", blocked == Some(true));
    }
}
