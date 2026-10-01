use serde::Serialize;
use sha2::{Digest, Sha256};

use super::DailyReportError;

pub(crate) fn stable_id(prefix: &str, value: &impl Serialize) -> Result<String, DailyReportError> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        DailyReportError::new(
            "identifier",
            "could not serialize deterministic identifier seed",
        )
    })?;
    Ok(format!("{prefix}-{}", &sha256_hex(&bytes)[..16]))
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn digest_uses_lowercase_sha256_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
