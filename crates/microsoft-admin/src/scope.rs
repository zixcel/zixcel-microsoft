use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub tenant: String,
    pub subject: String,
}
impl Scope {
    pub fn validate(&self) -> Result<()> {
        uuid(&self.tenant)?;
        uuid(&self.subject)
    }
}
pub fn uuid(value: &str) -> Result<()> {
    if value.len() != 36
        || value.bytes().enumerate().any(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b != b'-'
            } else {
                !b.is_ascii_hexdigit() || b.is_ascii_uppercase()
            }
        })
        || value == "00000000-0000-0000-0000-000000000000"
    {
        return Err(Error("invalid_uuid"));
    }
    Ok(())
}
pub fn domain(value: &str) -> Result<()> {
    if value.len() > 253
        || !value.contains('.')
        || value.split('.').any(|s| {
            s.is_empty()
                || s.len() > 63
                || s.starts_with('-')
                || s.ends_with('-')
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
    {
        return Err(Error("invalid_domain"));
    }
    Ok(())
}
pub fn email(value: &str) -> Result<()> {
    let (local, host) = value.split_once('@').ok_or(Error("invalid_address"))?;
    domain(host)?;
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._+-".contains(&b))
    {
        return Err(Error("invalid_address"));
    }
    Ok(())
}
