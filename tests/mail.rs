use zixcel_microsoft::mail::{MailContent, MailRequest, build_mail_plan, parse_mail_request};

fn request() -> MailRequest {
    parse_mail_request(include_str!("../examples/mail-request.json")).unwrap()
}

#[test]
fn multiple_accounts_read_boundaries_and_exact_request_keys() {
    let value = request();
    let plan = build_mail_plan(&value).unwrap();
    assert_eq!(plan, build_mail_plan(&value).unwrap());
    assert_eq!(plan.method, "GET");
    assert_eq!(plan.required_delegated_permission, "Mail.ReadBasic");
    assert!(!plan.url.contains("body"));
    assert!(plan.prefer.contains("ImmutableId"));
    assert_eq!(plan.max_response_bytes, value.max_response_bytes);
    assert!(!plan.network_calls_performed);
    assert!(!plan.follow_redirects);
    let mut body = value.clone();
    body.content = MailContent::Body;
    let body_plan = build_mail_plan(&body).unwrap();
    assert_eq!(body_plan.required_delegated_permission, "Mail.Read");
    assert!(body_plan.url.ends_with(",body"));
    assert_ne!(body_plan.plan_id, plan.plan_id);
    let mut other = value.clone();
    other.account_ref = "account:second".into();
    assert_ne!(build_mail_plan(&other).unwrap().plan_id, plan.plan_id);
    let mut escaped = value;
    escaped.folder_id = "A/B+=?".into();
    assert!(
        build_mail_plan(&escaped)
            .unwrap()
            .url
            .contains("A%2FB%2B%3D%3F/messages?")
    );
}

#[test]
fn rejects_embedded_credentials_unknown_fields_and_resource_excess() {
    let source = include_str!("../examples/mail-request.json");
    let mut json: serde_json::Value = serde_json::from_str(source).unwrap();
    json["token"] = "not-a-real-token".into();
    assert!(parse_mail_request(&json.to_string()).is_err());
    for maximum in [0, 101] {
        let mut value = request();
        value.maximum_messages = maximum;
        assert!(build_mail_plan(&value).is_err());
    }
    let mut value = request();
    value.max_response_bytes = 1_048_577;
    assert!(build_mail_plan(&value).is_err());
    value = request();
    value.folder_id = "..".into();
    assert!(build_mail_plan(&value).is_err());
    assert!(parse_mail_request(&" ".repeat(16_385)).is_err());
    let plan = serde_json::to_value(build_mail_plan(&request()).unwrap()).unwrap();
    for absent in [
        "hat",
        "role",
        "classification",
        "access_token",
        "client_secret",
    ] {
        assert!(plan.get(absent).is_none());
    }
}

#[test]
fn schema_driven_configuration_and_cli_use_the_same_request() {
    let schema: serde_json::Value =
        serde_json::from_str(zixcel_microsoft::mail::MAIL_REQUEST_SCHEMA).unwrap();
    let value = request();
    assert_eq!(schema["properties"]["schema"]["const"], value.schema);
    let declaration = std::process::Command::new(env!("CARGO_BIN_EXE_zixcel-microsoft"))
        .arg("mail-schema")
        .output()
        .unwrap();
    assert!(declaration.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&declaration.stdout).unwrap(),
        schema
    );
    assert_eq!(
        schema["properties"]["content"]["enum"],
        serde_json::json!(["metadata", "body"])
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_zixcel-microsoft"))
        .args(["mail-plan", "examples/mail-request.json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::to_value(build_mail_plan(&value).unwrap()).unwrap()
    );
}
