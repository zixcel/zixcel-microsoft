use crate::Result;
use serde::Deserialize;
use zeroize::{Zeroize, Zeroizing};
pub const MAX_TOKEN_RESPONSE: usize = 16384;
pub struct AccessToken {
    secret: Zeroizing<String>,
    lifetime_seconds: u64,
}
impl AccessToken {
    pub fn secret(&self) -> &str {
        &self.secret
    }
    pub fn lifetime_seconds(&self) -> u64 {
        self.lifetime_seconds
    }
}
#[derive(Deserialize)]
struct Response {
    access_token: String,
    token_type: String,
    expires_in: u64,
}
impl Drop for Response {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
pub fn parse_token(status: u16, bytes: &[u8]) -> Result<AccessToken> {
    if status != 200 || bytes.len() > MAX_TOKEN_RESPONSE {
        return Err("graph_token_unavailable");
    }
    let mut v: Response =
        serde_json::from_slice(bytes).map_err(|_| "invalid_graph_token_response")?;
    if !v.token_type.eq_ignore_ascii_case("Bearer")
        || !(60..=86400).contains(&v.expires_in)
        || v.access_token.is_empty()
        || v.access_token.len() > 12288
        || !v
            .access_token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._~+/-=".contains(&b))
    {
        return Err("invalid_graph_token_response");
    }
    Ok(AccessToken {
        secret: Zeroizing::new(std::mem::take(&mut v.access_token)),
        lifetime_seconds: v.expires_in,
    })
}
