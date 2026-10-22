mod support;
use sha2::{Digest, Sha256};
use support::*;
use zixcel_microsoft_admin::{
    digest,
    exchange::{Change, execute, plan},
};
#[test]
fn only_the_exact_generated_alias_in_an_accepted_onmicrosoft_domain_is_admitted() {
    let address = "security@example.org";
    let hash = format!("{:x}", Sha256::digest(address.as_bytes()));
    let generated = format!("shared-{}@tenant.onmicrosoft.com", &hash[..20]);
    for mode in 0..4 {
        let (_d, j) = journal();
        let mut before = snapshot();
        before
            .accepted_domains
            .push("tenant.onmicrosoft.com".into());
        let mut after = before.clone();
        let mut new = before.recipients[0].clone();
        new.id = TRUSTEE.into();
        new.primary = address.into();
        new.display_name = "Security".into();
        new.aliases = vec![
            address.into(),
            match mode {
                0 => generated.clone(),
                1 => "other@tenant.onmicrosoft.com".into(),
                2 => format!("shared-{}@outside.onmicrosoft.com", &hash[..20]),
                _ => "unapproved@example.org".into(),
            },
        ];
        after.recipients.push(new);
        let c = Change::CreateShared {
            address: address.into(),
            display_name: "Security".into(),
        };
        let p = plan(before.clone(), c, 100).unwrap();
        let mut f = Fake {
            before,
            after,
            calls: 0,
            writes: 0,
            fail_write: false,
            fail_post: false,
        };
        let r = execute(&mut f, &j, &p, &digest(&p).unwrap(), || 100).unwrap();
        assert_eq!(
            r.state,
            if mode == 0 {
                "verified"
            } else {
                "outcome_unknown"
            }
        );
        assert!(!r.replay_allowed);
    }
}
