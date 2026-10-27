mod support;
use serde_json::{Value, json};
use std::collections::VecDeque;
use support::*;
use zixcel_microsoft_admin::graph::{Read, ReadError, Transport, observe};
struct Fake {
    q: VecDeque<Result<Value, ReadError>>,
    paths: Vec<String>,
}
#[test]
fn transitive_group_reads_keep_identity_types_and_do_not_infer_effective_access() {
    let mut f = fake(vec![Ok(
        json!({"value":[{"id":MAILBOX,"@odata.type":"#microsoft.graph.user","displayName":"Member","private":"hidden"}]}),
    )]);
    let r = observe(
        &mut f,
        &scope(),
        Read::GroupMembers {
            group: TRUSTEE.into(),
        },
    )
    .unwrap();
    assert!(f.paths[2].contains("/transitiveMembers"));
    assert!(!f.paths[2].contains("$select"));
    assert_eq!(r["observation"]["complete"], true);
    assert_eq!(
        r["observation"]["data"][0]["@odata.type"],
        "#microsoft.graph.user"
    );
    assert_eq!(r["effective_permissions"], "not_inferred");
    assert!(!r.to_string().contains("hidden"));
    let mut f = fake(vec![Err(ReadError::Http(403))]);
    let r = observe(
        &mut f,
        &scope(),
        Read::UserMemberships {
            user: SUBJECT.into(),
        },
    )
    .unwrap();
    assert_eq!(r["observation"]["complete"], false);
    assert_eq!(r["observation"]["state"], "denied");
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
fn domain_operation_state_is_selected_bounded_and_does_not_infer_write_success() {
    let mut f = fake(vec![Ok(json!({"value":[
        {"id":"example.org","isVerified":false,"state":null},
        {"id":"other.example.org","isVerified":false,"state":{
            "operation":"Verification","status":"Failed",
            "lastActionDateTime":"2026-01-01T00:00:00Z","private":"hidden"
        }}
    ]}))]);
    let result = observe(&mut f, &scope(), Read::Domains).unwrap();
    assert!(f.paths[2].contains("supportedServices,state"));
    let rows = &result["observation"]["data"];
    assert!(rows[0].as_object().unwrap().contains_key("state"));
    assert!(rows[0]["state"].is_null());
    assert_eq!(rows[1]["state"]["status"], "Failed");
    assert_eq!(rows[1]["state"]["operation"], "Verification");
    assert_eq!(rows[1]["isVerified"], false);
    assert!(!result.to_string().contains("hidden"));
    assert_eq!(result["writes_performed"], false);
    let mut malformed = fake(vec![Ok(
        json!({"value":[{"id":"example.org","state":"invalid"}]}),
    )]);
    assert!(observe(&mut malformed, &scope(), Read::Domains).is_err());
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
#[test]
fn continuation_cannot_expand_fields_or_page_size() {
    for q in ["$select=id,passwordProfile", "$top=99999"] {
        let mut f = fake(vec![Ok(
            json!({"value":[],"@odata.nextLink":format!("https://graph.microsoft.com/v1.0/users?{q}")}),
        )]);
        assert!(observe(&mut f, &scope(), Read::Users).is_err());
    }
}
#[test]
fn missing_row_identity_is_not_a_complete_inventory() {
    let mut f = fake(vec![Ok(json!({"value":[{"isVerified":true}]}))]);
    assert!(observe(&mut f, &scope(), Read::Domains).is_err());
}
#[test]
fn user_license_assignment_is_selected_without_inferring_mailbox_provisioning() {
    let mut f = fake(vec![Ok(
        json!({"id":MAILBOX,"assignedLicenses":[{"skuId":"sku","disabledPlans":["disabled"]}],"assignedPlans":[{"servicePlanId":"plan","capabilityStatus":"Enabled","private":"hidden"}]}),
    )]);
    let r = observe(
        &mut f,
        &scope(),
        Read::User {
            user: MAILBOX.into(),
        },
    )
    .unwrap();
    assert_eq!(
        r["observation"]["data"]["assignedLicenses"][0]["skuId"],
        "sku"
    );
    assert!(!r.to_string().contains("hidden"));
}
#[test]
fn large_projected_directory_stops_with_partial_evidence() {
    let users = (0..100)
        .map(|n| json!({"id":format!("33333333-3333-3333-3333-{n:012}"),"displayName":"x".repeat(4000)}))
        .collect::<Vec<_>>();
    let mut f = fake(vec![Ok(json!({"value":users}))]);
    let r = observe(&mut f, &scope(), Read::Users).unwrap();
    assert_eq!(r["observation"]["state"], "partial");
    assert_eq!(r["observation"]["reason"], "projected_output_limit");
    assert!(serde_json::to_vec(&r).unwrap().len() < 100 * 1024);
}
#[test]
fn selected_user_response_must_match_the_requested_identity() {
    let mut f = fake(vec![Ok(json!({"id":TRUSTEE}))]);
    assert!(
        observe(
            &mut f,
            &scope(),
            Read::User {
                user: MAILBOX.into()
            }
        )
        .is_err()
    );
}
#[test]
fn duplicate_resource_ids_never_form_a_complete_inventory() {
    let mut f = fake(vec![Ok(
        json!({"value":[{"id":"example.org"},{"id":"example.org"}]}),
    )]);
    let r = observe(&mut f, &scope(), Read::Domains).unwrap();
    assert_eq!(r["observation"]["complete"], false);
    assert_eq!(r["observation"]["reason"], "duplicate_resource_identity");
}

#[test]
fn conditional_access_preserves_selected_conditions_and_defaults_are_distinct() {
    let mut f = fake(vec![Ok(
        json!({"value":[{"id":"policy","displayName":"Example", "state":"enabledForReportingButNotEnforced","conditions":{"users":{"includeUsers":["All"],"excludeUsers":[SUBJECT]},"applications":{"includeApplications":["All"]}},"grantControls":{"operator":"OR","builtInControls":["mfa"]},"private":"hidden"}]}),
    )]);
    let r = observe(&mut f, &scope(), Read::ConditionalAccess).unwrap();
    assert!(f.paths[2].starts_with("/v1.0/identity/conditionalAccess/policies?"));
    assert_eq!(
        r["observation"]["data"][0]["conditions"]["users"]["excludeUsers"][0],
        SUBJECT
    );
    assert_eq!(
        r["observation"]["data"][0]["grantControls"]["builtInControls"][0],
        "mfa"
    );
    assert!(!r.to_string().contains("hidden"));
    let mut f = fake(vec![Ok(json!({"id":"default","isEnabled":true}))]);
    let r = observe(&mut f, &scope(), Read::SecurityDefaults).unwrap();
    assert!(f.paths[2].starts_with("/v1.0/policies/identitySecurityDefaultsEnforcementPolicy?"));
    assert_eq!(r["observation"]["data"]["isEnabled"], true);
    let mut f = fake(vec![Err(ReadError::Http(403))]);
    let r = observe(&mut f, &scope(), Read::ConditionalAccess).unwrap();
    assert_eq!(r["observation"]["state"], "denied");
    assert_eq!(r["observation"]["complete"], false);
}
