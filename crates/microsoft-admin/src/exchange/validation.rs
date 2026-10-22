use super::Snapshot;
use crate::{Error, Result, domain, uuid};
use std::collections::BTreeSet;
pub fn snapshot(s: &Snapshot, now: u64) -> Result<()> {
    s.scope.validate()?;
    if !s.complete
        || s.observed_at > now
        || now - s.observed_at > 300
        || s.recipients.len() > 200
        || s.accepted_domains.len() > 100
    {
        return Err(Error("snapshot_incomplete_or_stale"));
    }
    if let Some(t) = &s.trustee {
        uuid(t)?;
    }
    let mut ids = BTreeSet::new();
    let mut addresses = BTreeSet::new();
    for d in &s.accepted_domains {
        domain(d)?;
    }
    for r in &s.recipients {
        if r.kind == "DiscoveryMailbox" {
            if r.id != format!("exchange-system:{}", r.primary) {
                uuid(&r.id)?;
            }
        } else {
            uuid(&r.id)?;
        }
        super::observed_address::validate(&r.primary)?;
        if r.display_name.len() > 120 || r.display_name.chars().any(char::is_control) {
            return Err(Error("invalid_recipient_name"));
        }
        if !ids.insert(&r.id)
            || r.aliases.len() > 400
            || !["UserMailbox", "SharedMailbox", "DiscoveryMailbox", "Other"]
                .contains(&r.kind.as_str())
        {
            return Err(Error("ambiguous_recipient"));
        }
        if !addresses.insert(&r.primary) {
            return Err(Error("duplicate_address"));
        }
        let mut aliases = BTreeSet::new();
        for a in &r.aliases {
            super::observed_address::validate(a)?;
            if !aliases.insert(a) || (a != &r.primary && !addresses.insert(a)) {
                return Err(Error("duplicate_address"));
            }
        }
    }
    Ok(())
}
