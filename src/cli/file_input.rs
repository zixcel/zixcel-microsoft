use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub(super) fn read_bounded_utf8(path: &str, max_bytes: u64, label: &str) -> Result<String, String> {
    let bytes = read_bounded_file(path, max_bytes, label)?;
    String::from_utf8(bytes).map_err(|_| format!("{label} must be UTF-8"))
}

/// Refuses links and special files so local acceptance has one unambiguous source.
pub(super) fn read_bounded_file(
    path: &str,
    max_bytes: u64,
    label: &str,
) -> Result<Vec<u8>, String> {
    let path = Path::new(path);
    let metadata =
        fs::symlink_metadata(path).map_err(|_| format!("{label} file could not be inspected"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!("{label} must be a regular non-symlink file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("{label} exceeds the configured byte boundary"));
    }
    let file = File::open(path).map_err(|_| format!("{label} file could not be opened"))?;
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| format!("{label} file could not be read"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(format!("{label} exceeds the configured byte boundary"));
    }
    Ok(bytes)
}
