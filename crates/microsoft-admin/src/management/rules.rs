use super::Change;
use crate::{Error, Result, Scope};
use serde_json::{Value, json};
pub(super) fn precondition(scope: &Scope, c: &Change, s: &Value) -> Result<()> {
    let allowed = match c {
        Change::AddDomain { .. } | Change::CreateUser { .. } | Change::PrepareDkim { .. } => {
            s["target"].is_null() && s["eligible"] == true
        }
        Change::VerifyDomain { domain } => {
            s["target"]["id"] == *domain && s["target"]["isVerified"] == false
        }
        Change::SetUsageLocation {
            user,
            usage_location,
        } => s["target"]["id"] == *user && s["target"]["usageLocation"] != *usage_location,
        Change::SetUserEnabled { user, enabled } => {
            s["target"]["id"] == *user
                && s["target"]["accountEnabled"].is_boolean()
                && s["target"]["accountEnabled"] != *enabled
        }
        Change::AssignLicense { user, sku } => {
            s["target"]["id"] == *user
                && s["eligible"] == true
                && s["target"]["usageLocation"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
                && s["target"]["assignedLicenses"]
                    .as_array()
                    .is_some_and(|a| !a.iter().any(|l| l["skuId"] == *sku))
        }
        Change::EnableDkim { domain } => {
            s["target"]["domain"] == *domain
                && s["target"]["enabled"] == false
                && ["selector1", "selector2"]
                    .iter()
                    .all(|k| s["target"][k].as_str().is_some_and(|s| !s.is_empty()))
        }
        Change::SendProbe { .. } => s["sender"] == scope.subject && s["target"].is_null(),
    };
    if !allowed {
        return Err(Error("management_precondition_failed"));
    }
    Ok(())
}
pub(super) fn postcondition(c: &Change, b: &Value, a: &Value) -> Result<bool> {
    let mut expected = b.clone();
    match c {
        Change::AddDomain { domain } => {
            return Ok(a["target"]["id"] == *domain && a["target"]["isVerified"].is_boolean());
        }
        Change::VerifyDomain { .. } => {
            let mut before = b["target"].clone();
            let mut after = a["target"].clone();
            if a["target"]["isVerified"] != true {
                return Ok(false);
            }
            for v in [&mut before, &mut after] {
                if let Some(o) = v.as_object_mut() {
                    o.remove("isVerified");
                    o.remove("supportedServices");
                }
            }
            return Ok(before == after);
        }
        Change::CreateUser {
            upn,
            display_name,
            mail_nickname,
            usage_location,
            account_enabled,
        } => {
            return Ok(a["target"]["id"]
                .as_str()
                .is_some_and(|id| crate::uuid(id).is_ok())
                && a["target"]["userPrincipalName"] == *upn
                && a["target"]["displayName"] == *display_name
                && a["target"]["mailNickname"] == *mail_nickname
                && a["target"]["usageLocation"] == *usage_location
                && a["target"]["accountEnabled"] == *account_enabled);
        }
        Change::SetUsageLocation { usage_location, .. } => {
            expected["target"]["usageLocation"] = json!(usage_location)
        }
        Change::SetUserEnabled { enabled, .. } => {
            expected["target"]["accountEnabled"] = json!(enabled)
        }
        Change::AssignLicense { sku, .. } => {
            let mut before = b["target"].clone();
            let mut after = a["target"].clone();
            let added = json!({"skuId":sku,"disabledPlans":[]});
            let old = before["assignedLicenses"]
                .as_array_mut()
                .ok_or(Error("license_observation_incomplete"))?;
            old.push(added);
            old.sort_by_key(|v| v["skuId"].as_str().unwrap_or_default().to_owned());
            if let Some(list) = after["assignedLicenses"].as_array_mut() {
                list.sort_by_key(|v| v["skuId"].as_str().unwrap_or_default().to_owned());
            }
            return Ok(before == after);
        }
        Change::PrepareDkim { domain } => {
            return Ok(a["target"]["domain"] == *domain
                && a["target"]["enabled"] == false
                && ["selector1", "selector2"]
                    .iter()
                    .all(|k| a["target"][k].as_str().is_some_and(|s| !s.is_empty())));
        }
        Change::EnableDkim { .. } => expected["target"]["enabled"] = json!(true),
        Change::SendProbe {
            recipient,
            correlation,
        } => {
            return Ok(a["sender"] == b["sender"]
                && a["target"]["correlation"] == *correlation
                && a["target"]["recipient"] == *recipient
                && a["target"]["id"].as_str().is_some_and(|s| !s.is_empty()));
        }
    }
    Ok(expected == *a)
}
