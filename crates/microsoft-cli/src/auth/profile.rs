use serde::{Deserialize, Serialize};
use zix_native_provider::Error;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    pub tenant: String,
    pub client_id: String,
    pub collection: String,
    pub graph_permissions: Vec<String>,
    #[serde(default)]
    pub expected_subject: Option<String>,
}
pub(super) fn load(path: &str) -> Result<Profile, Error> {
    zix_cli::files::parents(std::path::Path::new(path)).map_err(|e| Error(e.0))?;
    let mut p: Profile = crate::input::read(path)?;
    validate(&mut p)?;
    Ok(p)
}
pub(super) fn validate(p: &mut Profile) -> Result<(), Error> {
    zixcel_microsoft_admin::uuid(&p.tenant).map_err(|e| Error(e.0))?;
    zixcel_microsoft_admin::uuid(&p.client_id).map_err(|e| Error(e.0))?;
    if let Some(subject) = &p.expected_subject {
        zixcel_microsoft_admin::uuid(subject).map_err(|e| Error(e.0))?;
    }
    let allowed = [
        "User.Read",
        "Organization.Read.All",
        "Domain.Read.All",
        "LicenseAssignment.Read.All",
        "User.Read.All",
        "GroupMember.ReadBasic.All",
        "GroupMember.Read.All",
        "Group.Read.All",
        "Member.Read.Hidden",
        "Policy.Read.All",
        "Domain.ReadWrite.All",
        "User.ReadWrite.All",
        "User.EnableDisableAccount.All",
        "LicenseAssignment.ReadWrite.All",
        "Mail.Send",
        "Mail.Read",
    ];
    if p.graph_permissions.len() > allowed.len()
        || p.graph_permissions
            .iter()
            .any(|s| !allowed.contains(&s.as_str()))
        || !["User.Read", "Organization.Read.All"]
            .iter()
            .all(|s| p.graph_permissions.iter().any(|v| v == s))
        || p.collection == "session"
        || p.collection.is_empty()
        || p.collection.len() > 64
        || !p
            .collection
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err(Error("invalid_auth_profile"));
    }
    p.graph_permissions.sort();
    p.graph_permissions.dedup();
    Ok(())
}
impl Profile {
    pub fn hash(&self) -> Result<String, Error> {
        zixcel_microsoft_admin::digest(self).map_err(|e| Error(e.0))
    }
    pub fn scopes(&self, resource: &str) -> Result<String, Error> {
        match resource {
            "graph" => Ok(format!(
                "offline_access {}",
                self.graph_permissions
                    .iter()
                    .map(|s| format!("https://graph.microsoft.com/{s}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            "exchange" => Ok("offline_access https://outlook.office365.com/.default".into()),
            _ => Err(Error("invalid_auth_resource")),
        }
    }
    pub fn reference(
        &self,
        resource: &str,
        device: bool,
    ) -> Result<crowsi_credential_broker::SecretRef, Error> {
        let id = format!("{}-{resource}", &self.hash()?[..24]);
        crowsi_credential_broker::SecretRef::parse_canonical(&format!(
            "{}/microsoft/{}/microsoft/{id}",
            self.tenant,
            if device { "device" } else { "oauth" }
        ))
        .map_err(|_| Error("invalid_auth_reference"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_discovery_scopes_are_explicit_and_do_not_admit_arbitrary_permissions() {
        let mut baseline = profile();
        validate(&mut baseline).unwrap();
        let baseline_hash = baseline.hash().unwrap();
        for permission in [
            "GroupMember.ReadBasic.All",
            "GroupMember.Read.All",
            "Group.Read.All",
            "Member.Read.Hidden",
            "Policy.Read.All",
        ] {
            let mut p = profile();
            p.graph_permissions.push(permission.into());
            validate(&mut p).unwrap();
            assert!(
                p.scopes("graph")
                    .unwrap()
                    .contains(&format!("https://graph.microsoft.com/{permission}"))
            );
            assert_ne!(p.hash().unwrap(), baseline_hash);
        }
        let mut p = profile();
        p.graph_permissions
            .push("Policy.ReadWrite.ConditionalAccess".into());
        assert!(validate(&mut p).is_err());
    }
    fn profile() -> Profile {
        Profile {
            tenant: "11111111-1111-4111-8111-111111111111".into(),
            client_id: "22222222-2222-4222-8222-222222222222".into(),
            collection: "default".into(),
            graph_permissions: vec!["User.Read".into(), "Organization.Read.All".into()],
            expected_subject: None,
        }
    }
    #[test]
    fn owner_registration_explicit_scopes_and_separate_resource_references() {
        let mut p = profile();
        validate(&mut p).unwrap();
        assert_ne!(
            p.reference("graph", false).unwrap(),
            p.reference("exchange", false).unwrap()
        );
        assert!(p.scopes("graph").unwrap().contains("offline_access"));
        assert!(p.scopes("arbitrary").is_err());
        p.collection = "session".into();
        assert!(validate(&mut p).is_err());
        let mut p = profile();
        p.graph_permissions.push("Directory.ReadWrite.All".into());
        assert!(validate(&mut p).is_err());
        let mut p = profile();
        p.client_id = "common".into();
        assert!(validate(&mut p).is_err());
    }
}
