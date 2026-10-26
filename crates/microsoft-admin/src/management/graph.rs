use super::{Backend, Change, FailurePhase, MutationFailure};
use crate::{Error, Result, Scope, http::Http};
use serde_json::{Value, json};
use std::time::Duration;
use zeroize::Zeroizing;
mod mutate;
mod observe;
pub struct Graph {
    http: Http,
    token: Zeroizing<String>,
    password: Option<Zeroizing<String>>,
    last_failure: Option<MutationFailure>,
}
impl Graph {
    pub fn new(token: String, password: Option<String>, timeout: Duration) -> Result<Self> {
        let token = Zeroizing::new(token);
        let password = password.map(Zeroizing::new);
        if token.is_empty() || token.len() > 16384 || token.bytes().any(|b| b.is_ascii_control()) {
            return Err(Error("credential_invalid"));
        }
        Ok(Self {
            http: Http::new("https://graph.microsoft.com", timeout)?,
            token,
            password,
            last_failure: None,
        })
    }
    fn get(&mut self, path: &str, absent: bool) -> Result<Value> {
        let r = self.http.graph("GET", path, &self.token, None)?;
        if absent && r.status == 404 {
            return Ok(Value::Null);
        }
        r.require_success()?;
        r.json()
    }
    fn identity(&mut self, scope: &Scope) -> Result<()> {
        scope.validate()?;
        if self.get("/v1.0/me?$select=id", false)?["id"] != scope.subject {
            return Err(Error("subject_mismatch"));
        }
        if self.get(
            &format!("/v1.0/organization/{}?$select=id", scope.tenant),
            false,
        )?["id"]
            != scope.tenant
        {
            return Err(Error("tenant_mismatch"));
        }
        Ok(())
    }
    pub fn probe(&mut self, scope: &Scope, correlation: &str, incoming: bool) -> Result<Value> {
        crate::uuid(correlation)?;
        self.identity(scope)?;
        observe::probe(self, correlation, incoming)
    }
}
impl Backend for Graph {
    fn observe(&mut self, scope: &Scope, change: &Change) -> Result<Value> {
        change.validate()?;
        self.identity(scope)?;
        observe::state(self, scope, change)
    }
    fn prepare(&self, c: &Change) -> Result<()> {
        if matches!(c, Change::CreateUser { .. }) {
            let p = self
                .password
                .as_ref()
                .ok_or(Error("initial_password_missing"))?;
            if p.len() < 16
                || p.len() > 256
                || !p.is_ascii()
                || p.bytes().any(|b| b.is_ascii_control())
            {
                return Err(Error("invalid_initial_password"));
            }
        }
        Ok(())
    }
    fn mutate(&mut self, scope: &Scope, change: &Change) -> Result<()> {
        self.last_failure = None;
        self.identity(scope).inspect_err(|error| {
            self.last_failure = Some(MutationFailure::from_error(FailurePhase::Identity, *error));
        })?;
        self.prepare(change).inspect_err(|error| {
            self.last_failure = Some(MutationFailure::from_error(FailurePhase::Prepare, *error));
        })?;
        let (method, path, body) =
            mutate::request(change, self.password.as_deref().map(String::as_str))?;
        let r = self.http.graph(method, &path, &self.token, Some(&body));
        // The user password is transient request data, never retained in the plan.
        let mut body = body;

        if let Some(Value::String(s)) = body.pointer_mut("/passwordProfile/password") {
            use zeroize::Zeroize;
            s.zeroize();
        }
        let response = r.inspect_err(|error| {
            self.last_failure = Some(MutationFailure::from_error(FailurePhase::Request, *error));
        })?;
        response.require_success().inspect_err(|_| {
            self.last_failure = Some(MutationFailure::from_http(response.status, &response.bytes));
        })
    }
    fn mutation_failure(&self) -> Option<MutationFailure> {
        self.last_failure.clone()
    }
}
pub(super) fn selected(v: &Value, fields: &[&str]) -> Result<Value> {
    if !v.is_object() {
        return Err(Error("incomplete_observation"));
    }
    let mut out = serde_json::Map::new();
    for name in fields {
        out.insert(
            (*name).into(),
            v.get(*name)
                .cloned()
                .ok_or(Error("incomplete_observation"))?,
        );
    }
    Ok(Value::Object(out))
}
pub(super) fn user(g: &mut Graph, id: &str, absent: bool) -> Result<Value> {
    let v=g.get(&format!("/v1.0/users/{id}?$select=id,userPrincipalName,displayName,mailNickname,usageLocation,accountEnabled,assignedLicenses"),absent)?;
    if v.is_null() {
        return Ok(v);
    }
    let mut v = selected(
        &v,
        &[
            "id",
            "userPrincipalName",
            "displayName",
            "mailNickname",
            "usageLocation",
            "accountEnabled",
            "assignedLicenses",
        ],
    )?;
    crate::uuid(v["id"].as_str().ok_or(Error("incomplete_user"))?)?;
    if id.contains('@') {
        if v["userPrincipalName"] != id {
            return Err(Error("user_mismatch"));
        }
    } else if v["id"] != id {
        return Err(Error("user_mismatch"));
    }
    let a = v["assignedLicenses"]
        .as_array_mut()
        .ok_or(Error("incomplete_licenses"))?;
    if a.len() > 100 {
        return Err(Error("license_observation_limit"));
    }
    for l in a.iter_mut() {
        *l = selected(l, &["skuId", "disabledPlans"])?;
        if let Some(plans) = l["disabledPlans"].as_array_mut() {
            plans.sort_by_key(Value::to_string);
        } else {
            return Err(Error("incomplete_licenses"));
        }
    }
    a.sort_by_key(|v| v["skuId"].to_string());
    Ok(v)
}
