use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    configuration::{
        Plan,
        reconciliation::{self, ResolutionPlan},
    },
    exchange::Journal,
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
        "exchange.plan" => ("change-file", "/operation", "resolve_configuration"),
        "exchange.apply" => (
            "plan-file",
            "/plan/schema",
            "zix-microsoft-configuration-resolution-plan/v1",
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
    let arg = |key| {
        r.arguments
            .get(key)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    let binding = crate::catalog::binding()?;
    if r.operation == "exchange.apply" {
        let e: zix_operation_contracts::LocalPlan = crate::input::read(arg("plan-file")?)?;
        if !e.matches("exchange.apply", &binding, arg("approval-sha256")?) {
            return Err(Error("plan_binding_mismatch"));
        }
        let p: ResolutionPlan =
            serde_json::from_value(e.plan).map_err(|_| Error("invalid_resolution_plan"))?;
        let j = Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
        let mut backend = crate::configuration::backend(r, timeout)?;
        return reconciliation::execute(&mut backend, &j, scope, &p, now).map_err(|e| Error(e.0));
    }
    let input: Input = crate::input::read(arg("change-file")?)?;
    if input.operation != "resolve_configuration" {
        return Err(Error("invalid_resolution_operation"));
    }
    let e: zix_operation_contracts::LocalPlan = crate::input::read(&input.original_plan_file)?;
    if e.operation != "exchange.apply" {
        return Err(Error("invalid_original_operation"));
    }
    let p: Plan = serde_json::from_value(e.plan).map_err(|_| Error("invalid_original_plan"))?;
    let j = Journal::open(Path::new(&input.journal_directory)).map_err(|e| Error(e.0))?;
    let mut backend = crate::configuration::backend(r, timeout)?;
    let plan = reconciliation::plan(&mut backend, &j, scope, p, now).map_err(|e| Error(e.0))?;
    let e = zix_operation_contracts::LocalPlan::new(
        "exchange.apply".into(),
        binding,
        serde_json::to_value(plan).map_err(|_| Error("encoding_failed"))?,
    );
    Ok(json!({"plan":e,"sha256":e.digest(),"remote_writes":false}))
}
