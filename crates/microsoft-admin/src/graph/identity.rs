use super::{Read, Transport};
use crate::{Error, Result, Scope};
pub(super) fn verify(http: &mut impl Transport, scope: &Scope, read: &Read) -> Result<()> {
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
    Ok(())
}
