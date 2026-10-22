use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;
use zix_native_provider::{Error, Failure, Request};
use zixcel_microsoft_admin::{
    Scope,
    graph::{LiveRead, Read, Transport},
};
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
pub fn execute(r: &Request) -> Result<Value, Failure> {
    invoke(r).map_err(Failure::from)
}
fn invoke(r: &Request) -> Result<Value, Error> {
    zix_native_provider::validate_request(&crate::manifest()?, r)?;
    if r.operation == "admin.doctor" {
        return crate::doctor::report();
    }
    let arg = |k| {
        r.arguments
            .get(k)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    let token = || {
        std::env::var("ZIX_MICROSOFT_ACCESS_TOKEN")
            .map(Zeroizing::new)
            .map_err(|_| Error("credential_missing"))
    };
    let timeout = Duration::from_millis(r.limits.timeout_ms.saturating_sub(100).max(50));
    if r.operation == "users.read" {
        let p = crate::provider()?;
        let mut old = r.clone();
        old.limits.timeout_ms = old.limits.timeout_ms.min(20000);
        old.binding = None;
        let prepared = p.prepare(&old)?;
        let t = token()?;
        let mut http = LiveRead::new(
            &t,
            Duration::from_millis(old.limits.timeout_ms.saturating_sub(20).max(50)),
        )
        .map_err(|_| Error("transport_setup"))?;
        let path = prepared
            .url
            .strip_prefix("https://graph.microsoft.com")
            .ok_or(Error("origin_mismatch"))?;
        let v = http.get(path).map_err(|_| Error("provider_read_failed"))?;
        return p.project(
            &old,
            &serde_json::to_vec(&v).map_err(|_| Error("encoding_failed"))?,
        );
    }
    let scope = Scope {
        tenant: arg("tenant")?.into(),
        subject: arg("subject")?.into(),
    };
    scope.validate().map_err(|e| Error(e.0))?;
    if r.operation.starts_with("admin.") {
        let op = match r.operation.as_str() {
            "admin.organization" => Read::Organization,
            "admin.licenses" => Read::Licenses,
            "admin.domains" => Read::Domains,
            "admin.users" => Read::Users,
            "admin.user" => Read::User {
                user: arg("user")?.into(),
            },
            "admin.domain-dns" => Read::DomainDns {
                domain: arg("domain")?.into(),
            },
            _ => return Err(Error("unknown_operation")),
        };
        // Validate domain inputs before reading credentials.
        if let Read::DomainDns { domain } = &op {
            zixcel_microsoft_admin::domain(domain).map_err(|e| Error(e.0))?;
        }
        if let Read::User { user } = &op {
            zixcel_microsoft_admin::uuid(user).map_err(|e| Error(e.0))?;
        }
        let t = token()?;
        let mut http = LiveRead::new(&t, timeout).map_err(|_| Error("transport_setup"))?;
        return zixcel_microsoft_admin::graph::observe(&mut http, &scope, op)
            .map_err(|e| Error(e.0));
    }
    crate::exchange::invoke(r, &scope, timeout, now)
}
