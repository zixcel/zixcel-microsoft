use serde::de::DeserializeOwned;
use std::{io::Read, path::Path};
use zix_native_provider::Error;
pub fn read<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let path = Path::new(path);
    let m = std::fs::symlink_metadata(path).map_err(|_| Error("input_unavailable"))?;
    use std::os::unix::fs::MetadataExt;
    if !path.is_absolute()
        || !m.is_file()
        || m.mode() & 0o077 != 0
        || m.len() > 131072
        || m.uid()
            != std::fs::metadata("/proc/self")
                .map_err(|_| Error("owner_unavailable"))?
                .uid()
        || std::fs::canonicalize(path).map_err(|_| Error("input_unavailable"))? != path
    {
        return Err(Error("unsafe_input_file"));
    }
    let mut b = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(131073).read_to_end(&mut b))
        .map_err(|_| Error("input_unavailable"))?;
    if b.len() > 131072 {
        return Err(Error("input_limit"));
    }
    serde_json::from_slice(&b).map_err(|_| Error("invalid_input_json"))
}

pub fn write_private_new(path: &str, value: &impl serde::Serialize) -> Result<(), Error> {
    use std::{
        io::Write,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let path = Path::new(path);
    zix_cli::files::parents(path).map_err(|e| Error(e.0))?;
    let parent = path.parent().ok_or(Error("unsafe_configuration_path"))?;
    if std::fs::canonicalize(parent).map_err(|_| Error("configuration_directory_unavailable"))?
        != parent
    {
        return Err(Error("unsafe_configuration_path"));
    }
    let m = std::fs::metadata(parent).map_err(|_| Error("configuration_directory_unavailable"))?;
    let uid = std::fs::metadata("/proc/self")
        .map_err(|_| Error("owner_unavailable"))?
        .uid();
    if m.uid() != uid || m.mode() & 0o077 != 0 {
        return Err(Error("unsafe_configuration_directory"));
    }
    let b = serde_json::to_vec_pretty(value).map_err(|_| Error("encoding_failed"))?;
    if b.len() > 131072 {
        return Err(Error("configuration_limit"));
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| Error("configuration_exists_or_unavailable"))?;
    f.write_all(&b)
        .and_then(|_| f.sync_all())
        .map_err(|_| Error("configuration_commit_unknown"))?;
    std::fs::File::open(parent)
        .and_then(|d| d.sync_all())
        .map_err(|_| Error("configuration_commit_unknown"))
}
