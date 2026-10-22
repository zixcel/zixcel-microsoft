use crate::{Error, Result};
pub(super) fn continuation(next: &str, initial: &str) -> Result<String> {
    let u = url::Url::parse(next).map_err(|_| Error("invalid_continuation"))?;
    let base = initial.split('?').next().ok_or(Error("invalid_path"))?;
    if next.len() > 4096
        || u.origin().ascii_serialization() != "https://graph.microsoft.com"
        || u.path() != base
        || !u.username().is_empty()
        || u.password().is_some()
        || u.fragment().is_some()
        || u.query_pairs()
            .any(|(k, _)| !["$select", "$top", "$skiptoken", "$skip"].contains(&k.as_ref()))
    {
        return Err(Error("unsafe_continuation"));
    }
    let selected = initial.split_once("$select=").map(|(_, v)| v).unwrap_or("");
    for (key, value) in u.query_pairs() {
        if key == "$select" && value != selected {
            return Err(Error("unsafe_continuation"));
        }
        if key == "$top" && value.parse::<u16>().ok().is_none_or(|n| n == 0 || n > 100) {
            return Err(Error("unsafe_continuation"));
        }
    }
    Ok(format!(
        "{}?{}",
        u.path(),
        u.query().ok_or(Error("invalid_continuation"))?
    ))
}
