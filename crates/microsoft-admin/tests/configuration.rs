mod support;
use serde_json::{Value, json};
use support::*;
use zixcel_microsoft_admin::{
    Error, Result, Scope,
    configuration::{self, Backend, Change, Read},
    digest,
};
struct Fake {
    state: Value,
    after: Value,
    writes: usize,
    fail: bool,
}
impl Backend for Fake {
    fn inspect(&mut self, _: &Scope, _: &Read, _: Option<&Change>) -> Result<Value> {
        Ok(self.state.clone())
    }
    fn mutate(&mut self, _: &Scope, _: &Change) -> Result<()> {
        self.writes += 1;
        if self.fail {
            return Err(Error("synthetic_secret"));
        }
        self.state = self.after.clone();
        Ok(())
    }
}
fn backend(before: Value, after: Value) -> Fake {
    Fake {
        state: before,
        after,
        writes: 0,
        fail: false,
    }
}
fn settings() -> Change {
    serde_json::from_value(json!({"operation":"configure","target":{"resource":"mailbox","mailbox":MAILBOX},"identity":null,"values":{"MessageCopyForSentAsEnabled":true}})).unwrap()
}
#[test]
fn settings_preserve_other_fields_and_require_complete_fresh_exact_observation() {
    let before = json!({"complete":true,"status":"observed","data":{"MessageCopyForSentAsEnabled":false,"HiddenFromAddressListsEnabled":false}});
    for mode in 0..5 {
        let mut b = backend(before.clone(), before.clone());
        b.after["data"]["MessageCopyForSentAsEnabled"] = json!(true);
        let p = configuration::plan(&mut b, scope(), settings(), || 100).unwrap();
        let (_d, j) = journal();
        match mode {
            0 => b.state["complete"] = json!(false),
            1 => b.after["data"]["HiddenFromAddressListsEnabled"] = json!(true),
            2 => b.fail = true,
            _ => (),
        }
        let at = if mode == 3 { 700 } else { 101 };
        let approval = if mode == 4 {
            "0".repeat(64)
        } else {
            digest(&p).unwrap()
        };
        let r = configuration::execute(&mut b, &j, &p, &approval, || at);
        if [0, 3, 4].contains(&mode) {
            assert!(r.is_err());
            assert_eq!(b.writes, 0);
        } else {
            assert_eq!(r.unwrap()["state"], "outcome_unknown");
            assert_eq!(b.writes, 1);
        }
    }
    let mut b = backend(
        before.clone(),
        json!({"complete":true,"status":"observed","data":{"MessageCopyForSentAsEnabled":true,"HiddenFromAddressListsEnabled":false}}),
    );
    let p = configuration::plan(&mut b, scope(), settings(), || 100).unwrap();
    let (_d, j) = journal();
    let hash = digest(&p).unwrap();
    assert_eq!(
        configuration::execute(&mut b, &j, &p, &hash, || 101).unwrap()["state"],
        "verified"
    );
    b.state = before;
    assert!(configuration::execute(&mut b, &j, &p, &hash, || 101).is_err());
    assert_eq!(b.writes, 1);
}
#[test]
fn configuration_contract_rejects_open_properties_wildcards_invalid_values_and_selectors() {
    for values in [
        json!({"Command":"Remove-Mailbox"}),
        json!({"MessageCopyForSentAsEnabled":"true"}),
        json!({}),
    ] {
        let mut c = settings();
        if let Change::Configure {
            values: ref mut v, ..
        } = c
        {
            *v = serde_json::from_value(values).unwrap();
        }
        assert!(c.validate().is_err());
    }
    for selector in [
        json!({"resource":"mailbox","mailbox":"bad"}),
        json!({"resource":"organization","mailbox":MAILBOX}),
        json!({"resource":"policies","kind":"execute"}),
        json!({"resource":"folder_permissions","mailbox":MAILBOX,"folder":"other@example.org:\\Inbox"}),
    ] {
        assert!(match serde_json::from_value::<Read>(selector) {
            Ok(r) => r.validate().is_err(),
            Err(_) => true,
        });
    }
    let c:Change=serde_json::from_value(json!({"operation":"configure","target":{"resource":"policies","kind":"outbound_spam"},"identity":"*","values":{"AutoForwardingMode":"On"}})).unwrap();
    assert!(c.validate().is_err());
}
fn spec() -> Value {
    json!({"name":"zix:school","priority":0,"from":["school@example.org"],"subject_contains":[],"body_contains":[],"subject_or_body_contains":["child"],"delivery":"redirect","recipients":["child@example.org"],"keep_original":false,"stop_processing":true,"allow_external":false})
}
fn create() -> Change {
    serde_json::from_value(json!({"operation":"create_inbox_rule","mailbox":MAILBOX,"rule":spec(),"acknowledge_outlook_rules_blob_removal":true})).unwrap()
}
fn view(rows: Value) -> Value {
    json!({"complete":true,"status":"observed","data":{"mailbox":MAILBOX,"primary":"support@example.org","rules":rows},"routing_guard":{"complete":true,"known_cycle":false,"external_destinations":[],"nodes":[]}})
}
fn observed_rule(id: &str) -> Value {
    let mut r = spec();
    r.as_object_mut().unwrap().remove("allow_external");
    r["identity"] = json!(id);
    r["enabled"] = json!(true);
    r["manageable"] = json!(true);
    r["unsupported_properties"] = json!([]);
    r
}
#[test]
fn rule_creation_verifies_content_priority_and_preserves_unrelated_rules() {
    let unrelated = json!({"identity":"existing","name":"Other","priority":0,"enabled":true,"manageable":false});
    for mode in 0..4 {
        let before = view(json!([unrelated]));
        let mut after = view(json!([observed_rule("new"), unrelated]));
        after["data"]["rules"][1]["priority"] = json!(1);
        if mode == 1 {
            after["data"]["rules"][1]["name"] = json!("lost content");
        }
        if mode == 2 {
            after["data"]["rules"][0]["keep_original"] = json!(true);
        }
        if mode == 3 {
            after["data"]["rules"][0]["priority"] = json!(2);
        }
        let mut b = backend(before, after);
        let p = configuration::plan(&mut b, scope(), create(), || 100).unwrap();
        let (_d, j) = journal();
        assert_eq!(
            configuration::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap()["state"],
            if mode == 0 {
                "verified"
            } else {
                "outcome_unknown"
            }
        );
        assert_eq!(b.writes, 1);
    }
}
#[test]
fn routing_guards_refuse_cycles_unknown_graph_external_destinations_and_missing_ack() {
    for mode in 0..4 {
        let before = view(json!([]));
        let mut b = backend(before.clone(), before);
        let mut c = create();
        match mode {
            0 => b.state["routing_guard"]["known_cycle"] = json!(true),
            1 => b.state["routing_guard"]["complete"] = json!(false),
            2 => b.state["routing_guard"]["external_destinations"] = json!(["outside@example.com"]),
            _ => {
                if let Change::CreateInboxRule {
                    acknowledge_outlook_rules_blob_removal: ref mut a,
                    ..
                } = c
                {
                    *a = false
                }
            }
        }
        assert!(configuration::plan(&mut b, scope(), c, || 100).is_err());
        assert_eq!(b.writes, 0);
    }
    let mut c = create();
    if let Change::CreateInboxRule {
        rule: ref mut r, ..
    } = c
    {
        r.from.clear();
        r.subject_or_body_contains.clear();
    }
    assert!(c.validate().is_err());
}
#[test]
fn managed_update_delete_and_enable_have_exact_target_identity_and_no_unrelated_drift() {
    for operation in [
        "update_inbox_rule",
        "delete_inbox_rule",
        "set_inbox_rule_enabled",
    ] {
        let before = view(json!([observed_rule("owned")]));
        let mut change = json!({"operation":operation,"mailbox":MAILBOX,"identity":"owned","acknowledge_outlook_rules_blob_removal":true});
        let mut after = before.clone();
        match operation {
            "update_inbox_rule" => {
                let mut r = spec();
                r["keep_original"] = json!(true);
                change["rule"] = r;
                after["data"]["rules"][0]["keep_original"] = json!(true);
            }
            "delete_inbox_rule" => after["data"]["rules"] = json!([]),
            _ => {
                change["enabled"] = json!(false);
                after["data"]["rules"][0]["enabled"] = json!(false);
            }
        }
        let c: Change = serde_json::from_value(change).unwrap();
        let mut b = backend(before, after);
        let p = configuration::plan(&mut b, scope(), c, || 100).unwrap();
        let (_d, j) = journal();
        assert_eq!(
            configuration::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap()["state"],
            "verified"
        );
    }
    let mut b = backend(view(json!([observed_rule("owned")])), Value::Null);
    b.state["data"]["rules"][0]["manageable"] = json!(false);
    let c:Change=serde_json::from_value(json!({"operation":"delete_inbox_rule","mailbox":MAILBOX,"identity":"owned","acknowledge_outlook_rules_blob_removal":true})).unwrap();
    assert!(configuration::plan(&mut b, scope(), c, || 100).is_err());
}
#[test]
fn auto_mapping_never_claims_client_discovery_was_observed() {
    let v = json!({"complete":true,"status":"observed","data":{"trustee_smtp":"delegate@example.org","permissions":[{"rights":["FullAccess"],"deny":false,"inherited":false}],"auto_mapping":"not_observable_by_permission_listing"}});
    let mut b = backend(v.clone(), v);
    let c = Change::SetAutoMapping {
        mailbox: MAILBOX.into(),
        trustee: TRUSTEE.into(),
        enabled: true,
    };
    let p = configuration::plan(&mut b, scope(), c, || 100).unwrap();
    let (_d, j) = journal();
    let r = configuration::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap();
    assert_eq!(r["state"], "permission_verified_client_check_required");
    assert_eq!(r["auto_mapping_observed"], false);
}

