mod support;
use support::*;
use zixcel_microsoft_admin::{
    digest,
    exchange::{Change, execute, plan},
};
#[test]
fn each_secondary_alias_and_explicit_permission_delta_is_verified_separately() {
    for mode in 0..6 {
        let (_d, j) = journal();
        let mut before = snapshot();
        let mut after = before.clone();
        let change = match mode {
            0 => {
                after.recipients[0].aliases.push("new@example.org".into());
                Change::AddAlias {
                    mailbox: MAILBOX.into(),
                    address: "new@example.org".into(),
                }
            }
            1 => {
                after.recipients[0]
                    .aliases
                    .retain(|a| a != "help@example.org");
                Change::RemoveAlias {
                    mailbox: MAILBOX.into(),
                    address: "help@example.org".into(),
                }
            }
            2 => {
                before.trustee = Some(TRUSTEE.into());
                after.trustee = before.trustee.clone();
                after.recipients[0].full_access = true;
                Change::GrantFullAccess {
                    mailbox: MAILBOX.into(),
                    trustee: TRUSTEE.into(),
                }
            }
            3 => {
                before.trustee = Some(TRUSTEE.into());
                after.trustee = before.trustee.clone();
                before.recipients[0].full_access = true;
                Change::RevokeFullAccess {
                    mailbox: MAILBOX.into(),
                    trustee: TRUSTEE.into(),
                }
            }
            4 => {
                before.trustee = Some(TRUSTEE.into());
                after.trustee = before.trustee.clone();
                after.recipients[0].send_as = true;
                Change::GrantSendAs {
                    mailbox: MAILBOX.into(),
                    trustee: TRUSTEE.into(),
                }
            }
            _ => {
                before.trustee = Some(TRUSTEE.into());
                after.trustee = before.trustee.clone();
                before.recipients[0].send_as = true;
                Change::RevokeSendAs {
                    mailbox: MAILBOX.into(),
                    trustee: TRUSTEE.into(),
                }
            }
        };
        let p = plan(before.clone(), change, 100).unwrap();
        let mut f = Fake {
            before,
            after,
            calls: 0,
            writes: 0,
            fail_write: false,
            fail_post: false,
        };
        assert_eq!(
            execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100)
                .unwrap()
                .state,
            "verified"
        );
        assert_eq!(f.writes, 1);
    }
}

#[test]
fn primary_smtp_preserves_mailbox_identity_aliases_and_requires_exact_postcondition() {
    for address in ["help@example.org", "new@example.org"] {
        for drop_old in [false, true] {
            let (_d, j) = journal();
            let before = snapshot();
            let mut after = before.clone();
            after.recipients[0].primary = address.into();
            if !after.recipients[0].aliases.contains(&address.to_string()) {
                after.recipients[0].aliases.push(address.into());
            }
            if drop_old {
                after.recipients[0]
                    .aliases
                    .retain(|a| a != "support@example.org");
            }
            let p = plan(
                before.clone(),
                Change::SetPrimarySmtp {
                    mailbox: MAILBOX.into(),
                    address: address.into(),
                },
                100,
            )
            .unwrap();
            let mut f = Fake {
                before,
                after,
                calls: 0,
                writes: 0,
                fail_write: false,
                fail_post: false,
            };
            assert_eq!(
                execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100)
                    .unwrap()
                    .state,
                if drop_old {
                    "outcome_unknown"
                } else {
                    "verified"
                }
            );
        }
    }
}
#[test]
fn primary_smtp_rejects_other_recipient_conflicts_and_unverified_domains() {
    let mut s = snapshot();
    let mut other = s.recipients[0].clone();
    other.id = TRUSTEE.into();
    other.primary = "else@example.org".into();
    other.aliases = vec!["else@example.org".into()];
    s.recipients.push(other);
    for address in [
        "else@example.org",
        "support@example.org",
        "new@unverified.org",
    ] {
        assert!(
            plan(
                s.clone(),
                Change::SetPrimarySmtp {
                    mailbox: MAILBOX.into(),
                    address: address.into()
                },
                100
            )
            .is_err()
        );
    }
}
