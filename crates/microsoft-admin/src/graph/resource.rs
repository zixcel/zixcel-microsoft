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
            Self::Domains => (
                "domains".into(),
                vec![
                    "id",
                    "isVerified",
                    "isDefault",
                    "isInitial",
                    "authenticationType",
                    "supportedServices",
                ],
                true,
            ),
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
            Self::DomainDns { domain } => {
                crate::domain(domain)?;
                (
                    format!("domains/{domain}/serviceConfigurationRecords"),
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
        let initial = if matches!(self, Self::DomainDns { .. }) {
            format!("/v1.0/{path}")
        } else {
            format!("/v1.0/{path}?$select={}", fields.join(","))
        };
        Ok((initial, fields, many))
    }
}
