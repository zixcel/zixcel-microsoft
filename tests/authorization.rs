use zixcel_microsoft::{build_authorized_plan, parse_config, parse_provider_plan_request};

const CONFIG: &str = include_str!("../examples/config.toml");
const REQUEST: &str = include_str!("../examples/provider-plan-request.json");

#[test]
fn authorized_plan_preserves_the_upstream_evidence_chain() {
    let config = parse_config(CONFIG).expect("fixture config");
    let request = parse_provider_plan_request(REQUEST).expect("fixture request");
    let plan = build_authorized_plan(&config, &request).expect("authorized plan");

    assert_eq!(plan.provider, "microsoft");
    assert_eq!(
        plan.workflow_receipt_id,
        request.workflow_authorization.receipt_id
    );
    assert_eq!(plan.decision_id, request.workflow_authorization.decision_id);
    assert!(!plan.network_calls_performed);
    assert!(!plan.credentials_resolved);
}

#[test]
fn one_character_receipt_tamper_is_rejected() {
    let config = parse_config(CONFIG).expect("fixture config");
    let source = REQUEST.replacen("b1d1886e", "a1d1886e", 1);
    let request = parse_provider_plan_request(&source).expect("closed request");

    assert!(build_authorized_plan(&config, &request).is_err());
}

#[test]
fn one_character_decision_tamper_is_rejected() {
    let config = parse_config(CONFIG).expect("fixture config");
    let source = REQUEST.replacen("bounded-pilot", "bounded-pilou", 1);
    let request = parse_provider_plan_request(&source).expect("closed request");

    assert!(build_authorized_plan(&config, &request).is_err());
}

#[test]
fn unsafe_or_extended_requests_are_rejected() {
    let unsafe_source =
        REQUEST.replacen("\"network_access\": false", "\"network_access\": true", 1);
    let extended = REQUEST.replacen(
        "\"external_actions\": false",
        "\"external_actions\": false, \"unexpected\": true",
        1,
    );

    let unsafe_request = parse_provider_plan_request(&unsafe_source).expect("closed request");
    assert!(
        build_authorized_plan(&parse_config(CONFIG).expect("config"), &unsafe_request).is_err()
    );
    assert!(parse_provider_plan_request(&extended).is_err());
}
