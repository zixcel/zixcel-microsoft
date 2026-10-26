use crate::{Error, Result, Scope};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
pub struct Journal {
    root: PathBuf,
}
impl Journal {
    /// Existing private, caller-provisioned directory. No automatic credential/config setup.
    pub fn open(root: &Path) -> Result<Self> {
        if !root.is_absolute()
            || fs::canonicalize(root).map_err(|_| Error("journal_unavailable"))? != root
        {
            return Err(Error("unsafe_journal_path"));
        }
        let m = fs::symlink_metadata(root).map_err(|_| Error("journal_unavailable"))?;
        #[cfg(unix)]
        if m.mode() & 0o077 != 0
            || m.uid()
                != fs::metadata("/proc/self")
                    .map_err(|_| Error("owner_unavailable"))?
                    .uid()
        {
            return Err(Error("unsafe_journal_permissions"));
        }
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(Error("unsafe_journal_path"));
        }
        Ok(Self { root: root.into() })
    }
    pub(crate) fn admit(&self, hash: &str, scope: &Scope) -> Result<Admission> {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error("invalid_approval_hash"));
        }
        // Permanent per-plan marker survives crashes; tenant lock prevents concurrent local changes.
        let marker = self.root.join(format!("{hash}.attempt"));
        let mut f = create(&marker).map_err(|_| Error("attempt_already_exists_or_unavailable"))?;
        f.write_all(b"admitted; outcome unknown until independent readback\n")
            .and_then(|_| f.sync_all())
            .map_err(|_| Error("journal_write_failed"))?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error("journal_sync_failed"))?;
        let lock = self.root.join(format!("{}.active", scope.tenant));
        let mut f =
            create(&lock).map_err(|_| Error("tenant_locked_manual_reconciliation_required"))?;
        f.write_all(hash.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|_| Error("journal_write_failed"))?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error("journal_sync_failed"))?;
        Ok(Admission { lock, marker })
    }
}
fn create(path: &Path) -> std::io::Result<File> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    o.mode(0o600);
    o.open(path)
}
pub(crate) struct Admission {
    lock: PathBuf,
    marker: PathBuf,
}
impl Admission {
    pub(crate) fn record_failure(
        &self,
        failure: &crate::management::MutationFailure,
    ) -> Result<()> {
        let mut bytes = serde_json::to_vec(&serde_json::json!({"failure":failure}))
            .map_err(|_| Error("encoding_failed"))?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.marker)
            .map_err(|_| Error("journal_write_failed"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error("journal_write_failed"))
    }
    pub fn completed(self) -> Result<()> {
        let mut f = OpenOptions::new()
            .append(true)
            .open(&self.marker)
            .map_err(|_| Error("journal_write_failed"))?;
        f.write_all(b"verified by independent post-read\n")
            .and_then(|_| f.sync_all())
            .map_err(|_| Error("journal_write_failed"))?;
        fs::remove_file(&self.lock).map_err(|_| Error("lock_cleanup_failed"))?;
        File::open(self.lock.parent().ok_or(Error("journal_unavailable"))?)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error("journal_sync_failed"))
    }
}

impl Journal {
    pub(crate) fn directory(&self) -> &Path {
        &self.root
    }
    pub(crate) fn uncertain_evidence(&self, hash: &str, scope: &Scope) -> Result<String> {
        scope.validate()?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error("invalid_approval_hash"));
        }
        let lock = private_bytes(&self.root.join(format!("{}.active", scope.tenant)))?;
        if lock != hash.as_bytes() {
            return Err(Error("active_attempt_mismatch"));
        }
        let marker = private_bytes(&self.root.join(format!("{hash}.attempt")))?;
        if !marker.starts_with(b"admitted; outcome unknown until independent readback\n")
            || marker
                .windows(b"verified by independent post-read".len())
                .any(|v| v == b"verified by independent post-read")
        {
            return Err(Error("attempt_not_uncertain"));
        }
        use sha2::{Digest, Sha256};
        Ok(Sha256::digest(&marker)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
    pub(crate) fn resolve_uncertain(
        &self,
        hash: &str,
        expected: &str,
        scope: &Scope,
        receipt: &serde_json::Value,
    ) -> Result<()> {
        if self.uncertain_evidence(hash, scope)? != expected {
            return Err(Error("journal_state_drift"));
        }
        // Reserve once before touching the lock. Failures leave conservative evidence.
        let path = self.root.join(format!("{hash}.resolution"));
        let mut file =
            create(&path).map_err(|_| Error("resolution_already_exists_or_unavailable"))?;
        let bytes = serde_json::to_vec(receipt).map_err(|_| Error("encoding_failed"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error("journal_write_failed"))?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error("journal_sync_failed"))?;
        if self.uncertain_evidence(hash, scope)? != expected {
            return Err(Error("journal_state_drift"));
        }
        let archived = self.root.join(format!("{hash}.resolved-lock"));
        if archived.exists() {
            return Err(Error("resolution_archive_exists"));
        }
        fs::rename(self.root.join(format!("{}.active", scope.tenant)), archived)
            .map_err(|_| Error("lock_archive_failed"))?;
        File::open(&self.root)
            .and_then(|d| d.sync_all())
            .map_err(|_| Error("journal_sync_failed"))
    }
}
fn private_bytes(path: &Path) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(path).map_err(|_| Error("journal_unavailable"))?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > 16384 {
        return Err(Error("unsafe_journal_file"));
    }
    #[cfg(unix)]
    if m.mode() & 0o077 != 0
        || m.uid()
            != fs::metadata("/proc/self")
                .map_err(|_| Error("owner_unavailable"))?
                .uid()
    {
        return Err(Error("unsafe_journal_permissions"));
    }
    fs::read(path).map_err(|_| Error("journal_unavailable"))
}
