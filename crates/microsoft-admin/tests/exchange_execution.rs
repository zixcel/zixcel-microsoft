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
        assert!(execute(&mut f, &j, &p, &hash, || if mode == 1 { 701 } else { 100 }).is_err());
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
        assert_eq!(r.remote_acknowledged, !write_failure);
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
#[test]
fn slow_preflight_cannot_execute_after_approval_expired() {
    let (_d, j) = journal();
    let p = plan(snapshot(), alias(), 100).unwrap();
    let mut f = Fake::alias();
    let calls = std::cell::Cell::new(0);
    assert!(
        execute(&mut f, &j, &p, &digest(&p).unwrap(), || {
            let n = calls.get();
            calls.set(n + 1);
            if n < 2 { 100 } else { 701 }
        })
        .is_err()
    );
    assert_eq!(f.writes, 0);
}

#[test]
fn operator_review_window_does_not_substitute_for_a_fresh_preflight() {
    for fresh in [false, true] {
        let (_d, j) = journal();
        let p = plan(snapshot(), alias(), 100).unwrap();
        let mut f = Fake::alias();
        if fresh {
            f.before.observed_at = 500;
            f.after.observed_at = 500;
        }
        let result = execute(&mut f, &j, &p, &digest(&p).unwrap(), || 500);
        assert_eq!(result.is_ok(), fresh);
        assert_eq!(f.writes, usize::from(fresh));
    }
}

#[test]
fn delayed_postconditions_retry_only_reads_with_a_fixed_limit() {
    use zixcel_microsoft_admin::{
        Result, Scope,
        exchange::{Backend, Plan, Snapshot},
    };
    struct Delayed {
        inner: Fake,
        stale_reads: usize,
        waits: usize,
    }
    impl Backend for Delayed {
        fn snapshot(&mut self, scope: &Scope, change: Option<&Change>) -> Result<Snapshot> {
            if self.inner.writes > 0 && self.stale_reads > 0 {
                self.stale_reads -= 1;
                self.inner.calls += 1;
                return Ok(self.inner.before.clone());
            }
            self.inner.snapshot(scope, change)
        }
        fn apply(&mut self, p: &Plan) -> Result<()> {
            self.inner.apply(p)
        }
        fn wait_for_readback(&mut self) {
            self.waits += 1;
        }
    }
    for delay in [1, 2, 3, 10] {
        let (d, j) = journal();
        let p = plan(snapshot(), alias(), 100).unwrap();
        let mut backend = Delayed {
            inner: Fake::alias(),
            stale_reads: delay,
            waits: 0,
        };
        let receipt = execute(&mut backend, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
        assert_eq!(receipt.state == "verified", delay < 3);
        assert_eq!(backend.inner.writes, 1);
        assert_eq!(backend.inner.calls, 1 + (delay + 1).min(3));
        assert_eq!(backend.waits, delay.min(2));
        assert_eq!(
            d.path().join(format!("{TENANT}.active")).exists(),
            delay >= 3
        );
        assert!(!receipt.replay_allowed);
    }
}
