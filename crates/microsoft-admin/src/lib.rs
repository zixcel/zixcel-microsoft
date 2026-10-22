#![forbid(unsafe_code)]
//! Provider administration, not credential enrollment, a mail server or domain policy.
pub mod exchange;
pub mod graph;
mod scope;
pub use scope::{Scope, domain, email, uuid};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
pub fn digest(value: &impl serde::Serialize) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).map_err(|_| Error("encoding_failed"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