#[test]
fn configuration_resolution_never_repeats_writes_or_rewrites_unknown_history() {
    use zixcel_microsoft_admin::configuration::reconciliation;
    for applied in [false, true] {
        let before = json!({"complete":true,"status":"observed","data":{"MessageCopyForSentAsEnabled":false}});
        let mut after = before.clone();
        after["data"]["MessageCopyForSentAsEnabled"] = json!(true);
        let mut b = backend(before.clone(), after.clone());
        let p = configuration::plan(&mut b, scope(), settings(), || 100).unwrap();
        let (d, j) = journal();
        let hash = digest(&p).unwrap();
        b.fail = true;
        assert_eq!(
            configuration::execute(&mut b, &j, &p, &hash, || 101).unwrap()["state"],
            "outcome_unknown"
        );
        let original_marker = std::fs::read(d.path().join(format!("{hash}.attempt"))).unwrap();
        b.state = if applied { after } else { before };
        assert!(reconciliation::plan(&mut b, &j, &scope(), p.clone(), || 699).is_err());
        let resolution = reconciliation::plan(&mut b, &j, &scope(), p.clone(), || 1000).unwrap();
        let r = reconciliation::execute(&mut b, &j, &scope(), &resolution, || 1001).unwrap();
        assert_eq!(r["remote_write_attempted"], false);
        assert_eq!(r["original_outcome"], "unknown");
        assert_eq!(r["original_replay_allowed"], false);
        assert_eq!(
            r["observed_state"],
            if applied {
                "requested_state_observed"
            } else {
                "original_state_observed"
            }
        );
        assert_eq!(b.writes, 1);
        assert_eq!(
            std::fs::read(d.path().join(format!("{hash}.attempt"))).unwrap(),
            original_marker
        );
        assert!(!d.path().join(format!("{TENANT}.active")).exists());
        b.state = p.before.clone();
        assert!(configuration::execute(&mut b, &j, &p, &hash, || 101).is_err());
        assert_eq!(b.writes, 1);
    }
}
#[test]
fn resolution_refuses_expiry_scope_and_postreview_drift() {
    use zixcel_microsoft_admin::configuration::reconciliation;
    for mode in 0..3 {
        let v = json!({"complete":true,"status":"observed","data":{"MessageCopyForSentAsEnabled":false}});
        let mut b = backend(v.clone(), v);
        let p = configuration::plan(&mut b, scope(), settings(), || 100).unwrap();
        let (d, j) = journal();
        b.fail = true;
        configuration::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap();
        let mut r = reconciliation::plan(&mut b, &j, &scope(), p, || 1000).unwrap();
        match mode {
            0 => b.state["data"]["MessageCopyForSentAsEnabled"] = json!(true),
            1 => r.original.scope.subject = TRUSTEE.into(),
            _ => (),
        }
        assert!(
            reconciliation::execute(&mut b, &j, &scope(), &r, || if mode == 2 {
                1300
            } else {
                1001
            })
            .is_err()
        );
        assert!(d.path().join(format!("{TENANT}.active")).exists());
        assert_eq!(b.writes, 1);
    }
}

