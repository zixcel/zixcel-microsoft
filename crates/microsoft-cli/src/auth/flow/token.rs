use super::{Duration, Error, Http, Value, json, profile, storage};
use crowsi_credential_broker::CredentialStore;
use zixcel_microsoft_admin::http::Response;
pub(super) trait OAuth {
    fn token(
        &mut self,
        tenant: &str,
        form: &[(&str, &str)],
    ) -> zixcel_microsoft_admin::Result<Response>;
}
impl OAuth for Http {
    fn token(
        &mut self,
        tenant: &str,
        form: &[(&str, &str)],
    ) -> zixcel_microsoft_admin::Result<Response> {
        self.oauth(tenant, "token", form)
    }
}

use zeroize::Zeroize;
#[derive(serde::Deserialize)]
struct Tokens {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    token_type: String,
    expires_in: u64,
}
impl Drop for Tokens {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}
#[derive(serde::Deserialize)]
struct OAuthError {
    error: String,
}
pub(super) fn acquire(
    p: &profile::Profile,
    resource: &str,
    store: &impl CredentialStore,
    http: &mut impl OAuth,
    operation: &str,
    timestamp: u64,
    timeout: Duration,
) -> Result<Value, Error> {
    let scopes = p.scopes(resource)?;
    let polling = operation == "auth.poll";
    if !polling && operation != "auth.refresh" {
        return Err(Error("unknown_operation"));
    }
    let mut cache = storage::read(store, p, resource, polling)?;
    let response = if polling {
        if cache.expires_at <= timestamp {
            return Err(Error("device_authorization_expired"));
        }
        if cache.next_poll > timestamp {
            return Ok(json!({"state":"pending","poll_after_seconds":cache.next_poll-timestamp}));
        }
        if !(1..=60).contains(&cache.interval) || cache.device_code.is_empty() {
            return Err(Error("invalid_auth_cache"));
        }
        cache.next_poll = timestamp
            .checked_add(cache.interval)
            .ok_or(Error("invalid_clock"))?;
        storage::put(store, p, resource, true, &cache)?;
        http.token(
            &p.tenant,
            &[
                ("client_id", &p.client_id),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &cache.device_code),
            ],
        )
    } else {
        if cache.refresh_token.is_empty() {
            return Err(Error("interactive_authentication_required"));
        }
        http.token(
            &p.tenant,
            &[
                ("client_id", &p.client_id),
                ("grant_type", "refresh_token"),
                ("refresh_token", &cache.refresh_token),
                ("scope", &scopes),
            ],
        )
    }
    .map_err(|e| Error(e.0))?;
    if response.status != 200 {
        let e: OAuthError = serde_json::from_slice(&response.bytes)
            .map_err(|_| Error("authentication_service_failed"))?;
        if polling && ["authorization_pending", "slow_down"].contains(&e.error.as_str()) {
            if e.error == "slow_down" {
                cache.interval = cache
                    .interval
                    .checked_add(5)
                    .filter(|n| *n <= 60)
                    .ok_or(Error("poll_interval_exceeds_limit"))?;
            }
            cache.next_poll = timestamp + cache.interval;
            storage::put(store, p, resource, true, &cache)?;
            return Ok(json!({"state":"pending","poll_after_seconds":cache.interval}));
        }
        return Err(Error(match e.error.as_str() {
            "authorization_declined" | "access_denied" => "authorization_declined",
            "expired_token" => "device_authorization_expired",
            "invalid_grant" => "interactive_authentication_required",
            "invalid_client" | "unauthorized_client" => "public_client_configuration_rejected",
            _ => "authentication_service_failed",
        }));
    }
    let mut t: Tokens =
        serde_json::from_slice(&response.bytes).map_err(|_| Error("invalid_token_response"))?;
    validate_tokens(&t)?;
    if resource == "graph" {
        let subject = verify_graph(p, &t.access_token, timeout)?;
        if !polling && cache.subject.as_ref() != Some(&subject) {
            return Err(Error("auth_subject_mismatch"));
        }
        cache.subject = Some(subject);
    }
    cache.access_token.zeroize();
    cache.access_token = std::mem::take(&mut t.access_token);
    if !t.refresh_token.is_empty() {
        cache.refresh_token.zeroize();
        cache.refresh_token = std::mem::take(&mut t.refresh_token);
    }
    cache.device_code.zeroize();
    cache.expires_at = timestamp + t.expires_in;
    cache.interval = 0;
    cache.next_poll = 0;
    storage::put(store, p, resource, false, &cache)?;
    if polling {
        storage::delete(store, p, resource, true)?;
    }
    Ok(
        json!({"state":"authenticated","resource":resource,"tenant":p.tenant,"subject":cache.subject,"reference":p.reference(resource,false)?.canonical_id(),"expires_at":cache.expires_at,"token_values_returned":false}),
    )
}
fn validate_tokens(t: &Tokens) -> Result<(), Error> {
    if !t.token_type.eq_ignore_ascii_case("Bearer")
        || t.access_token.is_empty()
        || t.access_token.len() > 16384
        || !t.access_token.is_ascii()
        || t.access_token.bytes().any(|b| b.is_ascii_control())
        || t.refresh_token.len() > 32768
        || !t.refresh_token.is_ascii()
        || t.refresh_token.bytes().any(|b| b.is_ascii_control())
        || !(1..=172800).contains(&t.expires_in)
    {
        return Err(Error("invalid_token_response"));
    }
    Ok(())
}
fn verify_graph(p: &profile::Profile, token: &str, timeout: Duration) -> Result<String, Error> {
    let mut h = Http::new("https://graph.microsoft.com", timeout).map_err(|e| Error(e.0))?;
    let me = h
        .graph("GET", "/v1.0/me?$select=id", token, None)
        .map_err(|e| Error(e.0))?;
    me.require_success().map_err(|e| Error(e.0))?;
    let subject = me.json().map_err(|e| Error(e.0))?["id"]
        .as_str()
        .ok_or(Error("auth_subject_missing"))?
        .to_owned();
    zixcel_microsoft_admin::uuid(&subject).map_err(|e| Error(e.0))?;
    if p.expected_subject.as_ref().is_some_and(|s| s != &subject) {
        return Err(Error("auth_subject_mismatch"));
    }
    let org = h
        .graph(
            "GET",
            &format!("/v1.0/organization/{}?$select=id", p.tenant),
            token,
            None,
        )
        .map_err(|e| Error(e.0))?;
    org.require_success().map_err(|e| Error(e.0))?;
    if org.json().map_err(|e| Error(e.0))?["id"] != p.tenant {
        return Err(Error("auth_tenant_mismatch"));
    }
    Ok(subject)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_shape_and_lifetime_are_bounded() {
        let good = r#"{"access_token":"synthetic-access","refresh_token":"synthetic-refresh","token_type":"Bearer","expires_in":3600}"#;
        let mut t: Tokens = serde_json::from_str(good).unwrap();
        assert!(validate_tokens(&t).is_ok());
        t.expires_in = 0;
        assert!(validate_tokens(&t).is_err());
        t.expires_in = 3600;
        t.access_token = "contains\ncontrol".into();
        assert!(validate_tokens(&t).is_err());
        t.access_token = "valid".into();
        t.token_type = "Basic".into();
        assert!(validate_tokens(&t).is_err());
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crowsi_credential_broker::MemoryStore;
    use zeroize::Zeroizing;
    struct Fake {
        status: u16,
        body: &'static str,
        calls: usize,
    }
    impl OAuth for Fake {
        fn token(
            &mut self,
            _: &str,
            form: &[(&str, &str)],
        ) -> zixcel_microsoft_admin::Result<Response> {
            self.calls += 1;
            assert!(form.iter().any(|(k, _)| *k == "client_id"));
            Ok(Response {
                status: self.status,
                bytes: Zeroizing::new(self.body.as_bytes().to_vec()),
            })
        }
    }
    fn profile() -> profile::Profile {
        profile::Profile {
            tenant: "11111111-1111-4111-8111-111111111111".into(),
            client_id: "22222222-2222-4222-8222-222222222222".into(),
            collection: "default".into(),
            graph_permissions: vec!["Organization.Read.All".into(), "User.Read".into()],
            expected_subject: None,
        }
    }
    #[test]
    fn pending_slowdown_token_replacement_and_local_logout_stay_in_custody() {
        let p = profile();
        let store = MemoryStore::default();
        let c = storage::Cache {
            profile: p.hash().unwrap(),
            resource: "exchange".into(),
            subject: None,
            access_token: String::new(),
            refresh_token: String::new(),
            device_code: "synthetic-device".into(),
            interval: 5,
            next_poll: 105,
            expires_at: 500,
        };
        storage::put(&store, &p, "exchange", true, &c).unwrap();
        let mut h = Fake {
            status: 400,
            body: r#"{"error":"slow_down","error_description":"must not propagate"}"#,
            calls: 0,
        };
        let r = acquire(
            &p,
            "exchange",
            &store,
            &mut h,
            "auth.poll",
            101,
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(r["poll_after_seconds"], 4);
        assert_eq!(h.calls, 0);
        let r = acquire(
            &p,
            "exchange",
            &store,
            &mut h,
            "auth.poll",
            105,
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(r["poll_after_seconds"], 10);
        assert_eq!(h.calls, 1);
        h.status = 200;
        h.body = r#"{"access_token":"synthetic-access","refresh_token":"synthetic-refresh","token_type":"Bearer","expires_in":3600}"#;
        let r = acquire(
            &p,
            "exchange",
            &store,
            &mut h,
            "auth.poll",
            115,
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(r["state"], "authenticated");
        assert!(!r.to_string().contains("synthetic-"));
        assert!(storage::read(&store, &p, "exchange", true).is_err());
        let c = storage::read(&store, &p, "exchange", false).unwrap();
        assert_eq!(c.refresh_token, "synthetic-refresh");
        h.body = r#"{"access_token":"replacement-access","refresh_token":"replacement-refresh","token_type":"Bearer","expires_in":3600}"#;
        acquire(
            &p,
            "exchange",
            &store,
            &mut h,
            "auth.refresh",
            200,
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(
            storage::read(&store, &p, "exchange", false)
                .unwrap()
                .refresh_token,
            "replacement-refresh"
        );
        h.status = 400;
        h.body = r#"{"error":"invalid_grant","error_description":"secret diagnostic"}"#;
        assert_eq!(
            acquire(
                &p,
                "exchange",
                &store,
                &mut h,
                "auth.refresh",
                201,
                Duration::from_secs(1)
            )
            .unwrap_err()
            .0,
            "interactive_authentication_required"
        );
        storage::delete(&store, &p, "exchange", false).unwrap();
        assert!(storage::read(&store, &p, "exchange", false).is_err());
    }
}
