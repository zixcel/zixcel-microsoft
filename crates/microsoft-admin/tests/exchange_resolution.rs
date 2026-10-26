mod support;
use support::*;
use zixcel_microsoft_admin::{
    Result, Scope, digest,
    exchange::{self, Backend, Change, Plan, Snapshot, reconciliation},
};
struct ReadOnly(Snapshot);
impl Backend for ReadOnly {
    fn snapshot(&mut self, _: &Scope, _: Option<&Change>) -> Result<Snapshot> {
        Ok(self.0.clone())
    }
    fn apply(&mut self, _: &Plan) -> Result<()> {
        panic!("must never repeat a grant")
    }
}
fn original() -> Plan {
    let mut s = snapshot();
    s.trustee = Some(TRUSTEE.into());
    s.accepted_domains.push("example.onmicrosoft.com".into());
    exchange::plan(
        s,
        Change::GrantFullAccess {
            mailbox: MAILBOX.into(),
            trustee: TRUSTEE.into(),
        },
        100,
    )
    .unwrap()
}
fn observed(p: &Plan) -> ReadOnly {
    let mut s = p.before.clone();
    s.observed_at = 1000;
    s.recipients[0].full_access = true;
    s.recipients[0]
        .aliases
        .push("support@example.onmicrosoft.com".into());
    ReadOnly(s)
}
fn seed(d: &tempfile::TempDir, p: &Plan) -> String {
    use std::os::unix::fs::PermissionsExt;
    let hash = digest(p).unwrap();
    for (name, content) in [
        (
            format!("{hash}.attempt"),
            "admitted; outcome unknown until independent readback\n",
        ),
        (format!("{TENANT}.active"), hash.as_str()),
    ] {
        let f = d.path().join(name);
        std::fs::write(&f, content).unwrap();
        std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    hash
}
#[test]
fn reviewed_additions_archive_lock_without_rewriting_history_or_replaying_write() {
    for send_as in [false, true] {
        let (d, j) = journal();
        let mut old = original();
        if send_as {
            old.change = Change::GrantSendAs {
                mailbox: MAILBOX.into(),
                trustee: TRUSTEE.into(),
            };
        }
        let hash = seed(&d, &old);
        let marker = std::fs::read(d.path().join(format!("{hash}.attempt"))).unwrap();
        let mut f = observed(&old);
        if send_as {
            f.0.recipients[0].full_access = false;
            f.0.recipients[0].send_as = true;
        }
        let p = reconciliation::plan(&mut f, &j, &scope(), old, || 1000).unwrap();
        assert_eq!(
            p.service_alias_additions[MAILBOX],
            ["support@example.onmicrosoft.com"]
        );
        f.0.observed_at = 1001;
        f.0.recipients[0].aliases.reverse();
        let r = reconciliation::execute(&mut f, &j, &scope(), &p, || 1001).unwrap();
        assert_eq!(r["original_outcome"], "unknown");
        assert_eq!(r["remote_write_attempted"], false);
        assert_eq!(r["original_replay_allowed"], false);
        assert_eq!(r["desired_explicit_grant_observed"], true);
        assert_eq!(
            std::fs::read(d.path().join(format!("{hash}.attempt"))).unwrap(),
            marker
        );
        assert!(!d.path().join(format!("{TENANT}.active")).exists());
        assert_eq!(
            std::fs::read_to_string(d.path().join(format!("{hash}.resolved-lock"))).unwrap(),
            hash
        );
        assert!(reconciliation::execute(&mut f, &j, &scope(), &p, || 1001).is_err());
        f.0 = p.original.before.clone();
        assert!(exchange::execute(&mut f, &j, &p.original, &hash, || 100).is_err());
    }
}
#[test]
fn refuses_missing_grant_identity_domain_removal_primary_and_unrelated_permission_drift() {
    for mode in 0..12 {
        let (d, j) = journal();
        let old = original();
        seed(&d, &old);
        let mut f = observed(&old);
        match mode {
            0 => f.0.recipients[0].full_access = false,
            1 => f.0.recipients[0].primary = "other@example.org".into(),
            2 => {
                f.0.recipients[0].aliases.remove(1);
            }
            3 => f.0.recipients[0].send_as = true,
            4 => f.0.trustee = Some(SUBJECT.into()),
            5 => f.0.complete = false,
            6 => f.0.recipients[0].sign_in_blocked = Some(false),
            7 => f.0.recipients[0]
                .aliases
                .push("stranger@example.onmicrosoft.com".into()),
            8 => f.0.recipients[0]
                .aliases
                .push("support@evil.onmicrosoft.com".into()),
            9 => f.0.recipients[0]
                .aliases
                .push("support@example.org.evil.com".into()),
            10 => f.0.scope.subject = TRUSTEE.into(),
            _ => f.0.accepted_domains.pop().map(|_| ()).unwrap(),
        }
        assert!(
            reconciliation::plan(&mut f, &j, &scope(), old, || 1000).is_err(),
            "mode {mode}"
        );
        assert!(d.path().join(format!("{TENANT}.active")).exists());
    }
}
#[test]
fn refuses_early_expired_modified_journal_plan_and_postreview_drift() {
    for mode in 0..7 {
        let (d, j) = journal();
        let old = original();
        let hash = seed(&d, &old);
        let mut f = observed(&old);
        if mode == 0 {
            f.0.observed_at = 699;
            assert!(reconciliation::plan(&mut f, &j, &scope(), old, || 699).is_err());
            continue;
        }
        let mut p = reconciliation::plan(&mut f, &j, &scope(), old, || 1000).unwrap();
        match mode {
            1 => f.0.recipients[0].display_name = "Changed".into(),
            2 => {
                p.service_alias_additions.clear();
            }
            3 => std::fs::write(d.path().join(format!("{hash}.attempt")), b"changed").unwrap(),
            4 => p.original.scope.subject = TRUSTEE.into(),
            5 => p.decision = "unlock".into(),
            _ => (),
        }
        assert!(
            reconciliation::execute(&mut f, &j, &scope(), &p, || if mode == 6 {
                1300
            } else {
                1000
            })
            .is_err()
        );
        assert!(d.path().join(format!("{TENANT}.active")).exists());
    }
}

#[test]
fn planning_uses_clock_after_read_and_execution_rechecks_after_delayed_read() {
    use std::{cell::Cell, rc::Rc};
    struct DelayedRead {
        observed: Snapshot,
        clock: Rc<Cell<u64>>,
    }
    impl Backend for DelayedRead {
        fn snapshot(&mut self, _: &Scope, _: Option<&Change>) -> Result<Snapshot> {
            self.clock.set(self.clock.get() + 10);
            let mut s = self.observed.clone();
            s.observed_at = self.clock.get();
            Ok(s)
        }
        fn apply(&mut self, _: &Plan) -> Result<()> {
            panic!("no remote mutation")
        }
    }
    let (d, j) = journal();
    let old = original();
    seed(&d, &old);
    let clock = Rc::new(Cell::new(1000));
    let mut f = DelayedRead {
        observed: observed(&old).0,
        clock: clock.clone(),
    };
    let p = reconciliation::plan(&mut f, &j, &scope(), old, || clock.get()).unwrap();
    assert_eq!(p.created_at, 1010);
    assert_eq!(p.observation.observed_at, 1010);
    assert_eq!(p.expires_at, 1310);
    let receipt = reconciliation::execute(&mut f, &j, &scope(), &p, || clock.get()).unwrap();
    assert_eq!(receipt["observation"]["observed_at"], 1020);
    assert_eq!(receipt["remote_write_attempted"], false);
    assert!(!d.path().join(format!("{TENANT}.active")).exists());
}
