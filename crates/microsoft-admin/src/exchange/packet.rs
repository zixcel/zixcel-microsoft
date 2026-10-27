use super::{Change, ExchangeProfile};
use crate::{Error, Result, Scope};
use zeroize::Zeroizing;
pub(super) fn encode(
    scope: &Scope,
    mode: &str,
    change: Option<&Change>,
    profile: &ExchangeProfile,
    token: &str,
) -> Result<Zeroizing<Vec<u8>>> {
    encode_all(scope, mode, change, None, None, None, profile, token)
}
// One bounded wire packet keeps each existing operation payload distinct.
#[allow(clippy::too_many_arguments)]
pub(super) fn encode_all(
    scope: &Scope,
    mode: &str,
    change: Option<&Change>,
    management: Option<&crate::management::Change>,
    audit_mailbox: Option<&str>,
    configuration: Option<&serde_json::Value>,
    profile: &ExchangeProfile,
    token: &str,
) -> Result<Zeroizing<Vec<u8>>> {
    #[derive(serde::Serialize)]
    struct Input<'a> {
        scope: &'a Scope,
        mode: &'a str,
        change: Option<&'a Change>,
        user: &'a str,
        audit_mailbox: Option<&'a str>,
        management_change: Option<&'a crate::management::Change>,
        configuration: Option<&'a serde_json::Value>,
        module: &'a std::path::Path,
        token: &'a str,
    }
    let input = Zeroizing::new(
        serde_json::to_vec(&Input {
            scope,
            mode,
            audit_mailbox,
            management_change: management,
            configuration,
            change,
            user: &profile.user_principal_name,
            module: &profile.module_manifest,
            token,
        })
        .map_err(|_| Error("encoding_failed"))?,
    );
    if input.len() > 65536 {
        return Err(Error("exchange_input_limit"));
    }
    Ok(input)
}
