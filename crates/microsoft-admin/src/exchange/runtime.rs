use super::{ExchangeProfile, process};
use crate::{Error, Result};
use crowsi_process_adapter::{Launch, ProcessBudget};
use serde_json::Value;
use std::time::Duration;
use zeroize::Zeroizing;
pub fn inspect(profile: &ExchangeProfile, timeout: Duration) -> Result<Value> {
    profile.validate()?;
    let script = r#"$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; $InformationPreference='SilentlyContinue';
try {
 $text=[Console]::In.ReadToEnd(); if ($text.Length -gt 8192) {throw 'input_limit'}
 $r=ConvertFrom-Json -InputObject $text
 Import-Module -Name $r.module -RequiredVersion '3.10.1' -ErrorAction Stop
 $module=@(Get-Module ExchangeOnlineManagement)
 if ($module.Count -ne 1) {throw 'module_identity_mismatch'}
 $commands=@('Connect-ExchangeOnline','Disconnect-ExchangeOnline','Get-ConnectionInformation','Get-EXOMailbox')
 foreach ($name in $commands) {if (!(Get-Command $name -ErrorAction SilentlyContinue)) {throw 'command_unavailable'}}
 @{powershell_version=$PSVersionTable.PSVersion.ToString();module_version=$module[0].Version.ToString();authentication='not_attempted';containment='enforced';backend='crowsi-systemd-user-service'} | ConvertTo-Json -Compress
} catch {[Console]::Out.WriteLine('{"error":"runtime_probe_failed"}');exit 2}"#;
    let launch = Launch::new("/usr/bin/prlimit")
        .args([
            "--core=0:0",
            "--",
            profile.pwsh.to_str().ok_or(Error("invalid_runtime_path"))?,
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", profile.working_directory.as_os_str())
        .env("POWERSHELL_TELEMETRY_OPTOUT", "1")
        .env("POWERSHELL_UPDATECHECK", "Off")
        .env("DOTNET_PROCESSOR_COUNT", "1")
        .directory(&profile.working_directory)
        .budget(ProcessBudget {
            max_memory_bytes: Some(512 * 1024 * 1024),
            max_processes: Some(32),
            cpu_quota: Some(1.0),
            max_file_bytes: Some(64 * 1024 * 1024),
        });
    let request = Zeroizing::new(
        serde_json::to_vec(&serde_json::json!({"module":profile.module_manifest}))
            .map_err(|_| Error("encoding_failed"))?,
    );
    let bytes = process::run(launch, request, timeout)?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| Error("invalid_runtime_probe"))?;
    if value.get("error").is_some() || value["module_version"] != "3.10.1" {
        return Err(Error("runtime_probe_failed"));
    }
    Ok(value)
}
