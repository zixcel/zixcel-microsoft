//! Explicit local resolution of an uncertain domain-verification attempt.
//! This does not establish the original HTTP result or perform a remote write.
use super::{Backend, Change, Plan};
use crate::{Error, Result, Scope, digest, exchange::Journal};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionPlan {
    pub schema: String,
    pub original: Plan,
    pub journal_directory: std::path::PathBuf,
    pub original_sha256: String,
    pub attempt_sha256: String,
    pub observation: Value,
    pub decision: String,
    pub created_at: u64,
    pub expires_at: u64,
}
fn eligible(scope: &Scope, p: &Plan, observed: &Value, now: u64) -> Result<()> {
    scope.validate()?;
    p.change.validate()?;
    let Change::VerifyDomain { domain } = &p.change else {
        return Err(Error("unsupported_reconciliation"));
    };
    if p.scope != *scope
        || p.schema != "zix-microsoft-management-plan/v1"
        || p.expires_at.checked_sub(p.created_at) != Some(300)
        || now.saturating_sub(p.created_at) < 600
        || p.created_at > now
        || p.before["target"]["id"] != *domain
        || p.before["target"]["isVerified"] != false
        || observed["target"]["id"] != *domain
        || !observed["target"]["isVerified"].is_boolean()
        || observed["target"].get("state").is_none()
    {
        return Err(Error("reconciliation_precondition_failed"));
    }
    let state = &observed["target"]["state"];
    if !state.is_null() && state["status"] != "Failed" {
        return Err(Error("domain_operation_not_settled"));
    }
    Ok(())
}
pub fn plan(
    backend: &mut (impl Backend + ?Sized),
    journal: &Journal,
    scope: &Scope,
    original: Plan,
    now: u64,
) -> Result<ResolutionPlan> {
    let observation = backend.observe(scope, &original.change)?;
    eligible(scope, &original, &observation, now)?;
    let original_sha256 = digest(&original)?;
    let attempt_sha256 = journal.uncertain_evidence(&original_sha256, scope)?;
    Ok(ResolutionPlan {
        schema: "zix-microsoft-verification-resolution-plan/v1".into(),
        original,
        journal_directory: journal.directory().to_path_buf(),
        original_sha256,
        attempt_sha256,
        observation,
        decision:
            "retain_original_outcome_unknown_and_replay_prohibition_release_local_tenant_lock"
                .into(),
        created_at: now,
        expires_at: now.checked_add(300).ok_or(Error("invalid_clock"))?,
    })
}
pub fn execute(
    backend: &mut (impl Backend + ?Sized),
    journal: &Journal,
    scope: &Scope,
    p: &ResolutionPlan,
    now: impl Fn() -> u64,
) -> Result<Value> {
    let valid = |at| {
        if p.journal_directory != journal.directory()
            || p.schema != "zix-microsoft-verification-resolution-plan/v1"
            || p.original_sha256 != digest(&p.original)?
            || p.created_at > at
            || p.expires_at <= at
            || p.expires_at.checked_sub(p.created_at) != Some(300)
            || p.decision
                != "retain_original_outcome_unknown_and_replay_prohibition_release_local_tenant_lock"
        {
            return Err(Error("invalid_or_expired_resolution_plan"));
        }
        eligible(scope, &p.original, &p.observation, at)
    };
    valid(now())?;
    if backend.observe(scope, &p.original.change)? != p.observation {
        return Err(Error("remote_state_drift"));
    }
    valid(now())?;
    let receipt = json!({"schema":"zix-microsoft-verification-resolution/v1","original_plan_sha256":p.original_sha256,"original_outcome":"unknown","original_replay_allowed":false,"remote_write_attempted":false,"observation":p.observation,"decision":p.decision,"new_remote_operation_requires_separate_plan_and_operator_approval":true});
    journal.resolve_uncertain(&p.original_sha256, &p.attempt_sha256, scope, &receipt)?;
    Ok(receipt)
}
