use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    exchange::Journal,
    management::{
        Graph, Plan,
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
    let mut backend = Graph::new(token.to_string(), None, timeout).map_err(|e| Error(e.0))?;
    let journal = Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
    if r.operation.ends_with("reconcile-apply") {
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
        return reconciliation::execute(&mut backend, &journal, scope, &p, now)
            .map_err(|e| Error(e.0));
    }
    // Original envelope may bind an older official version. Its inner exact digest
    // must match the active lock and permanent attempt; it cannot authorize a write.
    let original: zix_operation_contracts::LocalPlan =
        crate::input::read(arg("original-plan-file")?)?;
    if original.operation != "management.domains.apply" {
        return Err(Error("unsupported_reconciliation"));
    }
    let original: Plan =
        serde_json::from_value(original.plan).map_err(|_| Error("invalid_management_plan"))?;
    let p = reconciliation::plan(&mut backend, &journal, scope, original, now())
        .map_err(|e| Error(e.0))?;
    let envelope = zix_operation_contracts::LocalPlan::new(
        "management.domains.reconcile-apply".into(),
        crate::catalog::binding()?,
        serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
    );
    Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}))
}
