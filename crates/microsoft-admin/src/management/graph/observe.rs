use super::{Change, Error, Graph, Result, Scope, Value, json, selected, user};
pub(super) fn state(g: &mut Graph, scope: &Scope, c: &Change) -> Result<Value> {
    let target = match c {
        Change::AddDomain { domain } | Change::VerifyDomain { domain } => {
            let v=g.get(&format!("/v1.0/domains/{domain}?$select=id,isVerified,isDefault,isInitial,authenticationType,supportedServices,state"),matches!(c,Change::AddDomain{..}))?;
            if v.is_null() {
                return Ok(json!({"target":null,"eligible":true}));
            }
            let v = selected(
                &v,
                &[
                    "id",
                    "isVerified",
                    "isDefault",
                    "isInitial",
                    "authenticationType",
                    "supportedServices",
                    "state",
                ],
            )?;
            if v["id"] != *domain {
                return Err(Error("domain_mismatch"));
            }
            v
        }
        Change::CreateUser { upn, .. } => {
            let domain = upn.split_once('@').ok_or(Error("invalid_address"))?.1;
            let d = g.get(
                &format!("/v1.0/domains/{domain}?$select=id,isVerified"),
                false,
            )?;
            if d["id"] != domain || d["isVerified"] != true {
                return Err(Error("domain_unverified"));
            }
            user(g, upn, true)?
        }
        Change::SetUserPrincipalName { user: id, upn } => {
            let domain = upn.split_once('@').ok_or(Error("invalid_address"))?.1;
            let d = g.get(
                &format!("/v1.0/domains/{domain}?$select=id,isVerified"),
                false,
            )?;
            if d["id"] != domain || d["isVerified"] != true {
                return Err(Error("domain_unverified"));
            }
            let target = user(g, id, false)?;
            let source = g.get(
                &format!("/v1.0/users/{id}?$select=id,onPremisesSyncEnabled"),
                false,
            )?;
            if source["id"] != *id
                || !source
                    .as_object()
                    .is_some_and(|o| o.contains_key("onPremisesSyncEnabled"))
            {
                return Err(Error("incomplete_user_source"));
            }
            let available = user(g, upn, true)?;
            return Ok(
                json!({"target":target,"eligible":source["onPremisesSyncEnabled"] != true && (available.is_null() || available["id"] == *id),"on_premises_sync_enabled":source["onPremisesSyncEnabled"]}),
            );
        }
        Change::SetUsageLocation { user: id, .. } | Change::SetUserEnabled { user: id, .. } => {
            user(g, id, false)?
        }
        Change::AssignLicense { user: id, sku } => {
            let target = user(g, id, false)?;
            let licenses = g.get(
                "/v1.0/subscribedSkus?$select=skuId,capabilityStatus,consumedUnits,prepaidUnits",
                false,
            )?;
            if licenses.get("@odata.nextLink").is_some() {
                return Err(Error("incomplete_licenses"));
            }
            let values = licenses["value"]
                .as_array()
                .ok_or(Error("incomplete_licenses"))?;
            if values.len() > 200 {
                return Err(Error("license_observation_limit"));
            }
            let mut found = values.iter().filter(|v| v["skuId"] == *sku);
            let license = found.next().ok_or(Error("sku_not_subscribed"))?;
            if found.next().is_some() {
                return Err(Error("ambiguous_sku"));
            }
            let capacity = license["prepaidUnits"]["enabled"]
                .as_u64()
                .ok_or(Error("incomplete_sku"))?;
            let used = license["consumedUnits"]
                .as_u64()
                .ok_or(Error("incomplete_sku"))?;
            return Ok(
                json!({"target":target,"sku":selected(license,&["skuId","capabilityStatus","consumedUnits","prepaidUnits"] )?,"eligible":license["capabilityStatus"]=="Enabled"&&capacity>used}),
            );
        }
        Change::SendProbe { correlation, .. } => {
            return Ok(json!({"sender":scope.subject,"target":probe(g,correlation,false)?}));
        }
        _ => return Err(Error("unsupported_graph_change")),
    };
    Ok(json!({"target":target,"eligible":true}))
}
pub(super) fn probe(g: &mut Graph, correlation: &str, incoming: bool) -> Result<Value> {
    let folder = if incoming { "inbox" } else { "sentitems" };
    let subject = format!("zix-mail-probe-{correlation}");
    let v=g.get(&format!("/v1.0/me/mailFolders/{folder}/messages?$filter=subject%20eq%20'{subject}'&$select=id,subject,toRecipients,receivedDateTime,sentDateTime&$top=2"),false)?;
    if v.get("@odata.nextLink").is_some() {
        return Err(Error("ambiguous_mail_probe"));
    }
    let values = v["value"]
        .as_array()
        .ok_or(Error("invalid_mail_observation"))?;
    if values.is_empty() {
        return Ok(Value::Null);
    }
    if values.len() != 1 || values[0]["subject"] != subject {
        return Err(Error("ambiguous_mail_probe"));
    }
    let r = values[0]["toRecipients"]
        .as_array()
        .ok_or(Error("invalid_mail_observation"))?;
    if r.len() != 1 {
        return Err(Error("ambiguous_mail_probe"));
    }
    let recipient = r[0]["emailAddress"]["address"]
        .as_str()
        .ok_or(Error("invalid_mail_observation"))?
        .to_ascii_lowercase();
    crate::email(&recipient)?;
    Ok(
        json!({"id":values[0]["id"].as_str().ok_or(Error("invalid_mail_observation"))?,"recipient":recipient,"correlation":correlation,"location":folder}),
    )
}
