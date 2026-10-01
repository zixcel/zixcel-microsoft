use super::{OidcLoginRequest, OidcProviderProfile};
use crate::{ConnectorError, boundary::identifier};

const PROFILE_SCHEMA: &str = "zixcel://microsoft/oidc-provider-profile/v1";
const ALLOWED_SCOPES: [&str; 3] = ["openid", "profile", "email"];

pub(super) fn validate_profile(value: &OidcProviderProfile) -> Result<(), ConnectorError> {
    if value.schema != PROFILE_SCHEMA {
        return Err(error("schema", "unsupported OIDC profile schema"));
    }
    identifier("profile_id", &value.profile_id)?;
    validate_issuer(&value.issuer)?;
    validate_public_client_id(&value.client_id)?;
    validate_loopback(&value.redirect_uri)?;
    if value.scopes.is_empty()
        || value.scopes.len() > ALLOWED_SCOPES.len()
        || has_duplicates(&value.scopes)
        || !value
            .scopes
            .iter()
            .all(|scope| ALLOWED_SCOPES.contains(&scope.as_str()))
        || !value.scopes.iter().any(|scope| scope == "openid")
    {
        return Err(error("scopes", "only bounded sign-in scopes are allowed"));
    }
    if value.allowed_methods.is_empty()
        || value.allowed_methods.len() > 4
        || has_duplicates(&value.allowed_methods)
    {
        return Err(error("allowed_methods", "must contain 1..=4 methods"));
    }
    Ok(())
}

pub(super) fn validate_request(
    profile: &OidcProviderProfile,
    value: &OidcLoginRequest,
) -> Result<(), ConnectorError> {
    identifier("request_id", &value.request_id)?;
    bounded_random("state", &value.state)?;
    bounded_random("nonce", &value.nonce)?;
    if value.state == value.nonce {
        return Err(error("nonce", "must be independent from state"));
    }
    pkce(&value.code_challenge)?;
    if !profile.allowed_methods.contains(&value.requested_method) {
        return Err(error(
            "requested_method",
            "authentication method is not allowed",
        ));
    }
    Ok(())
}

fn validate_issuer(value: &str) -> Result<(), ConnectorError> {
    let tenant = value
        .strip_prefix("https://login.microsoftonline.com/")
        .and_then(|rest| rest.strip_suffix("/v2.0"));
    if !tenant.is_some_and(valid_uuid) {
        return Err(error(
            "issuer",
            "must contain one pinned Microsoft tenant UUID",
        ));
    }
    Ok(())
}

fn validate_public_client_id(value: &str) -> Result<(), ConnectorError> {
    valid_uuid(value)
        .then_some(())
        .ok_or_else(|| error("client_id", "must be a public app UUID"))
}

fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23) && byte == b'-'
                || !matches!(index, 8 | 13 | 18 | 23) && byte.is_ascii_hexdigit()
        })
}

fn validate_loopback(value: &str) -> Result<(), ConnectorError> {
    let port = value
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix("/auth/callback"))
        .and_then(|part| part.parse::<u16>().ok());
    if !port.is_some_and(|value| value >= 1024) {
        return Err(error(
            "redirect_uri",
            "must be a high-port IPv4 loopback callback",
        ));
    }
    Ok(())
}

fn bounded_random(field: &'static str, value: &str) -> Result<(), ConnectorError> {
    if !(43..=128).contains(&value.len()) || !value.bytes().all(base64url) {
        return Err(error(field, "must be 43..=128 base64url characters"));
    }
    Ok(())
}

fn pkce(value: &str) -> Result<(), ConnectorError> {
    bounded_random("code_challenge", value)
}

fn base64url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

fn has_duplicates<T: Eq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
}

fn error(field: &'static str, message: &'static str) -> ConnectorError {
    ConnectorError::new(field, message)
}
