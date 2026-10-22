mod support;
use support::*;
use zixcel_microsoft_admin::{
    domain, email,
    exchange::{Change, plan},
    uuid,
};
#[test]
fn closed_contract_rejects_arbitrary_commands_and_extra_fields() {
    assert!(
        serde_json::from_str::<Change>(r#"{"operation":"run","command":"arbitrary"}"#).is_err()
    );
    assert!(
        serde_json::from_str::<Change>(
            r#"{"operation":"add_alias","mailbox":"x","address":"x","script":"x"}"#
        )
        .is_err()
    );
}
#[test]
fn untrusted_syntax_and_unicode_hosts_are_refused() {
    for x in [
        "x@other.org\r\nBcc: y",
        "$(whoami)@example.org",
        "x@localhost",
        "x@例.org",
        "a..b@example.org",
    ] {
        assert!(email(x).is_err());
    }
    for x in [
        "../example.org",
        "EXAMPLE.org",
        "https://example.org",
        "example.org.",
    ] {
        assert!(domain(x).is_err());
    }
    for x in ["../../token", "00000000-0000-0000-0000-000000000000", ""] {
        assert!(uuid(x).is_err());
    }
}
#[test]
fn fresh_alias_plan_is_exact_and_contains_no_credentials() {
    let p = plan(snapshot(), alias(), 100).unwrap();
    assert_eq!(p.expires_at, 220);
    let text = serde_json::to_string(&p).unwrap();
    assert!(!text.contains("token"));
    assert_eq!(p.scope, scope());
}
#[test]
fn stale_future_incomplete_or_ambiguous_snapshot_never_plans() {
    for mode in 0..4 {
        let mut s = snapshot();
        match mode {
            0 => s.observed_at = 101,
            1 => s.observed_at = 0,
            2 => s.complete = false,
            _ => s.recipients.push(s.recipients[0].clone()),
        };
        assert!(plan(s, alias(), if mode == 1 { 401 } else { 100 }).is_err());
    }
}
#[test]
fn aliases_require_authoritative_domain_unused_address_and_existing_target() {
    for c in [
        Change::AddAlias {
            mailbox: MAILBOX.into(),
            address: "x@outside.example".into(),
        },
        Change::AddAlias {
            mailbox: MAILBOX.into(),
            address: "help@example.org".into(),
        },
        Change::AddAlias {
            mailbox: TRUSTEE.into(),
            address: "new@example.org".into(),
        },
    ] {
        assert!(plan(snapshot(), c, 100).is_err());
    }
}
#[test]
fn primary_address_removal_and_missing_alias_are_rejected() {
    for a in ["support@example.org", "missing@example.org"] {
        assert!(
            plan(
                snapshot(),
                Change::RemoveAlias {
                    mailbox: MAILBOX.into(),
                    address: a.into()
                },
                100
            )
            .is_err()
        );
    }
    assert!(
        plan(
            snapshot(),
            Change::RemoveAlias {
                mailbox: MAILBOX.into(),
                address: "help@example.org".into()
            },
            100
        )
        .is_ok()
    );
}
#[test]
fn delegation_separates_full_access_send_as_and_exact_trustee() {
    let c = Change::GrantFullAccess {
        mailbox: MAILBOX.into(),
        trustee: TRUSTEE.into(),
    };
    assert!(plan(snapshot(), c.clone(), 100).is_err());
    let mut s = snapshot();
    s.trustee = Some(TRUSTEE.into());
    assert!(plan(s.clone(), c, 100).is_ok());
    assert!(
        plan(
            s,
            Change::RevokeSendAs {
                mailbox: MAILBOX.into(),
                trustee: TRUSTEE.into()
            },
            100
        )
        .is_err()
    );
}
#[test]
fn create_shared_does_not_implicitly_grant_permissions_or_change_dns() {
    let p = plan(
        snapshot(),
        Change::CreateShared {
            address: "security@example.org".into(),
            display_name: "Security".into(),
        },
        100,
    )
    .unwrap();
    assert_eq!(p.before.recipients.len(), 1);
    assert_eq!(p.before.recipients[0].primary, "support@example.org");
}
