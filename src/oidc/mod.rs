mod model;
mod validation;

pub use model::{
    MicrosoftAuthenticationMethod, OidcLoginPlan, OidcLoginRequest, OidcProviderProfile,
};

use crate::ConnectorError;
use validation::{validate_profile, validate_request};

/// Builds inert authorization parameters for a separately authorized host.
pub fn build_oidc_login_plan(
    profile: &OidcProviderProfile,
    request: &OidcLoginRequest,
) -> Result<OidcLoginPlan, ConnectorError> {
    validate_profile(profile)?;
    validate_request(profile, request)?;
    let mut scopes = profile.scopes.clone();
    scopes.sort();
    scopes.dedup();
    Ok(OidcLoginPlan {
        schema: "zixcel://microsoft/oidc-login-plan/v1",
        request_id: request.request_id.clone(),
        issuer: profile.issuer.clone(),
        authorization_endpoint: format!("{}/oauth2/v2.0/authorize", profile.issuer_base()),
        client_id: profile.client_id.clone(),
        redirect_uri: profile.redirect_uri.clone(),
        response_type: "code",
        code_challenge_method: "S256",
        code_challenge: request.code_challenge.clone(),
        state: request.state.clone(),
        nonce: request.nonce.clone(),
        scopes,
        requested_method: request.requested_method.clone(),
        network_calls_performed: false,
        credentials_resolved: false,
    })
}
