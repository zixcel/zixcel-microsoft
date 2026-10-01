use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicrosoftAuthenticationMethod {
    AuthenticatorPasskey,
    AuthenticatorPush,
    Fido2SecurityKey,
    Totp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OidcProviderProfile {
    pub schema: String,
    pub profile_id: String,
    pub issuer: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub allowed_methods: Vec<MicrosoftAuthenticationMethod>,
}

impl OidcProviderProfile {
    pub(crate) fn issuer_base(&self) -> &str {
        self.issuer
            .strip_suffix("/v2.0")
            .expect("validated Microsoft issuer")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OidcLoginRequest {
    pub request_id: String,
    pub state: String,
    pub nonce: String,
    pub code_challenge: String,
    pub requested_method: MicrosoftAuthenticationMethod,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OidcLoginPlan {
    pub schema: &'static str,
    pub request_id: String,
    pub issuer: String,
    pub authorization_endpoint: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: &'static str,
    pub code_challenge_method: &'static str,
    pub code_challenge: String,
    pub state: String,
    pub nonce: String,
    pub scopes: Vec<String>,
    pub requested_method: MicrosoftAuthenticationMethod,
    pub network_calls_performed: bool,
    pub credentials_resolved: bool,
}
