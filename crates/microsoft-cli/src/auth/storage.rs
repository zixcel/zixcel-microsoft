use super::profile::Profile;
use crowsi_credential_broker::{
    BrokerError, CredentialEntry, CredentialStore, LinuxCustodyStore, SecretValue,
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};
use zix_native_provider::Error;
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cache {
    pub profile: String,
    pub resource: String,
    pub subject: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub device_code: String,
    pub expires_at: u64,
    pub interval: u64,
    pub next_poll: u64,
}
impl Drop for Cache {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
        self.device_code.zeroize();
    }
}
pub(super) fn error(e: BrokerError) -> Error {
    Error(match e {
        BrokerError::NotFound => "auth_not_registered",
        BrokerError::BackendDenied => "credential_store_locked_or_denied",
        BrokerError::BackendUnavailable => "credential_store_unavailable",
        _ => "credential_store_operation_failed",
    })
}
pub(super) fn connect(p: &Profile) -> Result<LinuxCustodyStore, Error> {
    if let Some(a) = std::env::var_os("DBUS_SESSION_BUS_ADDRESS") {
        let a = a.to_str().ok_or(Error("invalid_runtime_environment"))?;
        if !a.starts_with("unix:path=/")
            || a.len() > 4096
            || a.contains(';')
            || a.bytes().any(|b| b.is_ascii_control())
        {
            return Err(Error("invalid_runtime_environment"));
        }
    }
    LinuxCustodyStore::connect(&p.collection).map_err(error)
}
pub(super) fn read(
    store: &impl CredentialStore,
    p: &Profile,
    resource: &str,
    device: bool,
) -> Result<Cache, Error> {
    let reference = p.reference(resource, device)?;
    let m = store.metadata(&reference).map_err(error)?;
    let entry = store
        .get(&reference, m.revision().as_str())
        .map_err(error)?;
    let c: Cache = entry
        .into_secret()
        .expose(|b| serde_json::from_slice(b).map_err(|_| Error("invalid_auth_cache")))?;
    if c.profile != p.hash()? || c.resource != resource {
        return Err(Error("auth_cache_binding_mismatch"));
    }
    Ok(c)
}
pub(super) fn put(
    store: &impl CredentialStore,
    p: &Profile,
    resource: &str,
    device: bool,
    c: &Cache,
) -> Result<(), Error> {
    let bytes =
        Zeroizing::new(serde_json::to_vec(c).map_err(|_| Error("auth_cache_encoding_failed"))?);
    if bytes.len() > 65536 {
        return Err(Error("auth_cache_limit"));
    }
    let hosts = if device {
        vec!["login.microsoftonline.com".into()]
    } else {
        vec![
            if resource == "graph" {
                "graph.microsoft.com"
            } else {
                "outlook.office365.com"
            }
            .into(),
            "login.microsoftonline.com".into(),
        ]
    };
    let entry = CredentialEntry::new(
        p.reference(resource, device)?,
        hosts,
        SecretValue::new(bytes.to_vec()).map_err(error)?,
    )
    .map_err(error)?;
    store.put(entry).map_err(error)
}
pub(super) fn delete(
    store: &impl CredentialStore,
    p: &Profile,
    resource: &str,
    device: bool,
) -> Result<(), Error> {
    match store.delete(&p.reference(resource, device)?) {
        Ok(()) | Err(BrokerError::NotFound) => Ok(()),
        Err(e) => Err(error(e)),
    }
}
pub(super) struct Lock(std::fs::File);
impl Lock {
    pub fn acquire(profile: &str) -> Result<Self, Error> {
        use std::{
            fs::OpenOptions,
            os::unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let path = std::path::Path::new(profile).with_extension("auth.lock");
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error("unsafe_auth_lock"));
        }
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|_| Error("auth_lock_unavailable"))?;
        let m = f.metadata().map_err(|_| Error("auth_lock_unavailable"))?;
        let uid = std::fs::metadata("/proc/self")
            .map_err(|_| Error("owner_unavailable"))?
            .uid();
        if !m.is_file() || m.uid() != uid || m.mode() & 0o077 != 0 {
            return Err(Error("unsafe_auth_lock"));
        }
        f.try_lock()
            .map_err(|_| Error("auth_operation_in_progress"))?;
        Ok(Self(f))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
