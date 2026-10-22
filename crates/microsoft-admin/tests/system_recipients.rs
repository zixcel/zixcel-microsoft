mod support;
use support::*;
use zixcel_microsoft_admin::exchange::{Change, Recipient, plan};
fn system() -> Recipient {
    let primary = "discoverysearchmailbox{55555555-5555-5555-5555-555555555555}@example.org";
    Recipient {
        id: format!("exchange-system:{primary}"),
        primary: primary.into(),
        aliases: vec![primary.into(), "reserved@example.org".into()],
        kind: "DiscoveryMailbox".into(),
        display_name: "Synthetic discovery".into(),
        full_access: false,
        send_as: false,
        sign_in_blocked: None,
    }
}
#[test]
fn system_inventory_keeps_address_conflicts_without_fabricating_entra_ids() {
    let mut s = snapshot();
    s.recipients.push(system());
    assert!(plan(s.clone(), alias(), 100).is_ok());
    assert_eq!(s.recipients.len(), 2);
    let create = Change::CreateShared {
        address: "reserved@example.org".into(),
        display_name: "Reserved".into(),
    };
    assert_eq!(plan(s, create, 100).unwrap_err().0, "address_conflict");
}
#[test]
fn system_reference_cannot_be_used_as_a_write_target_or_trustee() {
    let r = system();
    for c in [
        Change::RemoveAlias {
            mailbox: r.id.clone(),
            address: "reserved@example.org".into(),
        },
        Change::GrantFullAccess {
            mailbox: r.id.clone(),
            trustee: TRUSTEE.into(),
        },
        Change::GrantSendAs {
            mailbox: MAILBOX.into(),
            trustee: r.id.clone(),
        },
    ] {
        assert!(c.validate_syntax().is_err());
    }
}
#[test]
fn missing_or_forged_ids_never_gain_the_system_exception() {
    for mode in 0..5 {
        let mut r = system();
        match mode {
            0 => r.id.clear(),
            1 => r.kind = "UserMailbox".into(),
            2 => r.kind = "SharedMailbox".into(),
            3 => r.kind = "Other".into(),
            _ => r.id = "exchange-system:other@example.org".into(),
        }
        let mut s = snapshot();
        s.recipients.push(r);
        assert!(plan(s, alias(), 100).is_err());
    }
}
#[test]
fn duplicate_system_records_and_malformed_observed_addresses_are_refused() {
    let mut s = snapshot();
    s.recipients.extend([system(), system()]);
    assert!(plan(s, alias(), 100).is_err());
    for address in [
        "a..b@example.org",
        "x@example.org\r\nBcc:y@example.org",
        "a@(example.org)",
    ] {
        let mut r = system();
        r.aliases.push(address.into());
        let mut s = snapshot();
        s.recipients.push(r);
        assert!(plan(s, alias(), 100).is_err());
    }
}
#[test]
fn observed_punctuation_does_not_expand_the_write_address_contract() {
    let mut s = snapshot();
    s.recipients.push(system());
    assert!(plan(s, alias(), 100).is_ok());
    assert!(zixcel_microsoft_admin::email(&system().primary).is_err());
}
