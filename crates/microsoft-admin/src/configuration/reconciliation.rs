//! Local-only resolution; original outcomes and permanent replay bans survive.
use super::{Backend, Change, Plan, engine, rules};
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
    pub observed_state: String,
    pub created_at: u64,
    pub expires_at: u64,
}
fn state(p: &Plan, scope: &Scope, v: &Value, at: u64) -> Result<String> {
    engine::valid(p, p.created_at)?;
    rules::complete(v)?;
    if p.scope != *scope || p.created_at > at || at.saturating_sub(p.created_at) < 600 {
        return Err(Error("configuration_resolution_precondition_failed"));
    }
    if rules::canonical(&p.before) == rules::canonical(v) {
        return Ok(if matches!(p.change, Change::SetAutoMapping { .. }) {
            "permission_present_mapping_unobserved"
        } else {
            "original_state_observed"
        }
        .into());
    }
    if rules::expected(&p.change, &p.before, v).unwrap_or(false) {
        return Ok("requested_state_observed".into());
    }
    if matches!(p.change, Change::SetAutoMapping { .. }) {
        let mut intermediate = p.before.clone();
        intermediate["data"]["permissions"] = json!([]);
        if rules::canonical(&intermediate) == rules::canonical(v) {
            return Ok("full_access_absent_requires_separate_repair_plan".into());
        }
    }
    Err(Error("unrelated_configuration_drift"))
}
pub fn plan(
    b: &mut impl Backend,
    j: &Journal,
    scope: &Scope,
    original: Plan,
    now: impl Fn() -> u64,
) -> Result<ResolutionPlan> {
    let observation = b.inspect(scope, &original.change.read(), Some(&original.change))?;
    let at = now();
    let observed_state = state(&original, scope, &observation, at)?;
    let original_sha256 = digest(&original)?;
    let attempt_sha256 = j.uncertain_evidence(&original_sha256, scope)?;
    Ok(ResolutionPlan {
        schema: "zix-microsoft-configuration-resolution-plan/v1".into(),
        original,
        journal_directory: j.directory().into(),
        original_sha256,
        attempt_sha256,
        observation,
        observed_state,
        created_at: at,
        expires_at: at.checked_add(300).ok_or(Error("invalid_clock"))?,
    })
}
pub fn execute(
    b: &mut impl Backend,
    j: &Journal,
    scope: &Scope,
    p: &ResolutionPlan,
    now: impl Fn() -> u64,
) -> Result<Value> {
    let validate = |at| -> Result<()> {
        if p.schema != "zix-microsoft-configuration-resolution-plan/v1"
            || p.journal_directory != j.directory()
            || p.original_sha256 != digest(&p.original)?
            || p.created_at > at
            || p.expires_at <= at
            || p.expires_at.checked_sub(p.created_at) != Some(300)
            || state(&p.original, scope, &p.observation, at)? != p.observed_state
        {
            return Err(Error("invalid_or_expired_configuration_resolution"));
        }
        Ok(())
    };
    validate(now())?;
    let fresh = b.inspect(scope, &p.original.change.read(), Some(&p.original.change))?;
    if rules::canonical(&fresh) != rules::canonical(&p.observation) {
        return Err(Error("remote_state_drift"));
    }
    validate(now())?;
    let r = json!({"schema":"zix-microsoft-configuration-resolution/v1","original_plan_sha256":p.original_sha256,"original_outcome":"unknown","original_replay_allowed":false,"remote_write_attempted":false,"observed_state":p.observed_state,"auto_mapping_observed":false,"observation":fresh,"new_remote_operation_requires_separate_plan_and_operator_approval":true});
    j.resolve_uncertain(&p.original_sha256, &p.attempt_sha256, scope, &r)?;
    Ok(r)
}
