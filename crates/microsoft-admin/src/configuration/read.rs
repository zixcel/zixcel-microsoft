use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Organization {},
    Policies { kind: String },
    TransportRules {},
    Connectors {},
    RemoteDomains {},
    Rbac {},
    Mailbox { mailbox: String },
    ClientAccess { mailbox: String },
    UserPolicy { mailbox: String },
    Folders { mailbox: String },
    FolderPermissions { mailbox: String, folder: String },
    InboxRules { mailbox: String },
    MobileDevices { mailbox: String },
    AutoMapping { mailbox: String, trustee: String },
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
                ]
                .contains(&kind.as_str()) =>
            {
                return Err(Error("unknown_policy_kind"));
            }
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