#[test]
fn mapping_resolution_distinguishes_missing_grant_from_unobservable_mapping() {
    use zixcel_microsoft_admin::configuration::reconciliation;
    for initially_present in [false, true] {
        let grant = json!({"user":"delegate@example.org","rights":["FullAccess"],"deny":false,"inherited":false});
        let before = json!({"complete":true,"status":"observed","data":{"trustee_smtp":"delegate@example.org","permissions":if initially_present {json!([grant])} else {json!([])},"auto_mapping":"not_observable_by_permission_listing"}});
        let mut b = backend(before.clone(), before);
        let change = Change::SetAutoMapping {
            mailbox: MAILBOX.into(),
            trustee: TRUSTEE.into(),
            enabled: true,
        };
        let p = configuration::plan(&mut b, scope(), change, || 100).unwrap();
        let (_d, j) = journal();
        b.fail = true;
        configuration::execute(&mut b, &j, &p, &digest(&p).unwrap(), || 101).unwrap();
        b.state["data"]["permissions"] = json!([]);
        let r = reconciliation::plan(&mut b, &j, &scope(), p, || 1000).unwrap();
        assert_eq!(
            r.observed_state,
            "full_access_absent_requires_separate_repair_plan"
        );
        let receipt = reconciliation::execute(&mut b, &j, &scope(), &r, || 1001).unwrap();
        assert_eq!(receipt["remote_write_attempted"], false);
        assert_eq!(receipt["auto_mapping_observed"], false);
        assert_eq!(b.writes, 1);
    }
}
