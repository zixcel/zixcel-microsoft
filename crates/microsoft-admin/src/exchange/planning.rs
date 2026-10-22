use super::{Change, Plan, Snapshot, validation};
use crate::{Error, Result};
pub fn plan(before: Snapshot, change: Change, now: u64) -> Result<Plan> {
    change.validate_syntax()?;
    validation::snapshot(&before, now)?;
    super::change_validation::change(&before, &change)?;
    Ok(Plan {
        schema: "zix-microsoft-exchange-plan/v1".into(),
        scope: before.scope.clone(),
        change,
        before,
        created_at: now,
        expires_at: now.checked_add(120).ok_or(Error("invalid_clock"))?,
    })
}
pub(crate) fn validate(p: &Plan, now: u64) -> Result<()> {
    if p.schema != "zix-microsoft-exchange-plan/v1"
        || p.scope != p.before.scope
        || p.created_at > now
        || p.expires_at <= now
        || p.expires_at.checked_sub(p.created_at) != Some(120)
    {
        return Err(Error("invalid_or_expired_plan"));
    }
    validation::snapshot(&p.before, now)?;
    super::change_validation::change(&p.before, &p.change)
}
pub(crate) fn same_state(a: &Snapshot, b: &Snapshot) -> Result<bool> {
    fn stable(s: &Snapshot) -> Snapshot {
        let mut s = s.clone();
        s.observed_at = 0;
        s.accepted_domains.sort();
        s.recipients.sort_by(|a, b| a.id.cmp(&b.id));
        for r in &mut s.recipients {
            r.aliases.sort();
        }
        s
    }
    Ok(crate::digest(&stable(a))? == crate::digest(&stable(b))?)
}
