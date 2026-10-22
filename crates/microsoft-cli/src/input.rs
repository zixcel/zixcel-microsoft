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
