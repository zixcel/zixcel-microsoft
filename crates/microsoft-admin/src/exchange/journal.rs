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
    pub fn completed(self) -> Result<()> {
        let mut f = OpenOptions::new()
            .append(true)
            .open(&self.marker)
            .map_err(|_| Error("journal_write_failed"))?;
        f.write_all(b"verified by independent post-read\n")
            .and_then(|_| f.sync_all())
            .map_err(|_| Error("journal_write_failed"))?;
        fs::remove_file(&self.lock).map_err(|_| Error("lock_cleanup_failed"))
    }
}
