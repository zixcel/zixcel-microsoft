use super::{profile, storage};
use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::http::Http;
mod token;
#[derive(serde::Deserialize)]
struct Device {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    #[serde(default = "interval")]
    interval: u64,
}
fn interval() -> u64 {
    5
}
impl Drop for Device {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.device_code.zeroize();
    }
}
pub(super) fn now() -> Result<u64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| Error("clock_unavailable"))
}
pub(super) fn invoke(r: &Request) -> Result<Value, Error> {
    let arg = |name| {
        r.arguments
            .get(name)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    if r.operation == "auth.configure" {
        let mut p = profile::Profile {
            tenant: arg("tenant")?.into(),
            client_id: arg("client-id")?.into(),
            collection: arg("collection")?.into(),
            graph_permissions: r
                .arguments
                .get("graph-permissions")
                .map(|s| s.split(',').map(str::to_owned).collect())
                .unwrap_or_else(|| vec!["User.Read".into(), "Organization.Read.All".into()]),
            expected_subject: r.arguments.get("subject").cloned(),
        };
        profile::validate(&mut p)?;
        crate::input::write_private_new(arg("output")?, &p)?;
        return Ok(
            json!({"profile":arg("output")?,"stored":true,"client_registration":"owner_provisioned_public_client_required","tenant_configuration_changed":false}),
        );
    }
    let path = arg("profile")?;
    let p = profile::load(path)?;
    let resource = arg("resource")?;
    let scopes = p.scopes(resource)?;
    let _lock = storage::Lock::acquire(path)?;
    let store = storage::connect(&p)?;
    let timestamp = now()?;
    if r.operation == "auth.logout" {
        storage::delete(&store, &p, resource, false)?;
        storage::delete(&store, &p, resource, true)?;
        return Ok(json!({"local_credentials_removed":true,"remote_consent_revoked":false}));
    }
    if r.operation == "auth.status" {
        let c = storage::read(&store, &p, resource, false)?;
        return Ok(
            json!({"reference":p.reference(resource,false).map(|r|r.canonical_id())?,"resource":resource,"tenant":p.tenant,"subject":c.subject,"expires_at":c.expires_at,"state":if c.expires_at>timestamp{"not_expired"}else{"refresh_required"},"exchange_subject_verification":"performed_by_Exchange_on_use","permissions":"not_inferred_from_cache"}),
        );
    }
    let timeout = Duration::from_millis(r.limits.timeout_ms.saturating_sub(100).max(50));
    let mut http =
        Http::new("https://login.microsoftonline.com", timeout).map_err(|e| Error(e.0))?;
    if r.operation == "auth.start" {
        let response = http
            .oauth(
                &p.tenant,
                "devicecode",
                &[("client_id", &p.client_id), ("scope", &scopes)],
            )
            .map_err(|e| Error(e.0))?;
        response.require_success().map_err(|e| Error(e.0))?;
        let d: Device = serde_json::from_slice(&response.bytes)
            .map_err(|_| Error("invalid_device_authorization_response"))?;
        if ![
            "https://microsoft.com/devicelogin",
            "https://www.microsoft.com/devicelogin",
        ]
        .contains(&d.verification_uri.as_str())
            || d.user_code.is_empty()
            || d.user_code.len() > 32
            || !d
                .user_code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || d.device_code.is_empty()
            || d.device_code.len() > 16384
            || !(1..=1800).contains(&d.expires_in)
            || !(1..=60).contains(&d.interval)
        {
            return Err(Error("invalid_device_authorization_response"));
        }
        let c = storage::Cache {
            profile: p.hash()?,
            resource: resource.into(),
            device_code: d.device_code.clone(),
            expires_at: timestamp + d.expires_in,
            interval: d.interval,
            next_poll: timestamp + d.interval,
            subject: None,
            access_token: String::new(),
            refresh_token: String::new(),
        };
        storage::put(&store, &p, resource, true, &c)?;
        return Ok(
            json!({"verification_uri":d.verification_uri,"user_code":d.user_code,"expires_at":c.expires_at,"poll_after_seconds":d.interval,"state":"browser_authorization_required","token_values_returned":false}),
        );
    }
    token::acquire(
        &p,
        resource,
        &store,
        &mut http,
        r.operation.as_str(),
        timestamp,
        timeout,
    )
}
