use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zeroize::Zeroizing;
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    configuration::{self, Backend, Change, Plan, Read},
    exchange::{Journal, PowerShell},
};
pub fn selected(r: &Request) -> Result<bool, Error> {
    if r.operation == "exchange.snapshot" {
        return Ok(r.arguments.contains_key("resource"));
    }
    let (argument, pointer) = match r.operation.as_str() {
        "exchange.plan" => ("change-file", "/operation"),
        "exchange.apply" => ("plan-file", "/plan/schema"),
        _ => return Ok(false),
    };
    let v: Value = crate::input::read(r.arguments.get(argument).ok_or(Error("missing_argument"))?)?;
    Ok(v.pointer(pointer).and_then(Value::as_str).is_some_and(|s| {
        [
            "configure",
            "create_inbox_rule",
            "update_inbox_rule",
            "delete_inbox_rule",
            "set_inbox_rule_enabled",
            "set_auto_mapping",
            "zix-microsoft-configuration-plan/v1",
        ]
        .contains(&s)
    }))
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
    let mut change = None;
    let mut approved = None;
    let mut selector = None;
    match r.operation.as_str() {
        "exchange.snapshot" => {
            if r.arguments.contains_key("change-file") {
                return Err(Error("ambiguous_read_selector"));
            }
            let mut v = json!({"resource":arg("resource")?});
            for key in ["mailbox", "kind", "folder", "trustee", "identity"] {
                if let Some(s) = r.arguments.get(key) {
                    v[key] = json!(s);
                }
            }
            let read: Read =
                serde_json::from_value(v).map_err(|_| Error("invalid_configuration_selector"))?;
            read.validate().map_err(|e| Error(e.0))?;
            selector = Some(read);
        }
        "exchange.plan" => {
            let c: Change = crate::input::read(arg("change-file")?)?;
            c.validate().map_err(|e| Error(e.0))?;
            change = Some(c);
        }
        "exchange.apply" => {
            let envelope: zix_operation_contracts::LocalPlan =
                crate::input::read(arg("plan-file")?)?;
            if !envelope.matches("exchange.apply", &binding, arg("approval-sha256")?) {
                return Err(Error("plan_binding_mismatch"));
            }
            let p: Plan = serde_json::from_value(envelope.plan)
                .map_err(|_| Error("invalid_configuration_plan"))?;
            if p.scope != *scope {
                return Err(Error("plan_scope_mismatch"));
            }
            p.change.validate().map_err(|e| Error(e.0))?;
            approved = Some(p);
        }
        _ => return Err(Error("unknown_operation")),
    }
    let mut backend = backend(r, timeout)?;
    if let Some(read) = selector {
        let observation = backend
            .inspect(scope, &read, None)
            .map_err(|e| Error(e.0))?;
        return Ok(
            json!({"schema":"zix-microsoft-exchange-configuration-read/v1","scope":scope,"selector":read,"observation":observation,"writes_performed":false,"effective_permissions":"not_inferred"}),
        );
    }
    if let Some(c) = change {
        let p = configuration::plan(&mut backend, scope.clone(), c, now).map_err(|e| Error(e.0))?;
        let envelope = zix_operation_contracts::LocalPlan::new(
            "exchange.apply".into(),
            binding,
            serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
        );
        return Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}));
    }
    let p = approved.ok_or(Error("plan_missing"))?;
    let j = Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
    let hash = zixcel_microsoft_admin::digest(&p).map_err(|e| Error(e.0))?;
    configuration::execute(&mut backend, &j, &p, &hash, now).map_err(|e| Error(e.0))
}

pub(super) fn backend(r: &Request, timeout: Duration) -> Result<PowerShell, Error> {
    let graph = Zeroizing::new(
        std::env::var("ZIX_MICROSOFT_ACCESS_TOKEN").map_err(|_| Error("credential_missing"))?,
    );
    let exo = std::env::var("ZIX_MICROSOFT_EXCHANGE_TOKEN")
        .map_err(|_| Error("exchange_credential_missing"))?;
    let backend = PowerShell::new(crate::exchange::profile(r)?, exo, Some(&graph), timeout)
        .map_err(|e| Error(e.0))?;
    Ok(backend)
}
