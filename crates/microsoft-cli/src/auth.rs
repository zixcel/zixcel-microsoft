//! Device authorization and refresh-token custody. No token is a protocol result.
#[cfg(feature = "credential-store")]
mod flow;
#[cfg(feature = "credential-store")]
mod profile;
#[cfg(feature = "credential-store")]
mod storage;
use serde_json::Value;
use zix_native_provider::{Error, Request};
pub fn invoke(r: &Request) -> Result<Value, Error> {
    #[cfg(feature = "credential-store")]
    {
        flow::invoke(r)
    }
    #[cfg(not(feature = "credential-store"))]
    {
        let _ = r;
        Err(Error("credential_store_not_in_build"))
    }
}
