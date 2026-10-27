use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Organization {},
    Policies {
        kind: String,
    },
    TransportRules {},
    Connectors {},
    RemoteDomains {},
    Rbac {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        identity: Option<String>,
    },
    Mailbox {
        mailbox: String,
    },
    ClientAccess {
        mailbox: String,
    },
    UserPolicy {
        mailbox: String,
    },
    Folders {
        mailbox: String,
    },
    FolderPermissions {
        mailbox: String,
        folder: String,
    },
    InboxRules {
        mailbox: String,
    },
    MobileDevices {
        mailbox: String,
    },
    AutoMapping {
        mailbox: String,
        trustee: String,
    },
}
impl Read {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Policies { kind }
                if ![
                    "authentication",
                    "mobile",
                    "retention",
                    "sharing",
                    "owa",
                    "address_book",
                    "inbound_spam",
                    "outbound_spam",
                    "anti_phish",
                    "anti_phish_advanced",
                ]
                .contains(&kind.as_str()) =>
            {
                return Err(Error("unknown_policy_kind"));
            }
            Self::Rbac { kind, identity } => match kind.as_deref() {
                None | Some("groups") | Some("roles") if identity.is_none() => (),
                Some("assignments") => {
                    let id = identity
                        .as_deref()
                        .ok_or(Error("rbac_role_identity_required"))?;
                    text(id, 256)?;
                    if id.contains(['*', '?', '[', ']']) {
                        return Err(Error("wildcard_identity_refused"));
                    }
                }
                _ => return Err(Error("invalid_rbac_selector")),
            },
            Self::AutoMapping { mailbox, trustee } => {
                crate::uuid(mailbox)?;
                crate::uuid(trustee)?;
                if mailbox == trustee {
                    return Err(Error("self_delegation"));
                }
            }
            Self::Mailbox { mailbox }
            | Self::ClientAccess { mailbox }
            | Self::UserPolicy { mailbox }
            | Self::Folders { mailbox }
            | Self::InboxRules { mailbox }
            | Self::MobileDevices { mailbox } => crate::uuid(mailbox)?,
            Self::FolderPermissions { mailbox, folder } => {
                crate::uuid(mailbox)?;
                text(folder, 256)?;
                if !folder.starts_with('\\') || folder.contains(':') {
                    return Err(Error("invalid_folder_path"));
                }
            }
            _ => (),
        }
        Ok(())
    }
}
pub(crate) fn text(s: &str, max: usize) -> Result<()> {
    if s.is_empty() || s.len() > max || s.chars().any(char::is_control) {
        Err(Error("invalid_configuration_text"))
    } else {
        Ok(())
    }
}
