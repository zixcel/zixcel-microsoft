use super::{Change, Snapshot, planning};
use crate::Result;
use sha2::{Digest, Sha256};
pub(crate) fn verify(before: &Snapshot, after: &Snapshot, change: &Change) -> Result<bool> {
    let mut expected = before.clone();
    if let Change::CreateShared {
        address,
        display_name,
    } = change
    {
        let Some(new) = after.recipients.iter().find(|r| &r.primary == address) else {
            return Ok(false);
        };
        if new.kind != "SharedMailbox"
            || &new.display_name != display_name
            || new.sign_in_blocked != Some(true)
            || new.full_access
            || new.send_as
            || new.aliases.iter().any(|a| {
                let digest = format!("{:x}", Sha256::digest(address.as_bytes()));
                let alias = format!("shared-{}", &digest[..20]);
                a != address
                    && !before
                        .accepted_domains
                        .iter()
                        .any(|d| d.ends_with(".onmicrosoft.com") && *a == format!("{alias}@{d}"))
            })
        {
            return Ok(false);
        }
        expected.recipients.push(new.clone());
    } else {
        let Some(r) = expected
            .recipients
            .iter_mut()
            .find(|r| Some(r.id.as_str()) == change.mailbox())
        else {
            return Ok(false);
        };
        match change {
            Change::AddAlias { address, .. } => r.aliases.push(address.clone()),
            Change::RemoveAlias { address, .. } => r.aliases.retain(|a| a != address),
            Change::GrantFullAccess { .. } => r.full_access = true,
            Change::RevokeFullAccess { .. } => r.full_access = false,
            Change::GrantSendAs { .. } => r.send_as = true,
            Change::RevokeSendAs { .. } => r.send_as = false,
            _ => return Ok(false),
        }
    }
    planning::same_state(&expected, after)
}
