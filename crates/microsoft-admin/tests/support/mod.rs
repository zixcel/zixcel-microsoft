#![allow(dead_code)]
use zixcel_microsoft_admin::{
    Result, Scope,
    exchange::{Backend, Change, Plan, Recipient, Snapshot},
};
pub const TENANT: &str = "11111111-1111-1111-1111-111111111111";
pub const SUBJECT: &str = "22222222-2222-2222-2222-222222222222";
pub const MAILBOX: &str = "33333333-3333-3333-3333-333333333333";
pub const TRUSTEE: &str = "44444444-4444-4444-4444-444444444444";
pub fn scope() -> Scope {
    Scope {
        tenant: TENANT.into(),
        subject: SUBJECT.into(),
    }
}
pub fn snapshot() -> Snapshot {
    Snapshot {
        scope: scope(),
        observed_at: 100,
        accepted_domains: vec!["example.org".into()],
        complete: true,
        trustee: None,
        recipients: vec![Recipient {
            id: MAILBOX.into(),
            display_name: "Support".into(),
            kind: "SharedMailbox".into(),
            primary: "support@example.org".into(),
            aliases: vec!["support@example.org".into(), "help@example.org".into()],
            full_access: false,
            send_as: false,
            sign_in_blocked: Some(true),
        }],
    }
}
pub fn alias() -> Change {
    Change::AddAlias {
        mailbox: MAILBOX.into(),
        address: "contact@example.org".into(),
    }
}
pub struct Fake {
    pub before: Snapshot,
    pub after: Snapshot,
    pub calls: usize,
    pub writes: usize,
    pub fail_write: bool,
    pub fail_post: bool,
}
impl Fake {
    pub fn alias() -> Self {
        let before = snapshot();
        let mut after = before.clone();
        after.recipients[0]
            .aliases
            .push("contact@example.org".into());
        Self {
            before,
            after,
            calls: 0,
            writes: 0,
            fail_write: false,
            fail_post: false,
        }
    }
}
impl Backend for Fake {
    fn snapshot(&mut self, _: &Scope, _: Option<&Change>) -> Result<Snapshot> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(self.before.clone())
        } else if self.fail_post {
            Err(zixcel_microsoft_admin::Error("synthetic_read_failure"))
        } else {
            Ok(self.after.clone())
        }
    }
    fn apply(&mut self, _: &Plan) -> Result<()> {
        self.writes += 1;
        if self.fail_write {
            Err(zixcel_microsoft_admin::Error("synthetic_write_unknown"))
        } else {
            Ok(())
        }
    }
}
pub fn journal() -> (tempfile::TempDir, zixcel_microsoft_admin::exchange::Journal) {
    let d = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let j = zixcel_microsoft_admin::exchange::Journal::open(d.path()).unwrap();
    (d, j)
}
