use super::{Change, Read, rules};
use crate::{Error, Result, Scope, digest, exchange::Journal};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
pub trait Backend {
    fn inspect(&mut self, scope: &Scope, read: &Read, change: Option<&Change>) -> Result<Value>;
    fn mutate(&mut self, scope: &Scope, change: &Change) -> Result<()>;
    fn wait(&mut self) {}
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub scope: Scope,
    pub change: Change,
    pub before: Value,
    pub effects: Vec<String>,
    pub created_at: u64,
    pub expires_at: u64,
}
fn effects(c: &Change) -> Vec<String> {
    match c {
    Change::SetAutoMapping{..}=>vec!["Add FullAccess when absent; otherwise remove then re-add that exact grant; SendAs is preserved".into(),"AutoMapping is acknowledged by Exchange but requires Outlook client verification".into()],
    Change::CreateInboxRule{..}|Change::UpdateInboxRule{..}|Change::DeleteInboxRule{..}|Change::SetInboxRuleEnabled{..}=>vec!["Changes only the selected zix-owned Inbox rule; other observed rule fields and relative order must match".into(),"Exchange Inbox-rule mutation can remove Outlook client-only rules; explicit blob-removal acknowledgement is required".into(),"Loop check covers mailbox forwarding and Inbox rules; transport rules and external systems are not inferred".into()],
    Change::Configure{values,..} if values.get("HiddenFromAddressListsEnabled") == Some(&json!(true)) =>vec!["Hiding the mailbox from address lists can prevent desktop Outlook delegates from sending as it".into(),"Only listed settings are changed; other observed settings must remain identical".into()],
    _=>vec!["Only listed settings are changed; other observed settings must remain identical".into()],
}
}
pub fn plan(
    b: &mut impl Backend,
    scope: Scope,
    change: Change,
    now: impl Fn() -> u64,
) -> Result<Plan> {
    scope.validate()?;
    change.validate()?;
    let before = b.inspect(&scope, &change.read(), Some(&change))?;
    rules::precondition(&change, &before)?;
    let at = now();
    let p = Plan {
        schema: "zix-microsoft-configuration-plan/v1".into(),
        scope,
        effects: effects(&change),
        change,
        before,
        created_at: at,
        expires_at: at.checked_add(600).ok_or(Error("invalid_clock"))?,
    };
    if serde_json::to_vec(&p)
        .map_err(|_| Error("encoding_failed"))?
        .len()
        > 96 * 1024
    {
        return Err(Error("configuration_plan_observation_limit"));
    }
    Ok(p)
}
pub(super) fn valid(p: &Plan, now: u64) -> Result<()> {
    p.scope.validate()?;
    p.change.validate()?;
    if p.schema != "zix-microsoft-configuration-plan/v1"
        || p.effects != effects(&p.change)
        || p.created_at > now
        || p.expires_at <= now
        || p.expires_at.checked_sub(p.created_at) != Some(600)
    {
        return Err(Error("invalid_or_expired_configuration_plan"));
    }
    rules::precondition(&p.change, &p.before)
}
pub fn execute(
    b: &mut impl Backend,
    j: &Journal,
    p: &Plan,
    approval: &str,
    now: impl Fn() -> u64,
) -> Result<Value> {
    valid(p, now())?;
    let hash = digest(p)?;
    if hash != approval {
        return Err(Error("approval_mismatch"));
    }
    let fresh = b.inspect(&p.scope, &p.change.read(), Some(&p.change))?;
    if rules::canonical(&fresh) != rules::canonical(&p.before) {
        return Err(Error("remote_state_drift"));
    }
    valid(p, now())?;
    let admission = j.admit(&hash, &p.scope)?;
    let mut r = json!({"schema":"zix-microsoft-configuration-receipt/v1","plan_sha256":hash,"state":"outcome_unknown","write_attempted":true,"remote_acknowledged":false,"replay_allowed":false});
    if b.mutate(&p.scope, &p.change).is_err() {
        return Ok(r);
    }
    r["remote_acknowledged"] = json!(true);
    for attempt in 0..3 {
        if attempt > 0 {
            b.wait();
        }
        let Ok(after) = b.inspect(&p.scope, &p.change.read(), Some(&p.change)) else {
            continue;
        };
        if rules::expected(&p.change, &p.before, &after).unwrap_or(false) {
            r["state"] = json!(if matches!(p.change, Change::SetAutoMapping { .. }) {
                "permission_verified_client_check_required"
            } else {
                "verified"
            });
            if matches!(p.change, Change::SetAutoMapping { .. }) {
                r["auto_mapping_observed"] = json!(false);
            }
            admission.completed()?;
            return Ok(r);
        }
    }
    Ok(r)
}
