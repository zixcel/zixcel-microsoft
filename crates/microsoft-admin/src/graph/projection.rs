use crate::{Error, Result};
use serde_json::{Map, Value};
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
