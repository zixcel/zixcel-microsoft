use super::{Change, Error, Result, Value, json};
pub(super) fn request(c: &Change, password: Option<&str>) -> Result<(&'static str, String, Value)> {
    Ok(match c {
        Change::AddDomain { domain } => ("POST", "/v1.0/domains".into(), json!({"id":domain})),
        Change::VerifyDomain { domain } => (
            "POST",
            format!("/v1.0/domains/{domain}/verify"),
            json!({"forceTakeover":false}),
        ),
        Change::CreateUser {
            upn,
            display_name,
            mail_nickname,
            usage_location,
            account_enabled,
        } => (
            "POST",
            "/v1.0/users".into(),
            json!({"accountEnabled":account_enabled,"displayName":display_name,"mailNickname":mail_nickname,"userPrincipalName":upn,"usageLocation":usage_location,"passwordProfile":{"forceChangePasswordNextSignIn":true,"password":password.ok_or(Error("initial_password_missing"))?}}),
        ),
        Change::SetUsageLocation {
            user,
            usage_location,
        } => (
            "PATCH",
            format!("/v1.0/users/{user}"),
            json!({"usageLocation":usage_location}),
        ),
        Change::SetUserEnabled { user, enabled } => (
            "PATCH",
            format!("/v1.0/users/{user}"),
            json!({"accountEnabled":enabled}),
        ),
        Change::AssignLicense { user, sku } => (
            "POST",
            format!("/v1.0/users/{user}/assignLicense"),
            json!({"addLicenses":[{"skuId":sku,"disabledPlans":[]}],"removeLicenses":[]}),
        ),
        Change::SendProbe {
            recipient,
            correlation,
        } => (
            "POST",
            "/v1.0/me/sendMail".into(),
            json!({"message":{"subject":format!("zix-mail-probe-{correlation}"),"body":{"contentType":"Text","content":format!("Requested mail delivery probe. Correlation: {correlation}")},"toRecipients":[{"emailAddress":{"address":recipient}}]},"saveToSentItems":true}),
        ),
        _ => return Err(Error("unsupported_graph_change")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verification_cannot_force_takeover_and_licenses_cannot_remove_existing_assignments() {
        let (_, path, body) = request(
            &Change::VerifyDomain {
                domain: "example.net".into(),
            },
            None,
        )
        .unwrap();
        assert_eq!(path, "/v1.0/domains/example.net/verify");
        assert_eq!(body, json!({"forceTakeover":false}));
        let (_, _, body) = request(
            &Change::AssignLicense {
                user: "user".into(),
                sku: "sku".into(),
            },
            None,
        )
        .unwrap();
        assert_eq!(body["removeLicenses"], json!([]));
        assert_eq!(body["addLicenses"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn password_is_required_at_write_and_absent_from_change_artifact() {
        let c = Change::CreateUser {
            upn: "new@example.net".into(),
            display_name: "New".into(),
            mail_nickname: "new".into(),
            usage_location: "JP".into(),
            account_enabled: true,
        };
        assert!(!serde_json::to_string(&c).unwrap().contains("password"));
        assert!(request(&c, None).is_err());
        let (_, _, body) = request(&c, Some("synthetic-password")).unwrap();
        assert_eq!(
            body["passwordProfile"]["forceChangePasswordNextSignIn"],
            true
        );
    }
}
