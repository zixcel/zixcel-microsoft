use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    exchange::{Journal, PowerShell},
    management::{
        Backend, Graph, Plan,
        reconciliation::{self, ResolutionPlan},
    },
};
pub fn invoke(
    r: &Request,
    scope: &Scope,
    timeout: Duration,
    now: fn() -> u64,
) -> Result<Value, Error> {
    let arg = |name| {
        r.arguments
            .get(name)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    let token = zeroize::Zeroizing::new(
        std::env::var("ZIX_MICROSOFT_ACCESS_TOKEN").map_err(|_| Error("credential_missing"))?,
    );
    let is_dkim = r.operation.starts_with("management.dkim.");
    let mut backend: Box<dyn Backend> = if is_dkim {
        let exchange = zeroize::Zeroizing::new(
            std::env::var("ZIX_MICROSOFT_EXCHANGE_TOKEN")
                .map_err(|_| Error("exchange_credential_missing"))?,
        );
        Box::new(
            PowerShell::new(
                crate::exchange::profile(r)?,
                exchange.to_string(),
                Some(&token),
                timeout,
            )
            .map_err(|e| Error(e.0))?,
        )
    } else {
        Box::new(Graph::new(token.to_string(), None, timeout).map_err(|e| Error(e.0))?)
    };
    if r.operation.ends_with(".apply") {
        let journal =
            Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
        let envelope: zix_operation_contracts::LocalPlan = crate::input::read(arg("plan-file")?)?;
        if !envelope.matches(
            &r.operation,
            &crate::catalog::binding()?,
            arg("approval-sha256")?,
        ) {
            return Err(Error("plan_binding_mismatch"));
        }
        let p: ResolutionPlan =
            serde_json::from_value(envelope.plan).map_err(|_| Error("invalid_resolution_plan"))?;
        return reconciliation::execute(backend.as_mut(), &journal, scope, &p, now)
            .map_err(|e| Error(e.0));
    }
    // Original envelope may bind an older official version. Its inner exact digest
    // must match the active lock and permanent attempt; it cannot authorize a write.
    let input: ResolutionInput = crate::input::read(arg("change-file")?)?;
    if input.operation
        != if is_dkim {
            "resolve_dkim"
        } else {
            "resolve_verification"
        }
    {
        return Err(Error("unsupported_reconciliation"));
    }
    let journal = Journal::open(Path::new(&input.journal_directory)).map_err(|e| Error(e.0))?;
    let original: zix_operation_contracts::LocalPlan =
        crate::input::read(&input.original_plan_file)?;
    let apply_operation = if is_dkim {
        "management.dkim.apply"
    } else {
        "management.domains.apply"
    };
    if original.operation != apply_operation {
        return Err(Error("unsupported_reconciliation"));
    }
    let original: Plan =
        serde_json::from_value(original.plan).map_err(|_| Error("invalid_management_plan"))?;
    let p = reconciliation::plan(backend.as_mut(), &journal, scope, original, now())
        .map_err(|e| Error(e.0))?;
    let envelope = zix_operation_contracts::LocalPlan::new(
        apply_operation.into(),
        crate::catalog::binding()?,
        serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
    );
    Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolutionInput {
    operation: String,
    original_plan_file: String,
    journal_directory: String,
}
pub fn selected(r: &Request) -> Result<bool, Error> {
    let (argument, pointer, expected) = match r.operation.as_str() {
        "management.domains.plan" => ("change-file", "/operation", "resolve_verification"),
        "management.domains.apply" => (
            "plan-file",
            "/plan/schema",
            "zix-microsoft-verification-resolution-plan/v1",
        ),
        "management.dkim.plan" => ("change-file", "/operation", "resolve_dkim"),
        "management.dkim.apply" => (
            "plan-file",
            "/plan/schema",
            "zix-microsoft-dkim-resolution-plan/v1",
        ),
        _ => return Ok(false),
    };
    let path = r.arguments.get(argument).ok_or(Error("missing_argument"))?;
    let v: Value = crate::input::read(path)?;
    Ok(v.pointer(pointer).and_then(Value::as_str) == Some(expected))
}
