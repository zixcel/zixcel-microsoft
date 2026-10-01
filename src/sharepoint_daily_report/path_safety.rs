use super::DailyReportError;

const MAX_PATH_BYTES: usize = 512;
const MAX_SEGMENT_BYTES: usize = 128;

/// Enforces a portable drive-root-relative JSON path with no traversal.
pub(crate) fn drive_relative_path(value: &str) -> Result<(), DailyReportError> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.starts_with(['/', '\\'])
        || value.ends_with(['/', '\\'])
        || value.contains('\\')
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    ':' | '"' | '*' | '<' | '>' | '?' | '|' | '#' | '%'
                )
        })
    {
        return Err(DailyReportError::new(
            "drive_root_relative_path",
            "must be a safe drive-root-relative JSON path",
        ));
    }
    for segment in value.split('/') {
        if segment.is_empty()
            || matches!(segment, "." | "..")
            || segment.trim() != segment
            || segment.len() > MAX_SEGMENT_BYTES
        {
            return Err(DailyReportError::new(
                "drive_root_relative_path",
                "contains an invalid path segment",
            ));
        }
    }
    if !value.ends_with(".json") {
        return Err(DailyReportError::new(
            "drive_root_relative_path",
            "must identify a .json artifact",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::drive_relative_path;

    #[test]
    fn path_policy_rejects_roots_traversal_and_non_json_files() {
        for path in [
            "/report.json",
            "../report.json",
            r"reports\one.json",
            "one.txt",
        ] {
            assert!(drive_relative_path(path).is_err(), "accepted {path}");
        }
    }
}
