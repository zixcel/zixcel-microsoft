use zixcel_microsoft::{
    MicrosoftAuthenticationMethod, OidcLoginRequest, OidcProviderProfile, build_oidc_login_plan,
};

fn profile() -> OidcProviderProfile {
    OidcProviderProfile {
        schema: "zixcel://microsoft/oidc-provider-profile/v1".into(),
        profile_id: "ihat-entra".into(),
        issuer: "https://login.microsoftonline.com/11111111-2222-4333-8444-555555555555/v2.0"
            .into(),
        client_id: "00000000-0000-4000-8000-000000000001".into(),
        redirect_uri: "http://127.0.0.1:43119/auth/callback".into(),
        scopes: vec!["openid".into(), "profile".into(), "email".into()],
        allowed_methods: vec![
            MicrosoftAuthenticationMethod::AuthenticatorPasskey,
            MicrosoftAuthenticationMethod::Fido2SecurityKey,
        ],
    }
}

fn request() -> OidcLoginRequest {
    OidcLoginRequest {
        request_id: "login-request-001".into(),
        state: "state_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH".into(),
        nonce: "nonce_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH".into(),
        code_challenge: "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHijk".into(),
        requested_method: MicrosoftAuthenticationMethod::AuthenticatorPasskey,
    }
}

#[test]
fn creates_a_closed_network_free_pkce_plan() {
    let plan = build_oidc_login_plan(&profile(), &request()).expect("plan");
    assert_eq!(plan.code_challenge_method, "S256");
    assert_eq!(plan.response_type, "code");
    assert_eq!(plan.scopes, ["email", "openid", "profile"]);
    assert!(!plan.network_calls_performed);
    assert!(!plan.credentials_resolved);
}

#[test]
fn rejects_open_redirects_secrets_and_unapproved_methods() {
    let mut invalid = profile();
    invalid.redirect_uri = "https://example.com/callback".into();
    assert!(build_oidc_login_plan(&invalid, &request()).is_err());

    let mut invalid = profile();
    invalid.client_id = "secret://client/value".into();
    assert!(build_oidc_login_plan(&invalid, &request()).is_err());

    let mut invalid_request = request();
    invalid_request.requested_method = MicrosoftAuthenticationMethod::Totp;
    assert!(build_oidc_login_plan(&profile(), &invalid_request).is_err());
}

#[test]
fn rejects_replay_weak_material_and_graph_scopes() {
    let mut invalid_request = request();
    invalid_request.state = "short".into();
    assert!(build_oidc_login_plan(&profile(), &invalid_request).is_err());

    let mut invalid_request = request();
    invalid_request.nonce = invalid_request.state.clone();
    assert!(build_oidc_login_plan(&profile(), &invalid_request).is_err());

    let mut invalid = profile();
    invalid
        .scopes
        .push("https://graph.microsoft.com/.default".into());
    assert!(build_oidc_login_plan(&invalid, &request()).is_err());

    let mut invalid = profile();
    invalid.scopes.push("openid".into());
    assert!(build_oidc_login_plan(&invalid, &request()).is_err());
}

#[test]
fn rejects_multi_tenant_issuer_aliases() {
    for tenant in [
        "common",
        "organizations",
        "consumers",
        "example.onmicrosoft.com",
    ] {
        let mut invalid = profile();
        invalid.issuer = format!("https://login.microsoftonline.com/{tenant}/v2.0");
        assert!(
            build_oidc_login_plan(&invalid, &request()).is_err(),
            "{tenant}"
        );
    }
}
