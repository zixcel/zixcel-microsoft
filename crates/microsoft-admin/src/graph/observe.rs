use super::continuation::continuation;
use super::{Observation, Read, Transport, projection::select};
use crate::{Error, Result, Scope};
use serde_json::{Value, json};
pub fn observe(http: &mut impl Transport, scope: &Scope, read: Read) -> Result<Value> {
    scope.validate()?;
    let (initial, fields, many) = read.resource(&scope.tenant)?;
    super::identity::verify(http, scope, &read)?;
    let mut result = Observation::failed("not_attempted");
    let mut path = initial.clone();
    let mut rows = Vec::new();
    let mut output_bytes = 0usize;
    let mut seen_ids = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    'pages: for _ in 0..4 {
        if !seen.insert(path.clone()) {
            result.reason = "pagination_cycle".into();
            break;
        }
        let raw = match http.get(&path) {
            Ok(v) => v,
            Err(e) => {
                let (reason, status) = super::read_failure::classify(e);
                result.reason = reason.into();
                result.http_status = status;
                break;
            }
        };
        result.pages += 1;
        if matches!(&read,Read::User{user} if raw["id"].as_str()!=Some(user))
            || matches!(&read,Read::Organization if raw["id"].as_str()!=Some(&scope.tenant))
        {
            return Err(Error("response_identity_mismatch"));
        }
        let items = if many {
            raw["value"]
                .as_array()
                .ok_or(Error("collection_required"))?
                .clone()
        } else {
            vec![raw.clone()]
        };
        if items.len() > 100 || rows.len() + items.len() > 200 {
            result.reason = "item_limit".into();
            break;
        }
        for item in &items {
            if item["id"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s.len() > 256)
            {
                return Err(Error("response_identity_missing"));
            }
            if !seen_ids.insert(
                item["id"]
                    .as_str()
                    .ok_or(Error("response_identity_missing"))?
                    .to_owned(),
            ) {
                result.reason = "duplicate_resource_identity".into();
                break 'pages;
            }
            let selected = if matches!(read, Read::ConditionalAccess) {
                super::projection::policy(item, &fields)?
            } else {
                select(item, &fields, 0)?
            };
            output_bytes += serde_json::to_vec(&selected)
                .map_err(|_| Error("encoding_failed"))?
                .len()
                + 1;
            if output_bytes > 96 * 1024 {
                result.reason = "projected_output_limit".into();
                break 'pages;
            }
            rows.push(selected);
        }
        match raw.get("@odata.nextLink") {
            None => {
                result.complete = true;
                result.reason = "bounded_read_complete".into();
                break;
            }
            Some(Value::String(next)) => {
                path = continuation(next, &initial)?;
                result.reason = "page_limit".into();
            }
            _ => return Err(Error("invalid_continuation")),
        }
    }
    result.state = if result.complete {
        "supported"
    } else if result.pages > 0 {
        "partial"
    } else if result.reason == "permission_denied" {
        "denied"
    } else {
        "unobserved"
    }
    .into();
    result.data = if many {
        Value::Array(rows)
    } else {
        rows.into_iter().next().unwrap_or(Value::Null)
    };
    Ok(
        json!({"schema":"zix-microsoft-admin-read/v1","scope":scope,"request":read,"observation":result,
        "writes_performed":false,"mailbox_existence":"not_inferred_from_directory","effective_permissions":"not_inferred"}),
    )
}
