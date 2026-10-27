use crate::{Error, Result};
use serde_json::{Map, Value};
fn policy_field(v: &Value, depth: usize) -> Result<Value> {
    if depth > 8 {
        return Err(Error("policy_response_depth"));
    }
    Ok(match v {
        Value::String(s) if s.len() > 8192 => return Err(Error("policy_string_limit")),
        Value::Array(a) => {
            if a.len() > 200 {
                return Err(Error("policy_array_limit"));
            }
            Value::Array(
                a.iter()
                    .map(|v| policy_field(v, depth + 1))
                    .collect::<Result<_>>()?,
            )
        }
        Value::Object(m) => {
            if m.len() > 64 {
                return Err(Error("policy_property_limit"));
            }
            let mut out = Map::new();
            for (k, v) in m {
                if k.len() > 128 {
                    return Err(Error("policy_key_limit"));
                }
                out.insert(k.clone(), policy_field(v, depth + 1)?);
            }
            Value::Object(out)
        }
        _ => v.clone(),
    })
}
pub(super) fn policy(v: &Value, fields: &[&str]) -> Result<Value> {
    if !v.is_object() {
        return Err(Error("response_object_required"));
    }
    let mut out = Map::new();
    for k in fields {
        if let Some(v) = v.get(*k) {
            out.insert((*k).into(), policy_field(v, 0)?);
        }
    }
    Ok(Value::Object(out))
}
fn field(v: &Value, depth: usize) -> Result<Value> {
    if depth > 5 {
        return Err(Error("response_depth"));
    }
    match v {
        Value::String(s) if s.len() > 4096 => Err(Error("response_string_limit")),
        Value::Array(a) => {
            if a.len() > 100 {
                return Err(Error("response_array_limit"));
            }
            Ok(Value::Array(
                a.iter()
                    .map(|v| field(v, depth + 1))
                    .collect::<Result<_>>()?,
            ))
        }
        Value::Object(_) => select(
            v,
            &[
                "name",
                "isVerified",
                "isDefault",
                "type",
                "capabilityStatus",
                "servicePlanId",
                "servicePlanName",
                "provisioningStatus",
                "appliesTo",
                "enabled",
                "suspended",
                "warning",
                "lockedOut",
                "skuId",
                "disabledPlans",
                "assignedDateTime",
                "service",
            ],
            depth + 1,
        ),
        _ => Ok(v.clone()),
    }
}
pub fn select(v: &Value, fields: &[&str], depth: usize) -> Result<Value> {
    if !v.is_object() {
        return Err(Error("response_object_required"));
    }
    let mut out = Map::new();
    for k in fields {
        if let Some(value) = v.get(*k) {
            let projected = if *k == "state" && !value.is_null() {
                // Domain operation state is an explicit, bounded projection;
                // do not broaden the generic nested-object field allowlist.
                select(
                    value,
                    &["lastActionDateTime", "operation", "status"],
                    depth + 1,
                )?
            } else {
                field(value, depth + 1)?
            };
            out.insert((*k).into(), projected);
        }
    }
    Ok(Value::Object(out))
}
