use super::{Change, Snapshot};
use crate::{Error, Result, email, uuid};
pub fn change(s: &Snapshot, c: &Change) -> Result<()> {
    if let Some(id) = c.mailbox() {
        uuid(id)?;
        if !s
            .recipients
            .iter()
            .any(|r| r.id == id && ["UserMailbox", "SharedMailbox"].contains(&r.kind.as_str()))
        {
            return Err(Error("mailbox_not_found"));
        }
    }
    if let Some(t) = c.trustee() {
        uuid(t)?;
        if s.trustee.as_deref() != Some(t) || c.mailbox() == Some(t) {
            return Err(Error("trustee_mismatch"));
        }
    }
    match c {
        Change::CreateShared {
            address,
            display_name,
        } => {
            address_domain(s, address)?;
            unused(s, address)?;
            if display_name.is_empty()
                || display_name.len() > 120
                || display_name.chars().any(char::is_control)
            {
                return Err(Error("invalid_display_name"));
            }
        }
        Change::AddAlias { address, .. } => {
            address_domain(s, address)?;
            unused(s, address)?;
        }
        Change::SetPrimarySmtp { mailbox, address } => {
            address_domain(s, address)?;
            let r = s
                .recipients
                .iter()
                .find(|r| &r.id == mailbox)
                .ok_or(Error("mailbox_not_found"))?;
            if r.primary == *address {
                return Err(Error("already_in_requested_state"));
            }
            if s.recipients.iter().any(|other| {
                other.id != *mailbox
                    && (other.primary == *address || other.aliases.contains(address))
            }) {
                return Err(Error("address_conflict"));
            }
        }
        Change::RemoveAlias { mailbox, address } => {
            email(address)?;
            let r = s
                .recipients
                .iter()
                .find(|r| &r.id == mailbox)
                .ok_or(Error("mailbox_not_found"))?;
            if r.primary == *address || !r.aliases.contains(address) {
                return Err(Error("primary_or_missing_alias"));
            }
        }
        _ => {
            let r = s
                .recipients
                .iter()
                .find(|r| Some(r.id.as_str()) == c.mailbox())
                .ok_or(Error("mailbox_not_found"))?;
            let current = match c {
                Change::GrantFullAccess { .. } | Change::RevokeFullAccess { .. } => r.full_access,
                _ => r.send_as,
            };
            let desired = matches!(
                c,
                Change::GrantFullAccess { .. } | Change::GrantSendAs { .. }
            );
            if current == desired {
                return Err(Error("already_in_requested_state"));
            }
        }
    }
    Ok(())
}
fn address_domain(s: &Snapshot, address: &str) -> Result<()> {
    email(address)?;
    let d = address.split_once('@').ok_or(Error("invalid_address"))?.1;
    if !s.accepted_domains.iter().any(|a| a == d) {
        return Err(Error("domain_not_authoritative"));
    }
    Ok(())
}
fn unused(s: &Snapshot, a: &str) -> Result<()> {
    if s.recipients
        .iter()
        .any(|r| r.primary == a || r.aliases.iter().any(|s| s == a))
    {
        return Err(Error("address_conflict"));
    }
    Ok(())
}
