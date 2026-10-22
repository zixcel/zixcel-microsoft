use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeProfile {
    pub pwsh: PathBuf,
    pub module_manifest: PathBuf,
    pub working_directory: PathBuf,
    pub user_principal_name: String,
}
impl ExchangeProfile {
    pub fn validate(&self) -> Result<()> {
        for p in [&self.pwsh, &self.module_manifest, &self.working_directory] {
            if !p.is_absolute()
                || std::fs::canonicalize(p).map_err(|_| Error("exchange_runtime_unavailable"))?
                    != *p
            {
                return Err(Error("unsafe_exchange_runtime_path"));
            }
        }
        if !self.pwsh.is_file()
            || !self.module_manifest.is_file()
            || !self.working_directory.is_dir()
            || self.module_manifest.file_name()
                != Some(std::ffi::OsStr::new("ExchangeOnlineManagement.psd1"))
        {
            return Err(Error("exchange_runtime_unavailable"));
        }
        use std::os::unix::fs::MetadataExt;
        let uid = std::fs::metadata("/proc/self")
            .map_err(|_| Error("owner_unavailable"))?
            .uid();
        let working = std::fs::metadata(&self.working_directory)
            .map_err(|_| Error("exchange_runtime_unavailable"))?;
        if working.mode() & 0o077 != 0 || working.uid() != uid {
            return Err(Error("unsafe_exchange_working_directory"));
        }
        for path in [&self.pwsh, &self.module_manifest] {
            for parent in path.ancestors() {
                let m =
                    std::fs::metadata(parent).map_err(|_| Error("exchange_runtime_unavailable"))?;
                if m.mode() & 0o022 != 0 || (m.uid() != 0 && m.uid() != uid) {
                    return Err(Error("unsafe_exchange_runtime_permissions"));
                }
            }
        }
        crate::email(&self.user_principal_name)?;
        // This is an owner-supplied trusted module/runtime profile, never LLM request input.
        Ok(())
    }
}
