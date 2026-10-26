use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use zix_native_provider::{Error, Request};
use zixcel_microsoft_admin::{
    Scope,
    exchange::{Journal, PowerShell},
    management::{self, Backend, Change, Plan},
};
pub fn invoke(
    r: &Request,
    scope: &Scope,
    timeout: Duration,
    now: fn() -> u64,
) -> Result<Value, Error> {
    if crate::reconciliation::selected(r)? {
        return crate::reconciliation::invoke(r, scope, timeout, now);
    }
    let arg = |n| {
        r.arguments
            .get(n)
            .map(String::as_str)
            .ok_or(Error("missing_argument"))
    };
    let token = std::env::var("ZIX_MICROSOFT_ACCESS_TOKEN")
        .map(zeroize::Zeroizing::new)
        .map_err(|_| Error("credential_missing"))?;
    if r.operation == "management.mail.inspect" {
        let mut g =
            management::Graph::new(token.to_string(), None, timeout).map_err(|e| Error(e.0))?;
        let result = g
            .probe(scope, arg("correlation")?, true)
            .map_err(|e| Error(e.0))?;
        return Ok(
            json!({"state":if result.is_null(){"not_observed"}else{"received_in_authenticated_inbox"},"message":result,"observation_complete_for_other_folders":false}),
        );
    }
    let inspect = r.operation == "management.dkim.inspect";
    let apply = r.operation.ends_with(".apply");
    let approved: Option<Plan> = if apply {
        let e: zix_operation_contracts::LocalPlan = crate::input::read(arg("plan-file")?)?;
        if !e.matches(
            &r.operation,
            &crate::catalog::binding()?,
            arg("approval-sha256")?,
        ) {
            return Err(Error("plan_binding_mismatch"));
        }
        let p: Plan =
            serde_json::from_value(e.plan).map_err(|_| Error("invalid_management_plan"))?;
        if p.scope != *scope {
            return Err(Error("plan_scope_mismatch"));
        }
        Some(p)
    } else {
        None
    };
    let change = if inspect {
        Change::PrepareDkim {
            domain: arg("domain")?.into(),
        }
    } else if let Some(p) = &approved {
        p.change.clone()
    } else {
        crate::input::read(arg("change-file")?)?
    };
    change.validate().map_err(|e| Error(e.0))?;
    if r.operation.split('.').nth(1) != Some(change.group()) {
        return Err(Error("change_operation_mismatch"));
    }
    let mut backend: Box<dyn Backend> = if change.group() == "dkim" {
        let profile = crate::exchange::profile(r)?;
        let exo = zeroize::Zeroizing::new(
            std::env::var("ZIX_MICROSOFT_EXCHANGE_TOKEN")
                .map_err(|_| Error("exchange_credential_missing"))?,
        );
        Box::new(
            PowerShell::new(profile, exo.to_string(), Some(&token), timeout)
                .map_err(|e| Error(e.0))?,
        )
    } else {
        let password = if apply && matches!(change, Change::CreateUser { .. }) {
            Some(
                std::env::var("ZIX_MICROSOFT_INITIAL_PASSWORD")
                    .map_err(|_| Error("initial_password_missing"))?,
            )
        } else {
            None
        };
        Box::new(
            management::Graph::new(token.to_string(), password, timeout).map_err(|e| Error(e.0))?,
        )
    };
    if inspect {
        return backend.observe(scope, &change).map_err(|e| Error(e.0));
    }
    if let Some(p) = approved {
        let journal =
            Journal::open(Path::new(arg("journal-directory")?)).map_err(|e| Error(e.0))?;
        return management::execute(
            backend.as_mut(),
            &journal,
            &p,
            &zixcel_microsoft_admin::digest(&p).map_err(|e| Error(e.0))?,
            now,
        )
        .map_err(|e| Error(e.0));
    }
    let p =
        management::plan(backend.as_mut(), scope.clone(), change, now()).map_err(|e| Error(e.0))?;
    let envelope = zix_operation_contracts::LocalPlan::new(
        r.operation.replace(".plan", ".apply"),
        crate::catalog::binding()?,
        serde_json::to_value(p).map_err(|_| Error("encoding_failed"))?,
    );
    Ok(json!({"sha256":envelope.digest(),"plan":envelope,"remote_writes":false}))
}
