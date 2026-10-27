use super::canonical::canonical;
use super::preconditions::{complete, rule_rows};
use super::{Change, Rule};
use crate::{Error, Result};
use serde_json::{Value, json};
fn rule_matches(v: &Value, r: &Rule) -> bool {
    let wanted = json!({"name":r.name,"priority":r.priority,"from":r.from,"subject_contains":r.subject_contains,"body_contains":r.body_contains,"subject_or_body_contains":r.subject_or_body_contains,"delivery":r.delivery,"recipients":r.recipients,"keep_original":r.keep_original,"stop_processing":r.stop_processing,"enabled":true});
    wanted
        .as_object()
        .is_some_and(|m| m.iter().all(|(k, x)| canonical(&v[k]) == canonical(x)))
        && v["manageable"] == true
}
pub fn expected(c: &Change, b: &Value, a: &Value) -> Result<bool> {
    complete(a)?;
    let mut e = b.clone();
    match c {
        Change::Configure {
            identity, values, ..
        } => {
            if let Some(id) = identity {
                let rows = e["data"]
                    .as_array_mut()
                    .ok_or(Error("configuration_collection_required"))?;
                let row = rows
                    .iter_mut()
                    .find(|r| r["Identity"] == *id || r["Name"] == *id)
                    .ok_or(Error("configuration_identity_missing"))?;
                for (k, v) in values {
                    row[k] = v.clone();
                }
            } else {
                for (k, v) in values {
                    e["data"][k] = v.clone();
                }
            }
        }
        Change::SetAutoMapping { .. } => {
            if b["data"]["permissions"]
                .as_array()
                .is_some_and(Vec::is_empty)
            {
                e["data"]["permissions"] = json!([{"user":b["data"]["trustee_smtp"],"rights":["FullAccess"],"deny":false,"inherited":false}]);
            }
        }
        Change::SetInboxRuleEnabled {
            identity, enabled, ..
        } => {
            let rows = e["data"]["rules"]
                .as_array_mut()
                .ok_or(Error("inbox_rules_not_observed"))?;
            let row = rows
                .iter_mut()
                .find(|r| r["identity"] == *identity)
                .ok_or(Error("rule_not_safely_manageable"))?;
            row["enabled"] = json!(enabled);
        }
        _ => {
            let old_id = match c {
                Change::UpdateInboxRule { identity, .. }
                | Change::DeleteInboxRule { identity, .. } => Some(identity.as_str()),
                _ => None,
            };
            let new = match c {
                Change::CreateInboxRule { rule, .. } | Change::UpdateInboxRule { rule, .. } => {
                    Some(rule)
                }
                _ => None,
            };
            let mut after = rule_rows(a)?.clone();
            let mut before = rule_rows(b)?.clone();
            if let Some(id) = old_id {
                before.retain(|r| r["identity"] != id);
            }
            if let Some(spec) = new {
                let matches: Vec<_> = after.iter().filter(|r| r["name"] == spec.name).collect();
                if matches.len() != 1 || !rule_matches(matches[0], spec) {
                    return Ok(false);
                }
                if old_id.is_some_and(|id| matches[0]["identity"] != id) {
                    return Ok(false);
                }
                let id = matches[0]["identity"].clone();
                after.retain(|r| r["identity"] != id);
            }
            if old_id.is_some_and(|id| after.iter().any(|r| r["identity"] == id)) {
                return Ok(false);
            }
            // Rule insertion/deletion renumbers priorities. Other rule content and
            // relative order must be identical; no unrelated rule may disappear.
            for rows in [&mut before, &mut after] {
                rows.sort_by_key(|r| r["priority"].as_u64().unwrap_or(u64::MAX));
                for r in rows {
                    r.as_object_mut()
                        .ok_or(Error("invalid_rule"))?
                        .remove("priority");
                }
            }
            if before != after {
                return Ok(false);
            }
            e["data"]["rules"] = a["data"]["rules"].clone();
        }
    }
    Ok(canonical(&e) == canonical(a))
}
