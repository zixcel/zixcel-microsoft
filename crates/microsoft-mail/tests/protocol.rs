use serde_json::{Value, json};
use zixcel_microsoft_mail::{Application, MAX_TOKEN_RESPONSE, PlainMail, mailbox, parse_token};
const TENANT: &str = "aaaaaaaa-1111-2222-3333-bbbbbbbbbbbb";
const CLIENT: &str = "cccccccc-1111-2222-3333-dddddddddddd";
const SENDER: &str = "eeeeeeee-1111-2222-3333-ffffffffffff";
fn app() -> Application {
    Application::new(TENANT, CLIENT, SENDER).unwrap()
}
fn mail() -> PlainMail<'static> {
    PlainMail {
        to: "contact@example.org",
        subject: "Website contact",
        text: "Untrusted <html>\n\"text\"",
        reply_to: None,
    }
}
#[test]
fn app_credentials_are_encoded_and_confined_to_token_endpoint() {
    let r = app().client_secret_request("private+&= secret").unwrap();
    assert_eq!(
        r.url,
        format!("https://login.microsoftonline.com/{TENANT}/oauth2/v2.0/token")
    );
    assert_eq!(r.content_type, "application/x-www-form-urlencoded");
    let form: std::collections::BTreeMap<_, _> = url::form_urlencoded::parse(r.body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(form.len(), 4);
    assert_eq!(form["client_secret"], "private+&= secret");
    assert_eq!(form["scope"], "https://graph.microsoft.com/.default");
    assert_eq!(form["grant_type"], "client_credentials");
    assert_eq!(form["client_id"], CLIENT);
    for bad in ["", "\r\n", &"x".repeat(4097)] {
        assert!(app().client_secret_request(bad).is_err());
    }
}
#[test]
fn identity_rejects_path_or_tenant_injection() {
    for bad in [
        "common",
        "../me",
        "00000000-0000-0000-0000-000000000000",
        "aaaaaaaa-1111-2222-3333-bbbbbbbbbbb/",
    ] {
        assert!(Application::new(bad, CLIENT, SENDER).is_err());
        assert!(Application::new(TENANT, bad, SENDER).is_err());
        assert!(Application::new(TENANT, CLIENT, bad).is_err());
    }
    assert_eq!(
        Application::new(&TENANT.to_uppercase(), CLIENT, SENDER)
            .unwrap()
            .identity(),
        app().identity()
    );
}
#[test]
fn plaintext_delivery_has_fixed_sender_and_no_optional_reply() {
    let r = app().send_request(mail()).unwrap();
    assert_eq!(
        r.url,
        format!("https://graph.microsoft.com/v1.0/users/{SENDER}/sendMail")
    );
    let v: Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(v["saveToSentItems"], false);
    assert_eq!(
        v["message"]["body"],
        json!({"contentType":"Text","content":"Untrusted <html>\n\"text\""})
    );
    assert_eq!(
        v["message"]["toRecipients"][0]["emailAddress"]["address"],
        "contact@example.org"
    );
    assert_eq!(v["message"].as_object().unwrap().len(), 3);
    let mut m = mail();
    m.reply_to = Some("visitor@example.net");
    let v: Value = serde_json::from_str(&app().send_request(m).unwrap().body).unwrap();
    assert_eq!(
        v["message"]["replyTo"][0]["emailAddress"]["address"],
        "visitor@example.net"
    );
}
#[test]
fn mail_rejects_header_injection_and_invalid_addresses() {
    for bad in [
        "a@example.org\r\nBcc:x@example.org",
        "a,b@example.org",
        "a@localhost",
        "a@-example.org",
        "a..b@example.org",
    ] {
        assert!(!mailbox(bad));
        let mut m = mail();
        m.to = bad;
        assert!(app().send_request(m).is_err());
        let mut m = mail();
        m.reply_to = Some(bad);
        assert!(app().send_request(m).is_err());
    }
    for bad in ["", "  ", "ok\r\nBcc: a", &"s".repeat(999)] {
        let mut m = mail();
        m.subject = bad;
        assert!(app().send_request(m).is_err());
    }
    for bad in ["", "\0", &"x".repeat(32769)] {
        let mut m = mail();
        m.text = bad;
        assert!(app().send_request(m).is_err());
    }
}
#[test]
fn token_response_is_bounded_and_does_not_reflect_errors() {
    let good = br#"{"access_token":"a.b-c_d","token_type":"Bearer","expires_in":3599}"#;
    let t = parse_token(200, good).unwrap();
    assert_eq!(t.secret(), "a.b-c_d");
    assert_eq!(t.lifetime_seconds(), 3599);
    for status in [201, 302, 400, 401, 429, 500] {
        assert!(parse_token(status, good).is_err());
    }
    for bad in [
        json!({"access_token":"bad\r\nvalue","token_type":"Bearer","expires_in":3600}),
        json!({"access_token":"ok","token_type":"Basic","expires_in":3600}),
        json!({"access_token":"ok","token_type":"Bearer","expires_in":59}),
        json!({"access_token":"ok","token_type":"Bearer","expires_in":86401}),
    ] {
        assert!(parse_token(200, bad.to_string().as_bytes()).is_err());
    }
    assert!(
        parse_token(
            200,
            br#"{"access_token":"a","access_token":"b","token_type":"Bearer","expires_in":3600}"#
        )
        .is_err()
    );
    assert!(parse_token(200, &vec![b'x'; MAX_TOKEN_RESPONSE + 1]).is_err());
    assert_eq!(
        parse_token(401, b"provider secret or personal data").err(),
        Some("graph_token_unavailable")
    );
}
