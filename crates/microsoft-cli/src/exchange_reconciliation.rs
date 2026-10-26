use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    exchange::{
        Journal, Plan, PowerShell,
        reconciliation::{self, ResolutionPlan},
    },
};
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    operation: String,
    original_plan_file: String,
    journal_directory: String,
}
pub fn selected(r: &Request) -> Result<bool, Error> {
    let (arg, pointer, expected) = match r.operation.as_str() {
        "exchange.plan" => ("change-file", "/operation", "resolve_observed_grant"),
        "exchange.apply" => (
            "plan-file",
            "/plan/schema",
            "zix-microsoft-exchange-resolution-plan/v1",
        ),
        _ => return Ok(false),
    };
    let v: Value = crate::input::read(r.arguments.get(arg).ok_or(Error("missing_argument"))?)?;
    Ok(v.pointer(pointer).and_then(Value::as_str) == Some(expected))
}
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
    let binding = crate::catalog::binding()?;
    let approved = if r.operation == "exchange.apply" {
        let envelope: zix_operation_contracts::LocalPlan = crate::input::read(arg("plan-file")?)?;
        if !envelope.matches("exchange.apply", &binding, arg("approval-sha256")?) {
            return Err(Error("plan_binding_mismatch"));
        }
        Some(
            serde_json::from_value::<ResolutionPlan>(envelope.plan)
                .map_err(|_| Error("invalid_resolution_plan"))?,
        )
    } else {
        None
    };
    let graph = zeroize::Zeroizing::new(
        std::env::var("ZIX_MICROSOFT_ACCESS_TOKEN").map_err(|_| Error("credential_missing"))?,
    );
    let exo = std::env::var("ZIX_MICROSOFT_EXCHANGE_TOKEN")
        .map_err(|_| Error("exchange_credential_missing"))?;
    let mut backend = PowerShell::new(crate::exchange::profile(r)?, exo, Some(&graph), timeout)
        .map_err(|e| Error(e.0))?;
    if let Some(p) = approved {
        let journal =
            Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
        return reconciliation::execute(&mut backend, &journal, scope, &p, now)
            .map_err(|e| Error(e.0));
    }
    let input: Input = crate::input::read(arg("change-file")?)?;
    if input.operation != "resolve_observed_grant" {
        return Err(Error("unsupported_reconciliation"));
    }
    let original: zix_operation_contracts::LocalPlan =
        crate::input::read(&input.original_plan_file)?;
    if original.operation != "exchange.apply" {
        return Err(Error("unsupported_reconciliation"));
    }
    let original: Plan =
        serde_json::from_value(original.plan).map_err(|_| Error("invalid_plan"))?;
    let journal = Journal::open(Path::new(&input.journal_directory)).map_err(|e| Error(e.0))?;
    let p = reconciliation::plan(&mut backend, &journal, scope, original, now)
        .map_err(|e| Error(e.0))?;
    let envelope = zix_operation_contracts::LocalPlan::new(
        "exchange.apply".into(),
        binding,
        serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
    );
    Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}))
}
