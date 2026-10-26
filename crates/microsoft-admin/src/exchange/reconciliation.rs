//! Operator-reviewed local resolution after an uncertain permission grant.
//! Never repeats the write or retroactively marks the original attempt verified.
use super::{Backend, Change, Journal, Plan, Snapshot, planning, postcondition, validation};
use crate::{Error, Result, Scope, digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};
const DECISION: &str =
    "accept_observed_grant_and_listed_service_alias_additions_retain_original_outcome_unknown";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionPlan {
    pub schema: String,
    pub original: Plan,
    pub journal_directory: PathBuf,
    pub original_sha256: String,
    pub attempt_sha256: String,
    pub observation: Snapshot,
    pub service_alias_additions: BTreeMap<String, Vec<String>>,
    pub decision: String,
    pub created_at: u64,
    pub expires_at: u64,
}
fn eligible(
    scope: &Scope,
    p: &Plan,
    observed: &Snapshot,
    now: u64,
) -> Result<BTreeMap<String, Vec<String>>> {
    planning::validate(p, p.created_at)?;
    validation::snapshot(observed, now)?;
    if !matches!(
        p.change,
        Change::GrantFullAccess { .. } | Change::GrantSendAs { .. }
    ) || p.scope != *scope
        || observed.scope != *scope
        || observed.trustee.as_deref() != p.change.trustee()
        || now.saturating_sub(p.created_at) < 600
        || p.created_at > now
    {
        return Err(Error("reconciliation_precondition_failed"));
    }
    let mut adjusted = p.before.clone();
    let mut additions = BTreeMap::new();
    for before in &mut adjusted.recipients {
        let after = observed
            .recipients
            .iter()
            .find(|r| r.id == before.id)
            .ok_or(Error("reconciliation_recipient_missing"))?;
        let mut added = Vec::new();
        for alias in &after.aliases {
            if before.aliases.contains(alias) {
                continue;
            }
            let (local, domain) = alias
                .split_once('@')
                .ok_or(Error("invalid_service_alias"))?;
            if before.kind != "SharedMailbox"
                || !domain.ends_with(".onmicrosoft.com")
                || !p.before.accepted_domains.iter().any(|d| d == domain)
                || !before
                    .aliases
                    .iter()
                    .chain(std::iter::once(&before.primary))
                    .any(|a| a.split_once('@').is_some_and(|(l, _)| l == local))
            {
                return Err(Error("unreviewable_alias_drift"));
            }
            added.push(alias.clone());
        }
        if !added.is_empty() {
            added.sort();
            before.aliases.extend(added.clone());
            additions.insert(before.id.clone(), added);
        }
    }
    // All recipient identities, primary addresses, original aliases, unrelated
    // fields and domains must still match the exact original grant postcondition.
    if !postcondition::verify(&adjusted, observed, &p.change)? {
        return Err(Error("grant_postcondition_not_observed"));
    }
    Ok(additions)
}
pub fn plan(
    backend: &mut impl Backend,
    journal: &Journal,
    scope: &Scope,
    original: Plan,
    now: impl Fn() -> u64,
) -> Result<ResolutionPlan> {
    let observation = backend.snapshot(scope, Some(&original.change))?;
    let now = now();
    let service_alias_additions = eligible(scope, &original, &observation, now)?;
    let original_sha256 = digest(&original)?;
    let attempt_sha256 = journal.uncertain_evidence(&original_sha256, scope)?;
    Ok(ResolutionPlan {
        schema: "zix-microsoft-exchange-resolution-plan/v1".into(),
        original,
        journal_directory: journal.directory().to_path_buf(),
        original_sha256,
        attempt_sha256,
        observation,
        service_alias_additions,
        decision: DECISION.into(),
        created_at: now,
        expires_at: now.checked_add(300).ok_or(Error("invalid_clock"))?,
    })
}
pub fn execute(
    backend: &mut impl Backend,
    journal: &Journal,
    scope: &Scope,
    p: &ResolutionPlan,
    now: impl Fn() -> u64,
) -> Result<Value> {
    let valid = |at| {
        if p.schema != "zix-microsoft-exchange-resolution-plan/v1"
            || p.journal_directory != journal.directory()
            || p.original_sha256 != digest(&p.original)?
            || p.decision != DECISION
            || p.created_at > at
            || p.expires_at <= at
            || p.expires_at.checked_sub(p.created_at) != Some(300)
            || eligible(scope, &p.original, &p.observation, at)? != p.service_alias_additions
        {
            return Err(Error("invalid_or_expired_resolution_plan"));
        }
        Ok(())
    };
    valid(now())?;
    let fresh = backend.snapshot(scope, Some(&p.original.change))?;
    if eligible(scope, &p.original, &fresh, now())? != p.service_alias_additions
        || !planning::same_state(&p.observation, &fresh)?
    {
        return Err(Error("remote_state_drift"));
    }
    valid(now())?;
    let receipt = json!({"schema":"zix-microsoft-exchange-resolution/v1",
        "original_plan_sha256":p.original_sha256,"original_outcome":"unknown",
        "original_replay_allowed":false,"remote_write_attempted":false,
        "desired_explicit_grant_observed":true,"effective_access_inferred":false,
        "observation":fresh,"service_alias_additions":p.service_alias_additions,"decision":p.decision});
    journal.resolve_uncertain(&p.original_sha256, &p.attempt_sha256, scope, &receipt)?;
    Ok(receipt)
}
