use super::canonical::canonical;
use super::settings::target_row;
use super::{Change, Rule};
use crate::{Error, Result};
use serde_json::{Value, json};
pub(crate) fn complete(v: &Value) -> Result<()> {
    if v["complete"] != true || v["data"].is_null() {
        Err(Error("configuration_not_completely_observed"))
    } else {
        Ok(())
    }
}
pub(super) fn rule_rows(v: &Value) -> Result<&Vec<Value>> {
    v["data"]["rules"]
        .as_array()
        .ok_or(Error("inbox_rules_not_observed"))
}
pub(crate) fn precondition(c: &Change, b: &Value) -> Result<()> {
    complete(b)?;
    match c {
        Change::Configure {
            identity, values, ..
        } => {
            let row = target_row(b, identity.as_deref())?;
            if values.iter().any(|(k, _)| row.get(k).is_none()) {
                return Err(Error("configuration_property_unobserved"));
            }
            if values.iter().all(|(k, v)| row[k] == *v) {
                return Err(Error("already_in_requested_state"));
            }
        }
        Change::CreateInboxRule {
            rule,
            acknowledge_outlook_rules_blob_removal,
            ..
        } => {
            if !acknowledge_outlook_rules_blob_removal {
                return Err(Error("outlook_rule_blob_acknowledgement_required"));
            }
            if rule_rows(b)?.len() >= 100 {
                return Err(Error("inbox_rule_limit"));
            }
            if usize::from(rule.priority) > rule_rows(b)?.len() {
                return Err(Error("rule_priority_exceeds_observed_count"));
            }
            if rule_rows(b)?.iter().any(|r| r["name"] == rule.name) {
                return Err(Error("rule_name_conflict"));
            }
            routing_guard(b, rule)?;
        }
        Change::UpdateInboxRule {
            identity,
            rule,
            acknowledge_outlook_rules_blob_removal,
            ..
        } => {
            if !acknowledge_outlook_rules_blob_removal {
                return Err(Error("outlook_rule_blob_acknowledgement_required"));
            }
            if managed_rule(b, identity)?["enabled"] != true {
                return Err(Error("enable_rule_before_updating"));
            }
            if rule_rows(b)?
                .iter()
                .any(|r| r["identity"] != *identity && r["name"] == rule.name)
            {
                return Err(Error("rule_name_conflict"));
            }
            routing_guard(b, rule)?;
        }
        Change::DeleteInboxRule {
            identity,
            acknowledge_outlook_rules_blob_removal,
            ..
        } => {
            if !acknowledge_outlook_rules_blob_removal {
                return Err(Error("outlook_rule_blob_acknowledgement_required"));
            }
            managed_rule(b, identity)?;
        }
        Change::SetInboxRuleEnabled {
            identity,
            enabled,
            allow_external,
            acknowledge_outlook_rules_blob_removal,
            ..
        } => {
            if !acknowledge_outlook_rules_blob_removal {
                return Err(Error("outlook_rule_blob_acknowledgement_required"));
            }
            if managed_rule(b, identity)?["enabled"] == *enabled {
                return Err(Error("already_in_requested_state"));
            }
            if *enabled {
                let g = &b["routing_guard"];
                if g["complete"] != true
                    || g["known_cycle"] != false
                    || g["external_destinations"]
                        .as_array()
                        .is_none_or(|a| !a.is_empty() && !*allow_external)
                {
                    return Err(Error("routing_graph_not_safe_or_complete"));
                }
            }
        }
        Change::SetAutoMapping { .. } => {
            let rows = b["data"]["permissions"]
                .as_array()
                .ok_or(Error("permission_unobserved"))?;
            if rows.len() > 1
                || rows.first().is_some_and(|r| {
                    r["deny"] != false
                        || r["inherited"] != false
                        || canonical(&r["rights"]) != json!(["FullAccess"])
                })
                || b["data"]["trustee_smtp"]
                    .as_str()
                    .is_none_or(|s| crate::email(s).is_err())
            {
                return Err(Error("explicit_single_full_access_grant_required"));
            }
        }
    }
    Ok(())
}
fn managed_rule<'a>(b: &'a Value, id: &str) -> Result<&'a Value> {
    let rows: Vec<_> = rule_rows(b)?
        .iter()
        .filter(|r| r["identity"] == id)
        .collect();
    if rows.len() != 1 || rows[0]["manageable"] != true {
        return Err(Error("rule_not_safely_manageable"));
    }
    Ok(rows[0])
}
fn routing_guard(b: &Value, r: &Rule) -> Result<()> {
    let g = &b["routing_guard"];
    if g["known_cycle"] != false || g["complete"] != true {
        return Err(Error("routing_graph_not_safe_or_complete"));
    }
    if g["external_destinations"]
        .as_array()
        .is_none_or(|a| !a.is_empty() && !r.allow_external)
    {
        return Err(Error("external_routing_not_approved"));
    }
    Ok(())
}
