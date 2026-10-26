mod support;
use serde_json::{Value, json};
use std::collections::VecDeque;
use support::*;
use zixcel_microsoft_admin::graph::{Read, ReadError, Transport, observe};
struct Api {
    replies: VecDeque<Result<Value, ReadError>>,
    paths: Vec<String>,
}
impl Transport for Api {
    fn get(&mut self, path: &str) -> Result<Value, ReadError> {
        self.paths.push(path.into());
        self.replies
            .pop_front()
            .expect("no retries or unexpected calls")
    }
}
fn api(pages: Vec<Result<Value, ReadError>>) -> Api {
    let mut replies = VecDeque::from([
        Ok(json!({"id": SUBJECT})),
        Ok(json!({"id": TENANT})),
        Ok(json!({"id": "example.org", "isVerified": true})),
    ]);
    replies.extend(pages);
    Api {
        replies,
        paths: vec![],
    }
}
fn request() -> Read {
    Read::DomainDns {
        domain: "example.org".into(),
    }
}
#[test]
fn polymorphic_dns_records_are_bounded_and_projected_without_derived_server_selection() {
    let mut http = api(vec![Ok(json!({"value":[
        {"id":"mx","recordType":"Mx","isOptional":false,"label":"example.org","ttl":3600,"mailExchange":"example-org.mail.protection.outlook.com","preference":0,"private":"HIDDEN"},
        {"id":"txt","recordType":"Txt","text":"v=spf1 include:spf.protection.outlook.com -all"},
        {"id":"cname","recordType":"CName","canonicalName":"example.invalid"}
    ]}))]);
    let response = observe(&mut http, &scope(), request()).unwrap();
    assert_eq!(
        http.paths[3],
        "/v1.0/domains/example.org/serviceConfigurationRecords"
    );
    assert_eq!(response["observation"]["complete"], true);
    let rows = &response["observation"]["data"];
    assert_eq!(
        rows[0]["mailExchange"],
        "example-org.mail.protection.outlook.com"
    );
    assert_eq!(rows[0]["isOptional"], false);
    assert_eq!(
        rows[1]["text"],
        "v=spf1 include:spf.protection.outlook.com -all"
    );
    assert_eq!(rows[2]["canonicalName"], "example.invalid");
    assert!(!response.to_string().contains("HIDDEN"));
}
#[test]
fn actual_read_failures_retain_status_and_cause_without_empty_success_or_retry() {
    for (error, reason, status) in [
        (ReadError::Http(400), "request_rejected", Some(400)),
        (ReadError::Http(401), "authentication_rejected", Some(401)),
        (ReadError::Http(403), "permission_denied", Some(403)),
        (ReadError::Http(404), "not_found_or_hidden", Some(404)),
        (ReadError::Http(429), "rate_limited", Some(429)),
        (ReadError::Http(503), "remote_unavailable", Some(503)),
        (ReadError::Transport, "transport_failed", None),
        (ReadError::Invalid, "response_invalid", None),
        (ReadError::Budget, "request_or_byte_limit", None),
    ] {
        let mut http = api(vec![Err(error)]);
        let response = observe(&mut http, &scope(), request()).unwrap();
        assert_eq!(response["observation"]["complete"], false);
        assert_eq!(response["observation"]["reason"], reason);
        assert_eq!(response["observation"]["http_status"].as_u64(), status);
        assert_eq!(http.paths.len(), 4);
    }
}
#[test]
fn dns_pagination_keeps_record_identity_and_failure_after_partial_data() {
    let mut http = api(vec![
        Ok(
            json!({"value":[{"id":"mx","recordType":"Mx"}],"@odata.nextLink":"https://graph.microsoft.com/v1.0/domains/example.org/serviceConfigurationRecords?$skiptoken=next"}),
        ),
        Err(ReadError::Http(403)),
    ]);
    let response = observe(&mut http, &scope(), request()).unwrap();
    assert_eq!(response["observation"]["state"], "partial");
    assert_eq!(response["observation"]["data"][0]["id"], "mx");
    assert_eq!(response["observation"]["http_status"], 403);
    assert_eq!(http.paths.len(), 5);
}
#[test]
fn ownership_verification_records_are_available_before_domain_is_verified() {
    let mut http = Api {
        replies: VecDeque::from([
            Ok(json!({"id":SUBJECT})),
            Ok(json!({"id":TENANT})),
            Ok(
                json!({"value":[{"id":"ownership","recordType":"Txt","label":"@","text":"MS=synthetic-verification","ttl":3600}]}),
            ),
        ]),
        paths: vec![],
    };
    let r = observe(
        &mut http,
        &scope(),
        Read::DomainVerification {
            domain: "example.org".into(),
        },
    )
    .unwrap();
    assert_eq!(
        http.paths[2],
        "/v1.0/domains/example.org/verificationDnsRecords"
    );
    assert_eq!(r["observation"]["complete"], true);
    assert_eq!(
        r["observation"]["data"][0]["text"],
        "MS=synthetic-verification"
    );
}
