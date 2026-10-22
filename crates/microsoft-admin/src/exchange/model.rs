use crate::Scope;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipient {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub primary: String,
    pub aliases: Vec<String>,
    pub full_access: bool,
    pub send_as: bool,
    pub sign_in_blocked: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub scope: Scope,
    pub observed_at: u64,
    pub accepted_domains: Vec<String>,
    pub recipients: Vec<Recipient>,
    pub complete: bool,
    pub trustee: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    CreateShared {
        address: String,
        display_name: String,
    },
    AddAlias {
        mailbox: String,
        address: String,
    },
    RemoveAlias {
        mailbox: String,
        address: String,
    },
    GrantFullAccess {
        mailbox: String,
        trustee: String,
    },
    RevokeFullAccess {
        mailbox: String,
        trustee: String,
    },
    GrantSendAs {
        mailbox: String,
        trustee: String,
    },
    RevokeSendAs {
        mailbox: String,
        trustee: String,
    },
}
impl Change {
    pub fn mailbox(&self) -> Option<&str> {
        match self {
            Self::CreateShared { .. } => None,
            Self::AddAlias { mailbox, .. }
            | Self::RemoveAlias { mailbox, .. }
            | Self::GrantFullAccess { mailbox, .. }
            | Self::RevokeFullAccess { mailbox, .. }
            | Self::GrantSendAs { mailbox, .. }
            | Self::RevokeSendAs { mailbox, .. } => Some(mailbox),
        }
    }
    pub fn trustee(&self) -> Option<&str> {
        match self {
            Self::GrantFullAccess { trustee, .. }
            | Self::RevokeFullAccess { trustee, .. }
            | Self::GrantSendAs { trustee, .. }
            | Self::RevokeSendAs { trustee, .. } => Some(trustee),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub scope: Scope,
    pub change: Change,
    pub before: Snapshot,
    pub created_at: u64,
    pub expires_at: u64,
}
