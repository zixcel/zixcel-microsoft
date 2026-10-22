//! Observed SMTP dot-atoms may contain RFC punctuation; write inputs remain stricter.
use crate::{Error, Result};
pub(super) fn validate(value: &str) -> Result<()> {
    let (local, host) = value.split_once('@').ok_or(Error("invalid_address"))?;
    crate::domain(host)?;
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b".!#$%&'*+-/=?^_`{|}~".contains(&b)
        })
    {
        return Err(Error("invalid_address"));
    }
    Ok(())
}
