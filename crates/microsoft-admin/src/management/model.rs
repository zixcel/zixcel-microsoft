use crate::{Error, Result, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    AddDomain {
        domain: String,
    },
    VerifyDomain {
        domain: String,
    },
    CreateUser {
        upn: String,
        display_name: String,
        mail_nickname: String,
        usage_location: String,
        account_enabled: bool,
    },
    SetUsageLocation {
        user: String,
        usage_location: String,
    },
    SetUserEnabled {
        user: String,
        enabled: bool,
    },
    SetUserPrincipalName {
        user: String,
        upn: String,
    },
    AssignLicense {
        user: String,
        sku: String,
    },
    PrepareDkim {
        domain: String,
    },
    EnableDkim {
        domain: String,
    },
    SendProbe {
        recipient: String,
        correlation: String,
    },
}
impl Change {
    pub fn group(&self) -> &'static str {
        match self {
            Self::AddDomain { .. } | Self::VerifyDomain { .. } => "domains",
            Self::CreateUser { .. }
            | Self::SetUsageLocation { .. }
            | Self::SetUserEnabled { .. }
            | Self::SetUserPrincipalName { .. } => "users",
            Self::AssignLicense { .. } => "licenses",
            Self::PrepareDkim { .. } | Self::EnableDkim { .. } => "dkim",
            Self::SendProbe { .. } => "mail",
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::AddDomain { domain }
            | Self::VerifyDomain { domain }
            | Self::PrepareDkim { domain }
            | Self::EnableDkim { domain } => crate::domain(domain),
            Self::CreateUser {
                upn,
                display_name,
                mail_nickname,
                usage_location,
                ..
            } => {
                crate::email(upn)?;
                location(usage_location)?;
                if display_name.is_empty()
                    || display_name.len() > 120
                    || display_name.chars().any(char::is_control)
                    || mail_nickname.is_empty()
                    || mail_nickname.len() > 64
                    || !mail_nickname
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                {
                    return Err(Error("invalid_user_properties"));
                }
                Ok(())
            }
            Self::SetUsageLocation {
                user,
                usage_location,
            } => {
                crate::uuid(user)?;
                location(usage_location)
            }
            Self::SetUserEnabled { user, .. } => crate::uuid(user),
            Self::SetUserPrincipalName { user, upn } => {
                crate::uuid(user)?;
                crate::email(upn)
            }
            Self::AssignLicense { user, sku } => {
                crate::uuid(user)?;
                crate::uuid(sku)
            }
            Self::SendProbe {
                recipient,
                correlation,
            } => {
                crate::email(recipient)?;
                crate::uuid(correlation)
            }
        }
    }
}
fn location(s: &str) -> Result<()> {
    if s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(Error("invalid_usage_location"))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub scope: Scope,
    pub change: Change,
    pub before: Value,
    pub created_at: u64,
    pub expires_at: u64,
}
