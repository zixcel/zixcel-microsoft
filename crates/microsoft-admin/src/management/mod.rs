mod failure;
mod graph;
mod model;
mod rules;
use crate::{Error, Result, Scope, exchange::Journal};
pub use failure::{FailurePhase, FailureReason, MutationFailure, ProviderCode};
pub use graph::Graph;
pub use model::{Change, Plan};
use serde_json::{Value, json};
pub trait Backend {
    fn observe(&mut self, scope: &Scope, change: &Change) -> Result<Value>;
    fn prepare(&self, _change: &Change) -> Result<()> {
        Ok(())
    }
    /// One mutation only. Never retry a write, including after a transport failure.
    fn mutate(&mut self, scope: &Scope, change: &Change) -> Result<()>;
    fn mutation_failure(&self) -> Option<MutationFailure> {
        None
    }
}
pub fn plan(
    backend: &mut (impl Backend + ?Sized),
    scope: Scope,
    change: Change,
    now: u64,
) -> Result<Plan> {
    scope.validate()?;
    change.validate()?;
    let before = backend.observe(&scope, &change)?;
    rules::precondition(&scope, &change, &before)?;
    Ok(Plan {
        schema: "zix-microsoft-management-plan/v1".into(),
        scope,
        change,
        before,
        created_at: now,
        expires_at: now.checked_add(300).ok_or(Error("invalid_clock"))?,
    })
}
fn valid(p: &Plan, now: u64) -> Result<()> {
    p.scope.validate()?;
    p.change.validate()?;
    if p.schema != "zix-microsoft-management-plan/v1"
        || p.created_at > now
        || p.expires_at <= now
        || p.expires_at.checked_sub(p.created_at) != Some(300)
    {
        return Err(Error("invalid_or_expired_plan"));
    }
    rules::precondition(&p.scope, &p.change, &p.before)
}
pub fn execute(
    backend: &mut (impl Backend + ?Sized),
    journal: &Journal,
    p: &Plan,
    approval: &str,
    now: impl Fn() -> u64,
) -> Result<Value> {
    valid(p, now())?;
    let hash = crate::digest(p)?;
    if hash != approval {
        return Err(Error("approval_mismatch"));
    }
    let fresh = backend.observe(&p.scope, &p.change)?;
    if fresh != p.before {
        return Err(Error("remote_state_drift"));
    }
    valid(p, now())?;
    backend.prepare(&p.change)?;
    let admission = journal.admit(&hash, &p.scope)?;
    let mut receipt = json!({"schema":"zix-microsoft-management-receipt/v1","plan_sha256":hash,"write_attempted":true,"remote_acknowledged":false,"state":"outcome_unknown","replay_allowed":false});
    if let Err(error) = backend.mutate(&p.scope, &p.change) {
        let failure = backend
            .mutation_failure()
            .unwrap_or_else(|| MutationFailure::from_error(FailurePhase::Unknown, error));
        admission.record_failure(&failure)?;
        receipt["failure"] = serde_json::to_value(failure).map_err(|_| Error("encoding_failed"))?;
        return Ok(receipt);
    }
    receipt["remote_acknowledged"] = json!(true);
    let after = backend.observe(&p.scope, &p.change);
    if after
        .as_ref()
        .is_ok_and(|after| rules::postcondition(&p.change, &p.before, after).unwrap_or(false))
    {
        receipt["state"] = json!(if matches!(p.change, Change::SendProbe { .. }) {
            "sent_copy_observed_delivery_unverified"
        } else {
            "verified"
        });
        admission.completed()?;
    } else {
        let failure = MutationFailure::from_error(
            FailurePhase::PostRead,
            after.err().unwrap_or(Error("postcondition_not_observed")),
        );
        admission.record_failure(&failure)?;
        receipt["failure"] = serde_json::to_value(failure).map_err(|_| Error("encoding_failed"))?;
    }
    Ok(receipt)
}
