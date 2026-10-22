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
