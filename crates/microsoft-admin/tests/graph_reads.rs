mod support;
use serde_json::{Value, json};
use std::collections::VecDeque;
use support::*;
use zixcel_microsoft_admin::graph::{Read, ReadError, Transport, observe};
struct Fake {
    q: VecDeque<Result<Value, ReadError>>,
    paths: Vec<String>,
}
impl Transport for Fake {
    fn get(&mut self, p: &str) -> Result<Value, ReadError> {
        self.paths.push(p.into());
        self.q.pop_front().unwrap()
    }
}
fn fake(pages: Vec<Result<Value, ReadError>>) -> Fake {
    let mut q = VecDeque::from([Ok(json!({"id":SUBJECT})), Ok(json!({"id":TENANT}))]);
    q.extend(pages);
    Fake { q, paths: vec![] }
}
#[test]
fn tenant_and_subject_must_match_before_directory_discovery() {
    let mut f = fake(vec![]);
    f.q[0] = Ok(json!({"id":TRUSTEE}));
    assert!(observe(&mut f, &scope(), Read::Users).is_err());
    assert_eq!(f.paths.len(), 1);
    let mut f = fake(vec![]);
    f.q[1] = Ok(json!({"id":TRUSTEE}));
    assert!(observe(&mut f, &scope(), Read::Users).is_err());
    assert_eq!(f.paths.len(), 2);
}
#[test]
fn selected_nested_license_fields_are_preserved_other_data_excluded() {
    let mut f = fake(vec![Ok(
        json!({"value":[{"id":"license","skuPartNumber":"SYNTHETIC","servicePlans":[{"servicePlanId":"plan","provisioningStatus":"Success","private":"hidden"}],"prepaidUnits":{"enabled":1},"private":"hidden"}]}),
    )]);
    let r = observe(&mut f, &scope(), Read::Licenses).unwrap();
    assert_eq!(r["observation"]["complete"], true);
    assert_eq!(r["observation"]["data"][0]["prepaidUnits"]["enabled"], 1);
    assert!(!r.to_string().contains("hidden"));
}
#[test]
fn directory_addresses_do_not_prove_exchange_mailboxes() {
    let mut f = fake(vec![Ok(
        json!({"value":[{"id":MAILBOX,"proxyAddresses":["SMTP:support@example.org"],"accountEnabled":false}]}),
    )]);
    let r = observe(&mut f, &scope(), Read::Users).unwrap();
    assert_eq!(r["mailbox_existence"], "not_inferred_from_directory");
    assert_eq!(r["writes_performed"], false);
}
#[test]
fn pagination_follows_only_same_resource_and_has_explicit_completion() {
    let mut f = fake(vec![
        Ok(
            json!({"value":[],"@odata.nextLink":"https://graph.microsoft.com/v1.0/domains?$skiptoken=next"}),
        ),
        Ok(json!({"value":[{"id":"example.org","isVerified":true}]})),
    ]);
    let r = observe(&mut f, &scope(), Read::Domains).unwrap();
    assert_eq!(r["observation"]["pages"], 2);
    assert_eq!(r["observation"]["complete"], true);
}
#[test]
fn arbitrary_origins_paths_and_methods_cannot_be_introduced_by_pagination() {
    for next in [
        "http://graph.microsoft.com/v1.0/domains?$skiptoken=x",
        "https://evil.example/v1.0/domains?$skiptoken=x",
        "https://graph.microsoft.com/v1.0/users?$skiptoken=x",
        "https://graph.microsoft.com/v1.0/domains?$expand=private",
        "https://user@graph.microsoft.com/v1.0/domains?$skiptoken=x",
    ] {
        let mut f = fake(vec![Ok(json!({"value":[],"@odata.nextLink":next}))]);
        assert!(observe(&mut f, &scope(), Read::Domains).is_err());
        assert_eq!(f.paths.len(), 3);
    }
}
#[test]
fn denied_missing_and_budget_exhaustion_never_become_empty_success() {
    for (e, state) in [
        (ReadError::Http(403), "denied"),
        (ReadError::Http(404), "unobserved"),
        (ReadError::Budget, "unobserved"),
    ] {
        let mut f = fake(vec![Err(e)]);
        let r = observe(&mut f, &scope(), Read::Domains).unwrap();
        assert_eq!(r["observation"]["state"], state);
        assert_eq!(r["observation"]["complete"], false);
    }
}
#[test]
fn repeated_cursor_stops_without_repeating_request() {
    let page = json!({"value":[],"@odata.nextLink":"https://graph.microsoft.com/v1.0/domains?$skiptoken=same"});
    let mut f = fake(vec![Ok(page.clone()), Ok(page)]);
    let r = observe(&mut f, &scope(), Read::Domains).unwrap();
    assert_eq!(r["observation"]["reason"], "pagination_cycle");
    assert_eq!(f.paths.len(), 4);
}
#[test]
fn dns_requirements_are_requested_only_for_verified_selected_domain() {
    let mut f = fake(vec![Ok(json!({"id":"example.org","isVerified":false}))]);
    assert!(
        observe(
            &mut f,
            &scope(),
            Read::DomainDns {
                domain: "example.org".into()
            }
        )
        .is_err()
    );
    assert_eq!(f.paths.len(), 3);
}
#[test]
fn oversized_graph_collections_remain_partial() {
    let mut f = fake(vec![Ok(
        json!({"value":vec![json!({"id":"example.org"});101]}),
    )]);
    let r = observe(&mut f, &scope(), Read::Domains).unwrap();
    assert_eq!(r["observation"]["state"], "partial");
    assert_eq!(r["observation"]["complete"], false);
}
