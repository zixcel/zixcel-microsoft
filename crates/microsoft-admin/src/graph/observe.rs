use super::continuation::continuation;
use super::{Observation, ReadError, Transport, projection::select};
use crate::{Error, Result, Scope};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Organization,
    Licenses,
    Domains,
    Users,
    User { user: String },
    DomainDns { domain: String },
}
pub fn observe(http: &mut impl Transport, scope: &Scope, read: Read) -> Result<Value> {
    scope.validate()?;
    let (initial, fields, many) = read.resource(&scope.tenant)?;
    let identity = http
        .get("/v1.0/me?$select=id")
        .map_err(|_| Error("subject_unverified"))?;
    if identity["id"].as_str() != Some(&scope.subject) {
        return Err(Error("subject_mismatch"));
    }
    let tenant_path = format!("/v1.0/organization/{}?$select=id", scope.tenant);
    let tenant = http
        .get(&tenant_path)
        .map_err(|_| Error("tenant_unverified"))?;
    if tenant["id"].as_str() != Some(&scope.tenant) {
        return Err(Error("tenant_mismatch"));
    }
    if let Read::DomainDns { domain } = &read {
        let d = http
            .get(&format!("/v1.0/domains/{domain}?$select=id,isVerified"))
            .map_err(|_| Error("domain_unverified"))?;
        if d["id"].as_str() != Some(domain) || d["isVerified"] != true {
            return Err(Error("domain_unverified"));
        }
    }
    let mut result = Observation::failed("not_attempted");
    let mut path = initial.clone();
    let mut rows = Vec::new();
    let mut output_bytes = 0usize;
    let mut seen = std::collections::BTreeSet::new();
    'pages: for _ in 0..4 {
        if !seen.insert(path.clone()) {
            result.reason = "pagination_cycle".into();
            break;
        }
        let raw = match http.get(&path) {
            Ok(v) => v,
            Err(e) => {
                result.reason = match e {
                    ReadError::Http(401 | 403) => "permission_denied",
                    ReadError::Http(404) => "not_found_or_hidden",
                    ReadError::Budget => "request_or_byte_limit",
                    _ => "transport_or_schema_failure",
                }
                .into();
                break;
            }
        };
        result.pages += 1;
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
            let selected = select(item, &fields, 0)?;
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
