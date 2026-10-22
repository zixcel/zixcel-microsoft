use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zeroize::Zeroizing;
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    exchange::{self, Backend, Change, ExchangeProfile, Journal, Plan, PowerShell},
};
pub fn invoke(
    r: &Request,
    scope: &Scope,
    timeout: Duration,
    now: fn() -> u64,
) -> Result<Value, Error> {
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
    let binding = crate::catalog::binding()?;
    let change: Option<Change> = r
        .arguments
        .get("change-file")
        .map(|f| crate::input::read(f))
        .transpose()?;
    let approved = if r.operation == "exchange.apply" {
        let p: zix_operation_contracts::LocalPlan = crate::input::read(arg("plan-file")?)?;
        if !p.matches("exchange.apply", &binding, arg("approval-sha256")?) {
            return Err(Error("plan_binding_mismatch"));
        }
        let inner: Plan = serde_json::from_value(p.plan).map_err(|_| Error("invalid_plan"))?;
        if &inner.scope != scope {
            return Err(Error("plan_scope_mismatch"));
        }
        Some(inner)
    } else {
        None
    };
    let profile = std::env::var("ZIX_MICROSOFT_EXCHANGE_PROFILE")
        .map_err(|_| Error("exchange_profile_missing"))?;
    let profile: ExchangeProfile = crate::input::read(&profile)?;
    let exo = std::env::var("ZIX_MICROSOFT_EXCHANGE_TOKEN")
        .map_err(|_| Error("exchange_credential_missing"))?;
    let graph = token()?;
    let mut backend =
        PowerShell::new(profile, exo, Some(&graph), timeout).map_err(|e| Error(e.0))?;
    let value = match r.operation.as_str() {
        "exchange.snapshot" => serde_json::to_value(
            backend
                .snapshot(scope, change.as_ref())
                .map_err(|e| Error(e.0))?,
        ),
        "exchange.plan" => {
            let c = change.ok_or(Error("change_missing"))?;
            let s = backend.snapshot(scope, Some(&c)).map_err(|e| Error(e.0))?;
            let p = exchange::plan(s, c, now()).map_err(|e| Error(e.0))?;
            let envelope = zix_operation_contracts::LocalPlan::new(
                "exchange.apply".into(),
                binding,
                serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
            );
            return Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}));
        }
        "exchange.apply" => {
            let p = approved.ok_or(Error("plan_missing"))?;
            let journal =
                Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
            let hash = zixcel_microsoft_admin::digest(&p).map_err(|e| Error(e.0))?;
            serde_json::to_value(
                exchange::execute(&mut backend, &journal, &p, &hash, now)
                    .map_err(|e| Error(e.0))?,
            )
        }
        _ => return Err(Error("unknown_operation")),
    };
    value.map_err(|_| Error("encoding_failed"))
}
