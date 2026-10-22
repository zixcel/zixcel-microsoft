use super::{Change, ExchangeProfile, process};
use crate::{Error, Result, Scope};
use crowsi_process_adapter::{Launch, ProcessBudget};

use serde_json::{Value, json};
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
            token: Zeroizing::new(token),
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
        scope.validate()?;
        let script = concat!(
            include_str!("bridge-read.ps1"),
            "\n",
            include_str!("bridge-write.ps1"),
            "\n",
            include_str!("bridge-main.ps1")
        );
        let input=Zeroizing::new(serde_json::to_vec(&json!({"scope":scope,"mode":mode,"change":change,
            "user":self.profile.user_principal_name,"module":self.profile.module_manifest,"token":self.token.as_str()}))
            .map_err(|_|Error("encoding_failed"))?);
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
            .directory(&self.profile.working_directory)
            .budget(ProcessBudget {
                max_memory_bytes: Some(512 * 1024 * 1024),
                max_processes: Some(32),
                cpu_quota: Some(1.0),
                max_file_bytes: Some(8 * 1024 * 1024),
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
