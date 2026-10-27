use super::Read;
use crate::Result;
impl Read {
    pub(super) fn resource(&self, tenant: &str) -> Result<(String, Vec<&'static str>, bool)> {
        let (path, fields, many) = match self {
            Self::Organization => (
                format!("organization/{tenant}"),
                vec!["id", "displayName", "verifiedDomains"],
                false,
            ),
            Self::Licenses => (
                "subscribedSkus".into(),
                vec![
                    "id",
                    "skuId",
                    "skuPartNumber",
                    "capabilityStatus",
                    "consumedUnits",
                    "prepaidUnits",
                    "servicePlans",
                ],
                true,
            ),
            Self::SecurityDefaults => (
                "policies/identitySecurityDefaultsEnforcementPolicy".into(),
                vec!["id", "displayName", "isEnabled"],
                false,
            ),
            Self::ConditionalAccess => (
                "identity/conditionalAccess/policies".into(),
                vec![
                    "id",
                    "displayName",
                    "state",
                    "conditions",
                    "grantControls",
                    "sessionControls",
                    "createdDateTime",
                    "modifiedDateTime",
                ],
                true,
            ),
            Self::Domains => (
                "domains".into(),
                vec![
                    "id",
                    "isVerified",
                    "isDefault",
                    "isInitial",
                    "authenticationType",
                    "supportedServices",
                    "state",
                ],
                true,
            ),
            Self::Groups | Self::GroupMembers { .. } | Self::UserMemberships { .. } => {
                let path = match self {
                    Self::Groups => "groups".into(),
                    Self::GroupMembers { group } => {
                        crate::uuid(group)?;
                        format!("groups/{group}/transitiveMembers")
                    }
                    Self::UserMemberships { user } => {
                        crate::uuid(user)?;
                        format!("users/{user}/transitiveMemberOf/microsoft.graph.group")
                    }
                    _ => unreachable!(),
                };
                // Polymorphic member collections are projected locally; selecting
                // group-only properties on the directoryObject base is invalid.
                let fields = if matches!(self, Self::GroupMembers { .. }) {
                    vec!["id", "@odata.type", "displayName"]
                } else {
                    vec![
                        "id",
                        "displayName",
                        "mail",
                        "mailEnabled",
                        "securityEnabled",
                        "groupTypes",
                        "membershipRule",
                    ]
                };
                (path, fields, true)
            }
            Self::Users | Self::User { .. } => {
                let (path, many) = match self {
                    Self::User { user } => {
                        crate::uuid(user)?;
                        (format!("users/{user}"), false)
                    }
                    _ => ("users".into(), true),
                };
                (
                    path,
                    vec![
                        "id",
                        "displayName",
                        "userPrincipalName",
                        "mail",
                        "proxyAddresses",
                        "accountEnabled",
                        "assignedLicenses",
                        "assignedPlans",
                    ],
                    many,
                )
            }
            Self::DomainVerification { domain } | Self::DomainDns { domain } => {
                crate::domain(domain)?;
                (
                    format!(
                        "domains/{domain}/{}",
                        if matches!(self, Self::DomainVerification { .. }) {
                            "verificationDnsRecords"
                        } else {
                            "serviceConfigurationRecords"
                        }
                    ),
                    vec![
                        "id",
                        "recordType",
                        "isOptional",
                        "label",
                        "supportedService",
                        "ttl",
                        "mailExchange",
                        "preference",
                        "text",
                        "canonicalName",
                    ],
                    true,
                )
            }
        };
        // DNS record collections contain derived MX/TXT/CNAME properties. Fetch the
        // bounded collection, then project locally instead of selecting derived fields
        // on the base domainDnsRecord entity at the server.
        let initial = if matches!(
            self,
            Self::DomainDns { .. } | Self::DomainVerification { .. } | Self::GroupMembers { .. }
        ) {
            format!("/v1.0/{path}")
        } else {
            format!("/v1.0/{path}?$select={}", fields.join(","))
        };
        Ok((initial, fields, many))
    }
}
