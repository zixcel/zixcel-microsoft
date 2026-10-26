use super::{Change, ExchangeProfile, process};
use crate::{Error, Result, Scope};
use crowsi_process_adapter::{Launch, ProcessBudget};

use serde_json::Value;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;
pub struct PowerShell {
    pub(super) profile: ExchangeProfile,
    token: Zeroizing<String>,
    pub(super) graph: Option<super::super::graph::LiveRead>,
    deadline: Instant,
}
impl PowerShell {
    pub fn new(
        profile: ExchangeProfile,
        token: String,
        graph_token: Option<&str>,
        timeout: Duration,
    ) -> Result<Self> {
        let token = Zeroizing::new(token);
        profile.validate()?;
        if token.is_empty()
            || token.len() > 16384
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._~+/-=".contains(&b))
        {
            return Err(Error("credential_invalid"));
        }
        let graph = graph_token
            .map(|t| {
                crate::graph::LiveRead::new(t, timeout).map_err(|_| Error("graph_transport_failed"))
            })
            .transpose()?;
        Ok(Self {
            profile,
            token,
            graph,
            deadline: Instant::now() + timeout,
        })
    }
    pub(super) fn invoke(
        &self,
        scope: &Scope,
        mode: &str,
        change: Option<&Change>,
    ) -> Result<Value> {
        self.call(scope, mode, change, None)
    }
    fn call(
        &self,
        scope: &Scope,
        mode: &str,
        change: Option<&Change>,
        management: Option<&crate::management::Change>,
    ) -> Result<Value> {
        scope.validate()?;
        let script = concat!(
            include_str!("bridge-read.ps1"),
            "\n",
            include_str!("bridge-write.ps1"),
            "\n",
            include_str!("bridge-management.ps1"),
            "\n",
            include_str!("bridge-main.ps1")
        );
        let input = if management.is_some() {
            super::packet::encode_all(scope, mode, change, management, &self.profile, &self.token)?
        } else {
            super::packet::encode(scope, mode, change, &self.profile, &self.token)?
        };
        let launch = Launch::new("/usr/bin/prlimit")
            .args([
                "--core=0:0",
                "--",
                self.profile
                    .pwsh
                    .to_str()
                    .ok_or(Error("invalid_runtime_path"))?,
            ])
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.profile.working_directory.as_os_str())
            .env("POWERSHELL_TELEMETRY_OPTOUT", "1")
            .env("POWERSHELL_UPDATECHECK", "Off")
            .env("DOTNET_PROCESSOR_COUNT", "1")
            .directory(&self.profile.working_directory)
            .budget(ProcessBudget {
                max_memory_bytes: Some(512 * 1024 * 1024),
                max_processes: Some(32),
                cpu_quota: Some(1.0),
                max_file_bytes: Some(64 * 1024 * 1024),
            });
        let output = Zeroizing::new(process::run(
            launch,
            input,
            self.deadline
                .checked_duration_since(Instant::now())
                .ok_or(Error("exchange_deadline"))?,
        )?);
        if output
            .windows(self.token.len())
            .any(|s| s == self.token.as_bytes())
        {
            return Err(Error("credential_in_output"));
        }
        let response: Value =
            serde_json::from_slice(&output).map_err(|_| Error("invalid_exchange_output"))?;
        if response.get("error").is_some() {
            return Err(Error("exchange_operation_failed"));
        }
        Ok(response)
    }
}

impl crate::management::Backend for PowerShell {
    fn observe(&mut self, scope: &Scope, change: &crate::management::Change) -> Result<Value> {
        change.validate()?;
        self.verify_management_identity(scope)?;
        let v = self.call(scope, "management_snapshot", None, Some(change))?;
        validate_management_state(change, &v)?;
        Ok(v)
    }
    fn mutate(&mut self, scope: &Scope, change: &crate::management::Change) -> Result<()> {
        change.validate()?;
        self.verify_management_identity(scope)?;
        if self.call(scope, "management_apply", None, Some(change))?["acknowledged"] != true {
            return Err(Error("exchange_not_acknowledged"));
        }
        Ok(())
    }
}
impl PowerShell {
    fn verify_management_identity(&mut self, scope: &Scope) -> Result<()> {
        use crate::graph::Transport;
        let g = self
            .graph
            .as_mut()
            .ok_or(Error("graph_credential_missing"))?;
        if g.get("/v1.0/me?$select=id")
            .map_err(|_| Error("subject_unverified"))?["id"]
            != scope.subject
            || g.get(&format!("/v1.0/organization/{}?$select=id", scope.tenant))
                .map_err(|_| Error("tenant_unverified"))?["id"]
                != scope.tenant
        {
            return Err(Error("graph_scope_mismatch"));
        }
        Ok(())
    }
}
fn validate_management_state(c: &crate::management::Change, v: &Value) -> Result<()> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Dkim {
        domain: String,
        enabled: bool,
        selector1: String,
        selector2: String,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct State {
        target: Option<Dkim>,
        eligible: bool,
    }
    let s: State = serde_json::from_value(v.clone()).map_err(|_| Error("invalid_dkim_snapshot"))?;
    let domain = match c {
        crate::management::Change::PrepareDkim { domain }
        | crate::management::Change::EnableDkim { domain } => domain,
        _ => return Err(Error("unsupported_exchange_management_change")),
    };
    if !s.eligible {
        return Err(Error("authoritative_domain_required"));
    }
    if let Some(t) = s.target {
        if &t.domain != domain || t.selector1.len() > 253 || t.selector2.len() > 253 {
            return Err(Error("invalid_dkim_snapshot"));
        }
        for name in [&t.selector1, &t.selector2] {
            if !name.is_empty() {
                crate::domain(name)?;
            }
        }
        let _ = t.enabled;
    }
    Ok(())
}
